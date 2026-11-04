// ============================================================
// chat_api.rs — Lawu-2.5-Fine Chat Persistence API
// ------------------------------------------------------------
// Port :3005  Axum 0.8 + rusqlite + reqwest
// Shares /var/lib/lawu/auth.db dengan auth_api (chat_sessions, chat_messages).
//
// Endpoint:
//   GET    /health
//   GET    /api/conversations?limit=50        (auth)
//   GET    /api/conversations/:id             (auth)
//   POST   /api/conversations  body {title?}  (auth)
//   PATCH  /api/conversations/:id  body {title} (auth)
//   DELETE /api/conversations/:id              (auth, cascade messages)
//   POST   /api/chat  body {conversationId, message}  (auth, proxy ke :3001)
//
// Build: cargo build --release -> cp chat_api lawu-chat
// ============================================================
#[path = "../auth_middleware.rs"]
mod auth_middleware;

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use auth_middleware::{verify_token_with_db, AuthUser, DbPool};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{collections::HashMap, sync::Arc, time::Instant};
use tokio::sync::Mutex;
use uuid::Uuid;

const MAIN_RS_CHAT_URL: &str = "http://127.0.0.1:3001/api/chat";

#[derive(Clone)]
struct AppState {
    db: DbPool,
    http: reqwest::Client,
}

// ------------------------------ DTO ------------------------------
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SessionDto {
    id: String,
    title: String,
    created_at: String,
    updated_at: String,
    message_count: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MessageDto {
    id: String,
    role: String,
    content: String,
    latency_ms: Option<i64>,
    created_at: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateSessionReq {
    title: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateSessionReq {
    title: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChatReq {
    conversation_id: String,
    message: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ChatResp {
    user_message: MessageDto,
    assistant_message: MessageDto,
}

// ------------------------------ DB helpers ------------------------------
fn open_db() -> Connection {
    { let p = auth_middleware::auth_db_path(); let _ = std::fs::create_dir_all(std::path::Path::new(&p).parent().unwrap_or(std::path::Path::new("/var/lib/lawu"))); }
    let conn = Connection::open(auth_middleware::auth_db_path()).expect("gagal buka auth.db");
    let _ = conn.pragma_update(None, "journal_mode", "WAL");
    let _ = conn.pragma_update(None, "synchronous", "NORMAL");
    let _ = conn.pragma_update(None, "foreign_keys", "ON");
    // idempotent migration (sama dengan auth_api)
    let sql = include_str!("../../migrations/schema.sql");
    conn.execute_batch(sql).expect("migrate schema");
    conn
}

// ------------------------------ Handlers ------------------------------
async fn health() -> impl IntoResponse {
    Json(json!({
        "status": "ok",
        "service": "lawu-chat",
        "ts": chrono::Utc::now().to_rfc3339(),
    }))
}

// GET /api/conversations?limit=50
async fn list_conversations(
    headers: HeaderMap,
    State(st): State<AppState>,
    Query(q): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let auth = match verify_token_with_db(&headers, st.db.clone()).await {
        Ok(u) => u,
        Err(e) => return e,
    };
    let limit: i64 = q
        .get("limit")
        .and_then(|s| s.parse().ok())
        .unwrap_or(50)
        .clamp(1, 200);
    let pool = st.db.clone();
    let r = tokio::task::spawn_blocking(move || {
        let conn = pool.blocking_lock();
        let mut stmt = conn
            .prepare(
                "SELECT id, title, created_at, updated_at, message_count \
                 FROM chat_sessions WHERE user_id = ?1 ORDER BY updated_at DESC LIMIT ?2",
            )
            .map_err(|e| format!("db: {e}"))?;
        let rows = stmt
            .query_map(rusqlite::params![auth.id, limit], |r| {
                Ok(SessionDto {
                    id: r.get(0)?,
                    title: r.get(1)?,
                    created_at: r.get(2)?,
                    updated_at: r.get(3)?,
                    message_count: r.get(4)?,
                })
            })
            .map_err(|e| format!("query_map: {e}"))?
            .filter_map(|x| x.ok())
            .collect::<Vec<_>>();
        Ok::<_, String>(rows)
    })
    .await;
    match r {
        Ok(Ok(rows)) => (StatusCode::OK, Json(json!({ "sessions": rows }))),
        Ok(Err(e)) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": format!("spawn: {e}") })),
        ),
    }
}

// GET /api/conversations/:id
async fn get_conversation(
    headers: HeaderMap,
    State(st): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let auth = match verify_token_with_db(&headers, st.db.clone()).await {
        Ok(u) => u,
        Err(e) => return e,
    };
    let pool = st.db.clone();
    let r = tokio::task::spawn_blocking(move || -> Result<Value, (StatusCode, Value)> {
        let conn = pool.blocking_lock();
        let session = conn
            .query_row(
                "SELECT id, title, created_at, updated_at, message_count \
                 FROM chat_sessions WHERE id = ?1 AND user_id = ?2",
                rusqlite::params![&id, auth.id],
                |r| {
                    Ok(SessionDto {
                        id: r.get(0)?,
                        title: r.get(1)?,
                        created_at: r.get(2)?,
                        updated_at: r.get(3)?,
                        message_count: r.get(4)?,
                    })
                },
            )
            .map_err(|_| (StatusCode::NOT_FOUND, json!({ "error": "conversation tidak ditemukan" })))?;
        let mut stmt = conn
            .prepare(
                "SELECT id, role, content, latency_ms, created_at \
                 FROM chat_messages WHERE session_id = ?1 ORDER BY created_at ASC",
            )
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, json!({ "error": format!("db: {e}") })))?;
        let msgs = stmt
            .query_map(rusqlite::params![&id], |r| {
                Ok(MessageDto {
                    id: r.get(0)?,
                    role: r.get(1)?,
                    content: r.get(2)?,
                    latency_ms: r.get(3)?,
                    created_at: r.get(4)?,
                })
            })
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, json!({ "error": format!("query_map: {e}") })))?
            .filter_map(|x| x.ok())
            .collect::<Vec<_>>();
        Ok(json!({ "session": session, "messages": msgs }))
    })
    .await;
    match r {
        Ok(Ok(v)) => (StatusCode::OK, Json(v)),
        Ok(Err((s, v))) => (s, Json(v)),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": format!("spawn: {e}") })),
        ),
    }
}

