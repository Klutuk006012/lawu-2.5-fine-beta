// ============================================================
// auth_api.rs — Lawu-2.5-Fine Auth + User + API Key service
// ------------------------------------------------------------
// Port :3003   Axum 0.8 + rusqlite (bundled)
//
// Endpoint:
//   GET  /health
//   POST /api/auth/login       body {email,password}
//   POST /api/auth/logout      (auth)
//   GET  /api/auth/me          (auth)
//   GET  /api/users            (admin)
//   POST /api/users            (admin)
//   PATCH /api/users/:id       (admin)
//   DELETE /api/users/:id      (admin)
//   GET  /api/api-keys         (auth, milik sendiri)
//   POST /api/api-keys         body {name, scopes?}
//   DELETE /api/api-keys/:id   (auth, milik sendiri)
//
// Build: cargo build --release  ->  cp auth_api lawu-auth
// ============================================================
#[path = "../auth_middleware.rs"]
mod auth_middleware;

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, patch, post},
    Json, Router,
};
use auth_middleware::{
    get_jwt_secret, is_strong_password, is_valid_email, issue_jwt, require_admin,
    verify_token_with_db, AuthUser, DbPool, API_KEY_PREFIX,
};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Clone)]
struct AppState {
    db: DbPool,
}

// ------------------------------ DTO (camelCase) ------------------------------
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LoginReq {
    email: String,
    password: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct UserDto {
    id: i64,
    email: String,
    role: String,
    created_at: String,
    last_login_at: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateUserReq {
    email: String,
    password: String,
    role: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateUserReq {
    email: Option<String>,
    password: Option<String>,
    role: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateApiKeyReq {
    name: String,
    scopes: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ApiKeyDto {
    id: String,
    name: String,
    prefix: String,
    scopes: String,
    created_at: String,
    last_used_at: Option<String>,
    revoked_at: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CreateApiKeyResp {
    id: String,
    name: String,
    key: String, // plaintext, hanya muncul SEKALI
    prefix: String,
    scopes: String,
    created_at: String,
}

// ------------------------------ DB helpers ------------------------------
fn open_db() -> Connection {
    { let p = auth_middleware::auth_db_path(); let _ = std::fs::create_dir_all(std::path::Path::new(&p).parent().unwrap_or(std::path::Path::new("/var/lib/lawu"))); }
    let conn = Connection::open(auth_middleware::auth_db_path()).expect("gagal buka auth.db");
    let _ = conn.pragma_update(None, "journal_mode", "WAL");
    let _ = conn.pragma_update(None, "synchronous", "NORMAL");
    let _ = conn.pragma_update(None, "foreign_keys", "ON");
    run_migrations(&conn);
    seed_default_user(&conn);
    conn
}

fn run_migrations(conn: &Connection) {
    let sql = include_str!("../../migrations/schema.sql");
    conn.execute_batch(sql).expect("migrate schema");
}

fn seed_default_user(conn: &Connection) {
    let exists: bool = conn
        .query_row(
            "SELECT 1 FROM users WHERE email = 'dev@lawu.ai.id' LIMIT 1",
            [],
            |_| Ok(true),
        )
        .unwrap_or(false);
    if exists {
        return;
    }
    let hash = hash_password("12345678").expect("hash default user");
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO users (email, password_hash, role, created_at) VALUES (?1, ?2, 'admin', ?3)",
        rusqlite::params!["dev@lawu.ai.id", hash, now],
    )
    .expect("insert default admin user");
    eprintln!("[lawu-auth] Seeded default admin user dev@lawu.ai.id");
}

// ------------------------------ Argon2 hash ------------------------------
fn hash_password(pw: &str) -> Result<String, String> {
    use argon2::{
        password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
        Argon2,
    };
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    argon2
        .hash_password(pw.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| format!("argon2 hash: {e}"))
}

fn verify_password(pw: &str, hash: &str) -> bool {
    use argon2::{
        password_hash::{PasswordHash, PasswordVerifier},
        Argon2,
    };
    let parsed = match PasswordHash::new(hash) {
        Ok(p) => p,
        Err(_) => return false,
    };
    Argon2::default()
        .verify_password(pw.as_bytes(), &parsed)
        .is_ok()
}

// ------------------------------ Handlers ------------------------------
async fn health() -> impl IntoResponse {
    Json(json!({
        "status": "ok",
        "service": "lawu-auth",
        "ts": chrono::Utc::now().to_rfc3339(),
    }))
}

// POST /api/auth/login
async fn login(State(st): State<AppState>, Json(req): Json<LoginReq>) -> impl IntoResponse {
    let email = req.email.trim().to_lowercase();
    let pool = st.db.clone();
    let res = tokio::task::spawn_blocking(move || -> Result<Value, (StatusCode, Value)> {
        let conn = pool.blocking_lock();
        let row = conn
            .query_row(
                "SELECT id, email, password_hash, role, created_at, last_login_at \
                 FROM users WHERE email = ?1",
                rusqlite::params![&email],
                |r| {
                    Ok((
                        r.get::<_, i64>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                        r.get::<_, String>(3)?,
                        r.get::<_, String>(4)?,
                        r.get::<_, Option<String>>(5)?,
                    ))
                },
            )
            .map_err(|_| {
                (
                    StatusCode::UNAUTHORIZED,
                    json!({ "error": "email atau password salah" }),
                )
            })?;
        let (id, email, hash, role, created_at, last_login_at) = row;
        if !verify_password(&req.password, &hash) {
            return Err((
                StatusCode::UNAUTHORIZED,
                json!({ "error": "email atau password salah" }),
            ));
        }
        let now = chrono::Utc::now().to_rfc3339();
        let _ = conn.execute(
            "UPDATE users SET last_login_at = ?1 WHERE id = ?2",
            rusqlite::params![&now, id],
        );
        let user = AuthUser {
            id,
            email: email.clone(),
            role: role.clone(),
        };
        let token = match issue_jwt(&user) {
            Ok(t) => t,
            Err(e) => {
                return Err((StatusCode::INTERNAL_SERVER_ERROR, json!({ "error": e })))
            }
        };
        Ok(json!({
            "token": token,
            "user": UserDto {
                id, email, role, created_at, last_login_at
            }
        }))
    })
    .await;
    match res {
        Ok(Ok(v)) => (StatusCode::OK, Json(v)),
        Ok(Err((s, v))) => (s, Json(v)),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": format!("spawn: {e}") })),
        ),
    }
}

// POST /api/auth/logout  (stateless JWT — no-op server-side)
async fn logout(headers: HeaderMap, State(st): State<AppState>) -> impl IntoResponse {
    match verify_token_with_db(&headers, st.db.clone()).await {
        Ok(_) => (StatusCode::OK, Json(json!({ "ok": true }))),
        Err(e) => e,
    }
}

// GET /api/auth/me
async fn me(headers: HeaderMap, State(st): State<AppState>) -> impl IntoResponse {
    let u = match verify_token_with_db(&headers, st.db.clone()).await {
        Ok(u) => u,
        Err(e) => return e,
    };
    let pool = st.db.clone();
    let r = tokio::task::spawn_blocking(move || {
        let conn = pool.blocking_lock();
        conn.query_row(
            "SELECT id, email, role, created_at, last_login_at FROM users WHERE id = ?1",
            rusqlite::params![u.id],
            |row| {
                Ok(UserDto {
                    id: row.get(0)?,
                    email: row.get(1)?,
                    role: row.get(2)?,
                    created_at: row.get(3)?,
                    last_login_at: row.get(4)?,
                })
            },
        )
    })
    .await;
    match r {
        Ok(Ok(dto)) => (StatusCode::OK, Json(json!({ "user": dto }))),
        _ => (
            StatusCode::OK,
            Json(json!({
                "user": {
                    "id": u.id, "email": u.email, "role": u.role,
                    "createdAt": null, "lastLoginAt": null
                }
            })),
        ),
    }
}

// GET /api/users  (admin)
async fn list_users(headers: HeaderMap, State(st): State<AppState>) -> impl IntoResponse {
    let auth = match verify_token_with_db(&headers, st.db.clone()).await {
        Ok(u) => u,
        Err(e) => return e,
    };
    if let Err(e) = require_admin(&auth) {
        return e;
    }
    let pool = st.db.clone();
    let r = tokio::task::spawn_blocking(move || {
        let conn = pool.blocking_lock();
        let mut stmt = conn
            .prepare("SELECT id, email, role, created_at, last_login_at FROM users ORDER BY id")
            .map_err(|e| format!("db: {e}"))?;
        let users = stmt
            .query_map([], |r| {
                Ok(UserDto {
                    id: r.get(0)?,
                    email: r.get(1)?,
                    role: r.get(2)?,
                    created_at: r.get(3)?,
                    last_login_at: r.get(4)?,
                })
            })
            .map_err(|e| format!("query_map: {e}"))?
            .filter_map(|x| x.ok())
            .collect::<Vec<_>>();
        Ok::<_, String>(users)
    })
    .await;
    match r {
        Ok(Ok(users)) => (StatusCode::OK, Json(json!({ "users": users }))),
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

// POST /api/users  (admin)
async fn create_user(
    headers: HeaderMap,
    State(st): State<AppState>,
    Json(req): Json<CreateUserReq>,
) -> impl IntoResponse {
    let auth = match verify_token_with_db(&headers, st.db.clone()).await {
        Ok(u) => u,
        Err(e) => return e,
    };
    if let Err(e) = require_admin(&auth) {
        return e;
    }
    let email = req.email.trim().to_lowercase();
    if !is_valid_email(&email) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "email tidak valid" })),
        );
    }
    if !is_strong_password(&req.password) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "password minimal 8 karakter" })),
        );
    }
    let role = req.role.unwrap_or_else(|| "guest".into());
    if role != "admin" && role != "guest" {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "role harus 'admin' atau 'guest'" })),
        );
    }
    let pool = st.db.clone();
    let r = tokio::task::spawn_blocking(move || -> Result<Value, (StatusCode, Value)> {
        let conn = pool.blocking_lock();
        let exists: bool = conn
            .query_row(
                "SELECT 1 FROM users WHERE email = ?1",
                rusqlite::params![&email],
                |_| Ok(true),
            )
            .unwrap_or(false);
        if exists {
            return Err((StatusCode::CONFLICT, json!({ "error": "email sudah dipakai" })));
        }
        let hash = match hash_password(&req.password) {
            Ok(h) => h,
            Err(e) => return Err((StatusCode::INTERNAL_SERVER_ERROR, json!({ "error": e }))),
        };
        let now = chrono::Utc::now().to_rfc3339();
        match conn.execute(
            "INSERT INTO users (email, password_hash, role, created_at) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![&email, &hash, &role, &now],
        ) {
            Ok(_) => {
                let id = conn.last_insert_rowid();
                Ok(json!({
                    "user": UserDto {
                        id, email, role, created_at: now, last_login_at: None
                    }
                }))
            }
            Err(e) => Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({ "error": format!("db insert: {e}") }),
            )),
        }
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