// POST /api/conversations
async fn create_conversation(
    headers: HeaderMap,
    State(st): State<AppState>,
    Json(req): Json<CreateSessionReq>,
) -> impl IntoResponse {
    let auth = match verify_token_with_db(&headers, st.db.clone()).await {
        Ok(u) => u,
        Err(e) => return e,
    };
    let title = req.title.unwrap_or_else(|| "Obrolan Baru".into());
    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let pool = st.db.clone();
    let session = SessionDto {
        id: id.clone(),
        title,
        created_at: now.clone(),
        updated_at: now,
        message_count: 0,
    };
    let r = tokio::task::spawn_blocking(move || -> Result<Value, (StatusCode, Value)> {
        let conn = pool.blocking_lock();
        conn.execute(
            "INSERT INTO chat_sessions (id, user_id, title, created_at, updated_at, message_count) \
             VALUES (?1, ?2, ?3, ?4, ?5, 0)",
            rusqlite::params![&id, auth.id, &session.title, &session.created_at, &session.updated_at],
        )
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({ "error": format!("db insert: {e}") }),
            )
        })?;
        Ok(json!({ "session": session }))
    })
    .await;
    match r {
        Ok(Ok(v)) => (StatusCode::CREATED, Json(v)),
        Ok(Err((s, v))) => (s, Json(v)),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": format!("spawn: {e}") })),
        ),
    }
}

// PATCH /api/conversations/:id
async fn update_conversation(
    headers: HeaderMap,
    State(st): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateSessionReq>,
) -> impl IntoResponse {
    let auth = match verify_token_with_db(&headers, st.db.clone()).await {
        Ok(u) => u,
        Err(e) => return e,
    };
    let title = req.title;
    let now = chrono::Utc::now().to_rfc3339();
    let pool = st.db.clone();
    let r = tokio::task::spawn_blocking(move || -> Result<Value, (StatusCode, Value)> {
        let conn = pool.blocking_lock();
        let n = conn
            .execute(
                "UPDATE chat_sessions SET title = ?1, updated_at = ?2 WHERE id = ?3 AND user_id = ?4",
                rusqlite::params![&title, &now, &id, auth.id],
            )
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, json!({ "error": format!("db: {e}") })))?;
        if n == 0 {
            return Err((StatusCode::NOT_FOUND, json!({ "error": "conversation tidak ditemukan" })));
        }
        let s = conn
            .query_row(
                "SELECT id, title, created_at, updated_at, message_count \
                 FROM chat_sessions WHERE id = ?1",
                rusqlite::params![&id],
                |r| Ok(SessionDto {
                    id: r.get(0)?,
                    title: r.get(1)?,
                    created_at: r.get(2)?,
                    updated_at: r.get(3)?,
                    message_count: r.get(4)?,
                }),
            )
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    json!({ "error": format!("reload: {e}") }),
                )
            })?;
        Ok(json!({ "session": s }))
    })
    .await;
    match r {
        Ok(Ok(v)) => (StatusCode::OK, Json(v)),
        Ok(Err((s, v))) => (s, Json(v)),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": format!("spawn: {e}") })),
        ),
    }
}

// DELETE /api/conversations/:id  (cascade messages via FK)
async fn delete_conversation(
    headers: HeaderMap,
    State(st): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let auth = match verify_token_with_db(&headers, st.db.clone()).await {
        Ok(u) => u,
        Err(e) => return e,
    };
    let pool = st.db.clone();
    let r = tokio::task::spawn_blocking(move || -> Result<Value, (StatusCode, Value)> {
        let conn = pool.blocking_lock();
        match conn.execute(
            "DELETE FROM chat_sessions WHERE id = ?1 AND user_id = ?2",
            rusqlite::params![&id, auth.id],
        ) {
            Ok(n) if n > 0 => Ok(json!({ "ok": true })),
            Ok(_) => Err((StatusCode::NOT_FOUND, json!({ "error": "conversation tidak ditemukan" }))),
            Err(e) => Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({ "error": format!("db: {e}") }),
            )),
        }
    })
    .await;
    match r {
        Ok(Ok(v)) => (StatusCode::OK, Json(v)),
        Ok(Err((s, v))) => (s, Json(v)),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": format!("spawn: {e}") })),
        ),
    }
}