// PATCH /api/users/:id  (admin)
async fn update_user(
    headers: HeaderMap,
    State(st): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateUserReq>,
) -> impl IntoResponse {
    let auth = match verify_token_with_db(&headers, st.db.clone()).await {
        Ok(u) => u,
        Err(e) => return e,
    };
    if let Err(e) = require_admin(&auth) {
        return e;
    }
    let pool = st.db.clone();
    let r = tokio::task::spawn_blocking(move || -> Result<Value, (StatusCode, Value)> {
        let conn = pool.blocking_lock();
        let exists: bool = conn
            .query_row("SELECT 1 FROM users WHERE id = ?1", rusqlite::params![id], |_| Ok(true))
            .unwrap_or(false);
        if !exists {
            return Err((StatusCode::NOT_FOUND, json!({ "error": "user tidak ditemukan" })));
        }
        if let Some(email) = req.email.as_deref() {
            let email = email.trim().to_lowercase();
            if !is_valid_email(&email) {
                return Err((StatusCode::BAD_REQUEST, json!({ "error": "email tidak valid" })));
            }
            let taken: bool = conn
                .query_row(
                    "SELECT 1 FROM users WHERE email = ?1 AND id != ?2",
                    rusqlite::params![&email, id],
                    |_| Ok(true),
                )
                .unwrap_or(false);
            if taken {
                return Err((StatusCode::CONFLICT, json!({ "error": "email sudah dipakai" })));
            }
            let _ = conn.execute(
                "UPDATE users SET email = ?1 WHERE id = ?2",
                rusqlite::params![&email, id],
            );
        }
        if let Some(pw) = req.password.as_deref() {
            if !is_strong_password(pw) {
                return Err((StatusCode::BAD_REQUEST, json!({ "error": "password minimal 8 karakter" })));
            }
            let hash = match hash_password(pw) {
                Ok(h) => h,
                Err(e) => return Err((StatusCode::INTERNAL_SERVER_ERROR, json!({ "error": e }))),
            };
            let _ = conn.execute(
                "UPDATE users SET password_hash = ?1 WHERE id = ?2",
                rusqlite::params![&hash, id],
            );
        }
        if let Some(role) = req.role.as_deref() {
            if role != "admin" && role != "guest" {
                return Err((StatusCode::BAD_REQUEST, json!({ "error": "role harus 'admin' atau 'guest'" })));
            }
            let _ = conn.execute(
                "UPDATE users SET role = ?1 WHERE id = ?2",
                rusqlite::params![role, id],
            );
        }
        let dto = conn
            .query_row(
                "SELECT id, email, role, created_at, last_login_at FROM users WHERE id = ?1",
                rusqlite::params![id],
                |r| Ok(UserDto {
                    id: r.get(0)?,
                    email: r.get(1)?,
                    role: r.get(2)?,
                    created_at: r.get(3)?,
                    last_login_at: r.get(4)?,
                }),
            )
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    json!({ "error": format!("reload: {e}") }),
                )
            })?;
        Ok(json!({ "user": dto }))
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