// POST /api/chat  body {conversationId, message}
async fn chat(
    headers: HeaderMap,
    State(st): State<AppState>,
    Json(req): Json<ChatReq>,
) -> impl IntoResponse {
    let auth: AuthUser = match verify_token_with_db(&headers, st.db.clone()).await {
        Ok(u) => u,
        Err(e) => return e,
    };

    // 1. verify conversationId milik user + insert userMessage
    let user_msg_id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let conv_id = req.conversation_id.clone();
    let message_text = req.message.clone();
    let pool_for_insert = st.db.clone();
    let user_msg_id_for_insert = user_msg_id.clone();
    let now_for_insert = now.clone();
    let conv_id_for_insert = conv_id.clone();
    let msg_for_insert = message_text.clone();
    let auth_id = auth.id;
    let r = tokio::task::spawn_blocking(move || -> Result<(), (StatusCode, Value)> {
        let conn = pool_for_insert.blocking_lock();
        let exists: bool = conn
            .query_row(
                "SELECT 1 FROM chat_sessions WHERE id = ?1 AND user_id = ?2",
                rusqlite::params![&conv_id_for_insert, auth_id],
                |_| Ok(true),
            )
            .unwrap_or(false);
        if !exists {
            return Err((StatusCode::NOT_FOUND, json!({ "error": "conversation tidak ditemukan" })));
        }
        conn.execute(
            "INSERT INTO chat_messages (id, session_id, role, content, created_at) \
             VALUES (?1, ?2, 'user', ?3, ?4)",
            rusqlite::params![&user_msg_id_for_insert, &conv_id_for_insert, &msg_for_insert, &now_for_insert],
        )
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({ "error": format!("db insert user msg: {e}") }),
            )
        })?;
        Ok(())
    })
    .await;
    if let Err(e) = r {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": format!("spawn: {e}") })),
        );
    }
    match r.unwrap() {
        Ok(()) => {}
        Err((s, v)) => return (s, Json(v)),
    }

    // 2. forward to main.rs :3001
    let started = Instant::now();
    let llm_resp = st
        .http
        .post(MAIN_RS_CHAT_URL)
        .json(&json!({ "message": message_text }))
        .send()
        .await;
    let latency_ms = started.elapsed().as_millis() as i64;
    let assistant_text = match llm_resp {
        Ok(r) if r.status().is_success() => {
            // main.rs mungkin mengembalikan {reply, content, answer, ...} — coba beberapa field
            match r.json::<Value>().await {
                Ok(v) => v
                    .get("reply")
                    .and_then(|x| x.as_str())
                    .or_else(|| v.get("content").and_then(|x| x.as_str()))
                    .or_else(|| v.get("answer").and_then(|x| x.as_str()))
                    .or_else(|| v.get("response").and_then(|x| x.as_str()))
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| v.to_string()),
                Err(_) => String::from("[error: respons LLM tidak ter-parse]"),
            }
        }
        Ok(r) => format!("[error: main.rs :3001 status {}]", r.status()),
        Err(e) => format!("[error: gagal connect main.rs :3001 — {e}]"),
    };

    // 3. insert assistantMessage + update session counter
    let assistant_msg_id = Uuid::new_v4().to_string();
    let now2 = chrono::Utc::now().to_rfc3339();
    let pool = st.db.clone();
    let conv_id2 = conv_id.clone();
    let assistant_msg_id2 = assistant_msg_id.clone();
    let assistant_text2 = assistant_text.clone();
    let now3 = now2.clone();
    let message_text_for_title = message_text.clone();
    let r = tokio::task::spawn_blocking(move || -> Result<(), (StatusCode, Value)> {
        let conn = pool.blocking_lock();
        conn.execute(
            "INSERT INTO chat_messages (id, session_id, role, content, latency_ms, created_at) \
             VALUES (?1, ?2, 'assistant', ?3, ?4, ?5)",
            rusqlite::params![&assistant_msg_id2, &conv_id2, &assistant_text2, latency_ms, &now3],
        )
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({ "error": format!("db insert assistant msg: {e}") }),
            )
        })?;
        conn.execute(
            "UPDATE chat_sessions SET message_count = message_count + 2, updated_at = ?1 \
             WHERE id = ?2",
            rusqlite::params![&now3, &conv_id2],
        )
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({ "error": format!("db update session: {e}") }),
            )
        })?;
        // Auto-title: kalau title = "Obrolan Baru" dan message_count sekarang 2
        let row: Option<(String, i64)> = conn
            .query_row(
                "SELECT title, message_count FROM chat_sessions WHERE id = ?1",
                rusqlite::params![&conv_id2],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .ok();
        if let Some((title, count)) = row {
            if title == "Obrolan Baru" && count == 2 {
                let new_title: String = message_text_for_title.chars().take(40).collect();
                let _ = conn.execute(
                    "UPDATE chat_sessions SET title = ?1 WHERE id = ?2",
                    rusqlite::params![&new_title, &conv_id2],
                );
            }
        }
        Ok(())
    })
    .await;
    if let Err(e) = r {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": format!("spawn: {e}") })),
        );
    }
    match r.unwrap() {
        Ok(()) => {}
        Err((s, v)) => return (s, Json(v)),
    }

    // 4. return both messages
    let user_msg = MessageDto {
        id: user_msg_id,
        role: "user".into(),
        content: message_text,
        latency_ms: None,
        created_at: now,
    };
    let assistant_msg = MessageDto {
        id: assistant_msg_id,
        role: "assistant".into(),
        content: assistant_text,
        latency_ms: Some(latency_ms),
        created_at: now2,
    };
    (StatusCode::OK, Json(json!(ChatResp {
        user_message: user_msg,
        assistant_message: assistant_msg,
    })))
}

// ------------------------------ main ------------------------------
#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let conn = open_db();
    let pool: DbPool = Arc::new(Mutex::new(conn));
    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .expect("build reqwest client");
    let state = AppState { db: pool, http };

    let app = Router::new()
        .route("/health", get(health))
        .route("/api/conversations", get(list_conversations).post(create_conversation))
        .route("/api/conversations/{id}", get(get_conversation).patch(update_conversation).delete(delete_conversation))
        .route("/api/chat", post(chat))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3005")
        .await
        .expect("bind 127.0.0.1:3005");
    tracing::info!("[lawu-chat] listening on http://127.0.0.1:3005");
    axum::serve(listener, app).await.expect("server error");
}