// DELETE /api/users/:id  (admin)
async fn delete_user(
    headers: HeaderMap,
    State(st): State<AppState>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    let auth = match verify_token_with_db(&headers, st.db.clone()).await {
        Ok(u) => u,
        Err(e) => return e,
    };
    if let Err(e) = require_admin(&auth) {
        return e;
    }
    if auth.id == id {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "tidak bisa hapus diri sendiri" })),
        );
    }
    let pool = st.db.clone();
    let r = tokio::task::spawn_blocking(move || -> Result<Value, (StatusCode, Value)> {
        let conn = pool.blocking_lock();
        match conn.execute("DELETE FROM users WHERE id = ?1", rusqlite::params![id]) {
            Ok(n) if n > 0 => Ok(json!({ "ok": true })),
            Ok(_) => Err((StatusCode::NOT_FOUND, json!({ "error": "user tidak ditemukan" }))),
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

// GET /api/api-keys  (auth, milik sendiri)
async fn list_api_keys(headers: HeaderMap, State(st): State<AppState>) -> impl IntoResponse {
    let auth = match verify_token_with_db(&headers, st.db.clone()).await {
        Ok(u) => u,
        Err(e) => return e,
    };
    let pool = st.db.clone();
    let r = tokio::task::spawn_blocking(move || {
        let conn = pool.blocking_lock();
        let mut stmt = conn
            .prepare(
                "SELECT id, name, key_prefix, scopes, created_at, last_used_at, revoked_at \
                 FROM api_keys WHERE user_id = ?1 ORDER BY created_at DESC",
            )
            .map_err(|e| format!("db: {e}"))?;
        let keys = stmt
            .query_map(rusqlite::params![auth.id], |r| {
                Ok(ApiKeyDto {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    prefix: r.get(2)?,
                    scopes: r.get(3)?,
                    created_at: r.get(4)?,
                    last_used_at: r.get(5)?,
                    revoked_at: r.get(6)?,
                })
            })
            .map_err(|e| format!("query_map: {e}"))?
            .filter_map(|x| x.ok())
            .collect::<Vec<_>>();
        Ok::<_, String>(keys)
    })
    .await;
    match r {
        Ok(Ok(keys)) => (StatusCode::OK, Json(json!({ "keys": keys }))),
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

// POST /api/api-keys  (auth)
async fn create_api_key(
    headers: HeaderMap,
    State(st): State<AppState>,
    Json(req): Json<CreateApiKeyReq>,
) -> impl IntoResponse {
    let auth = match verify_token_with_db(&headers, st.db.clone()).await {
        Ok(u) => u,
        Err(e) => return e,
    };
    if req.name.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "name wajib diisi" })),
        );
    }
    let scopes = req.scopes.unwrap_or_else(|| "chat,search".into());
    let id = Uuid::new_v4().to_string();
    let name = req.name.trim().to_string();
    // generate random key: lawu_ + 32 byte hex = 38 char total
    use rand::RngCore;
    let mut buf = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut buf);
    let hex_part = hex::encode(buf);
    let key = format!("{API_KEY_PREFIX}{hex_part}");
    let key_hash = auth_middleware::sha256_hex(&key);
    let prefix = format!("{API_KEY_PREFIX}{}", &hex_part[..8]);
    let now = chrono::Utc::now().to_rfc3339();
    let pool = st.db.clone();
    let resp = CreateApiKeyResp {
        id: id.clone(),
        name: name.clone(),
        key: key.clone(),
        prefix: prefix.clone(),
        scopes: scopes.clone(),
        created_at: now.clone(),
    };
    let r = tokio::task::spawn_blocking(move || -> Result<Value, (StatusCode, Value)> {
        let conn = pool.blocking_lock();
        conn.execute(
            "INSERT INTO api_keys (id, user_id, name, key_hash, key_prefix, scopes, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![&id, auth.id, &name, &key_hash, &prefix, &scopes, &now],
        )
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({ "error": format!("db insert: {e}") }),
            )
        })?;
        Ok(serde_json::to_value(&resp).unwrap_or(json!({})))
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

// DELETE /api/api-keys/:id  (auth, milik sendiri)
async fn revoke_api_key(
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
        let now = chrono::Utc::now().to_rfc3339();
        match conn.execute(
            "UPDATE api_keys SET revoked_at = ?1 WHERE id = ?2 AND user_id = ?3 AND revoked_at IS NULL",
            rusqlite::params![&now, &id, auth.id],
        ) {
            Ok(n) if n > 0 => Ok(json!({ "ok": true })),
            Ok(_) => Err((
                StatusCode::NOT_FOUND,
                json!({ "error": "api key tidak ditemukan / sudah direvoke" }),
            )),
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

// ------------------------------ main ------------------------------
#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // pre-warm jwt secret (also persists it)
    let _ = get_jwt_secret();

    let conn = open_db();
    let pool: DbPool = Arc::new(Mutex::new(conn));
    let state = AppState { db: pool };

    let app = Router::new()
        .route("/health", get(health))
        // Auth
        .route("/api/auth/login", post(login))
        .route("/api/auth/logout", post(logout))
        .route("/api/auth/me", get(me))
        // Users (admin)
        .route("/api/users", get(list_users).post(create_user))
        .route("/api/users/{id}", patch(update_user).delete(delete_user))
        // API Keys (auth, milik sendiri)
        .route("/api/api-keys", get(list_api_keys).post(create_api_key))
        .route("/api/api-keys/{id}", axum::routing::delete(revoke_api_key))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3003")
        .await
        .expect("bind 127.0.0.1:3003");
    tracing::info!("[lawu-auth] listening on http://127.0.0.1:3003");
    axum::serve(listener, app).await.expect("server error");
}
