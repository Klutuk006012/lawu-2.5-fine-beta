// ============================================================
// training_api.rs — Lawu-2.5-Fine Training Management API (port :3006)
// ------------------------------------------------------------
// Paten 3 (NO GPU): "training" = RAG augmentation, bukan fine-tuning bobot.
//
// Fitur:
//   - Material queue management (upload dokumen untuk di-ingest)
//   - Ingest trigger (spawn `lawu-ingest --file <path>`)
//   - Curriculum scheduling (cron-like auto-ingest)
//   - Evaluation: Q&A pairs test, compare dengan jawaban LLM via /api/chat
//   - Training history log
//
// Port :3006  Axum 0.8 + rusqlite + reqwest
// Shares /var/lib/lawu/auth.db dengan auth_api/chat_api.
//
// Build  : cargo build --release   →   cp training_api lawu-training
// Deploy : systemd lawu-training.service + nginx /api/training/
// ============================================================
#[path = "../auth_middleware.rs"]
mod auth_middleware;

use axum::{
    extract::{DefaultBodyLimit, Multipart, Path as AxumPath, Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{delete, get, patch, post},
    Json, Router,
};
use chrono::{Datelike, Timelike};
use auth_middleware::{require_admin, verify_token_with_db, DbPool};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::Path,
    process::Stdio,
    sync::Arc,
    time::Instant,
};
use tokio::sync::Mutex;

// ------------------------------ Konfigurasi ------------------------------
const STATE_DIR: &str = "/var/lib/lawu";
const UPLOAD_DIR: &str = "/var/lib/lawu/training_uploads";
const LOG_DIR: &str = "/var/log/lawu";
const LAWU_INGEST_BIN: &str = "/home/lawu/lawu-server/target/release/lawu-ingest";
const TANTIVY_INDEX_DIR: &str = "/mnt/data/tantivy_index/wikipedia-id-train";
const MODEL_NAME: &str = "Qwen2.5-Coder-7B Q4_K_M";
const CHAT_URL_DEFAULT: &str = "http://127.0.0.1:3005/api/chat";
const MAX_UPLOAD: usize = 500 * 1024 * 1024; // 500 MB

fn chat_url() -> String {
    std::env::var("LAWU_LLM_URL").unwrap_or_else(|_| CHAT_URL_DEFAULT.to_string())
}

#[derive(Clone)]
struct AppState {
    db: DbPool,
    http: reqwest::Client,
}

// ------------------------------ DTO (camelCase) ------------------------------
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StatusDto {
    model: String,
    active: bool,
    total_docs: i64,
    total_sessions: i64,
    last_training_at: Option<String>,
    last_eval_score: Option<f64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MaterialDto {
    id: i64,
    title: String,
    source: String,
    category: String,
    priority: String,
    doc_count: i64,
    file_path: Option<String>,
    status: String,
    created_at: String,
    indexed_at: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ScheduleDto {
    id: i64,
    name: String,
    cron_expr: String,
    action: String,
    enabled: bool,
    last_run_at: Option<String>,
    next_run_at: Option<String>,
    created_at: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HistoryDto {
    id: i64,
    started_at: String,
    finished_at: Option<String>,
    action: String,
    material_id: Option<i64>,
    doc_count: i64,
    status: String,
    duration_sec: Option<i64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct QaDto {
    id: i64,
    question: String,
    expected_answer: String,
    category: Option<String>,
    created_at: String,
    last_result: Option<EvalResultDto>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct EvalResultDto {
    actual_answer: Option<String>,
    similarity_score: Option<f64>,
    latency_ms: Option<i64>,
    run_at: String,
    status: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateScheduleReq {
    name: String,
    cron_expr: String,
    action: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PatchScheduleReq {
    enabled: Option<bool>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateQaReq {
    question: String,
    expected_answer: String,
    category: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct IngestResp {
    started: bool,
    pid: u32,
    material_id: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EvalRunResultDto {
    qa_id: i64,
    question: String,
    actual_answer: String,
    expected_answer: String,
    similarity_score: f64,
    latency_ms: i64,
    status: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EvalRunSummaryDto {
    results: Vec<EvalRunResultDto>,
    average_score: f64,
    total_soal: usize,
}

// ------------------------------ DB helpers ------------------------------
fn open_db() -> Connection {
    {
        let p = auth_middleware::auth_db_path();
        let _ = std::fs::create_dir_all(
            Path::new(&p)
                .parent()
                .unwrap_or(Path::new(STATE_DIR)),
        );
    }
    let conn = Connection::open(auth_middleware::auth_db_path()).expect("gagal buka auth.db");
    let _ = conn.pragma_update(None, "journal_mode", "WAL");
    let _ = conn.pragma_update(None, "synchronous", "NORMAL");
    let _ = conn.pragma_update(None, "foreign_keys", "ON");
    // idempotent migration: includes existing + 5 new training tables
    let sql = include_str!("../../migrations/schema.sql");
    conn.execute_batch(sql).expect("migrate schema");
    conn
}

fn ensure_dirs() {
    for d in [STATE_DIR, UPLOAD_DIR, LOG_DIR] {
        let _ = fs::create_dir_all(d);
    }
}

// ------------------------------ Helpers ------------------------------
fn now_rfc() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn sanitize_filename(raw: &str) -> String {
    let base = Path::new(raw)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("material");
    let s: String = base
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '.' || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let trimmed = s.trim();
    let truncated = if trimmed.len() > 120 {
        &trimmed[..120]
    } else {
        trimmed
    };
    format!("{}-{}", chrono::Utc::now().format("%Y%m%d%H%M%S"), truncated)
}

/// Tokenize teks menjadi set kata lowercase (untuk Jaccard / keyword overlap).
fn tokenize(s: &str) -> HashSet<String> {
    s.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| t.len() > 1)
        .map(|t| t.to_string())
        .collect()
}

/// Skor similaritas sederhana berbasis Jaccard pada set token.
/// (Tidak butuh embeddings — itu rencana Qdrant v3.7.0.)
fn similarity_jaccard(expected: &str, actual: &str) -> f64 {
    let a = tokenize(expected);
    let b = tokenize(actual);
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let inter = a.intersection(&b).count() as f64;
    let union = a.union(&b).count() as f64;
    if union == 0.0 {
        return 0.0;
    }
    inter / union
}

fn eval_status_for_score(score: f64) -> &'static str {
    if score >= 0.70 {
        "excellent"
    } else if score >= 0.40 {
        "good"
    } else {
        "needs_improvement"
    }
}

/// Ambil nilai kolom `enabled` sebagai bool (INTEGER 0/1).
fn row_bool(r: &rusqlite::Row, idx: usize) -> rusqlite::Result<bool> {
    let v: i64 = r.get(idx)?;
    Ok(v != 0)
}

/// Estimasi jumlah dokumen di index Tantivy (heuristic: hitung segment files).
/// Untuk angka presisi, gunakan Tantivy reader API (rencana v3.7.0 dengan Qdrant).
fn count_tantivy_docs() -> i64 {
    let p = Path::new(TANTIVY_INDEX_DIR);
    if !p.exists() {
        return 0;
    }
    let mut n: i64 = 0;
    if let Ok(rd) = fs::read_dir(p) {
        for entry in rd.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with(".store") || name.starts_with("segment.") {
                n += 1;
            }
        }
    }
    n
}

// ------------------------------ Cron matcher (5-field) ------------------------------
/// Parsing sederhana cron 5-field: menit jam dom month dow.
/// Field bisa: "*" | angka | "*/n" | "a,b" | "a-b".
fn cron_field_matches(field: &str, value: u32, min: u32, max: u32) -> bool {
    let _ = (min, max);
    for part in field.split(',') {
        let part = part.trim();
        if part == "*" {
            return true;
        }
        if let Some(step_str) = part.strip_prefix("*/") {
            if let Ok(step) = step_str.parse::<u32>() {
                if step == 0 {
                    continue;
                }
                if value % step == 0 {
                    return true;
                }
            }
            continue;
        }
        if let Some((a, b)) = part.split_once('-') {
            if let (Ok(a), Ok(b)) = (a.trim().parse::<u32>(), b.trim().parse::<u32>()) {
                if value >= a && value <= b {
                    return true;
                }
            }
            continue;
        }
        if let Ok(v) = part.parse::<u32>() {
            if v == value {
                return true;
            }
        }
    }
    false
}

/// Cek apakah ekspresi cron cocok dengan waktu UTC tertentu.
fn cron_matches(expr: &str, dt: chrono::DateTime<chrono::Utc>) -> bool {
    let fields: Vec<&str> = expr.split_whitespace().collect();
    if fields.len() != 5 {
        return false;
    }
    let m = dt.minute() as u32;
    let h = dt.hour() as u32;
    let dom = dt.day();
    let mon = dt.month();
    let dow = dt.weekday().num_days_from_sunday(); // Sunday=0..Saturday=6
    let dow_field = fields[4];
    // Standar cron: 0/7 = Sunday. chrono num_days_from_sunday: Sunday=0.
    let dow_match = cron_field_matches(dow_field, dow, 0, 6)
        || (dow == 0 && (dow_field == "7" || dow_field.split(',').any(|p| p.trim() == "7")));
    cron_field_matches(fields[0], m, 0, 59)
        && cron_field_matches(fields[1], h, 0, 23)
        && cron_field_matches(fields[2], dom, 1, 31)
        && cron_field_matches(fields[3], mon, 1, 12)
        && dow_match
}

/// Hitung next_run_at sebagai menit berikutnya yang cocok (greedy, paling lama 7 hari).
fn next_run_at(expr: &str, from: chrono::DateTime<chrono::Utc>) -> Option<String> {
    let mut t = from
        .with_second(0)?
        .with_nanosecond(0)?
        .checked_add_signed(chrono::Duration::minutes(1))?;
    for _ in 0..(60 * 24 * 7) {
        if cron_matches(expr, t) {
            return Some(t.to_rfc3339());
        }
        t = t.checked_add_signed(chrono::Duration::minutes(1))?;
    }
    None
}

// ------------------------------ Spawn ingest ------------------------------
async fn spawn_ingest(file_path: String) -> Result<u32, String> {
    let _ = fs::create_dir_all(STATE_DIR);
    let mut cmd = tokio::process::Command::new(LAWU_INGEST_BIN);
    cmd.arg("--file").arg(&file_path)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(false);
    let child = cmd.spawn().map_err(|e| format!("spawn lawu-ingest: {e}"))?;
    Ok(child.id().unwrap_or(0))
}

// ------------------------------ Handlers: health ------------------------------
async fn health() -> impl IntoResponse {
    Json(json!({
        "status": "ok",
        "service": "lawu-training",
        "port": 3006,
        "ts": now_rfc(),
    }))
}

// ------------------------------ Handlers: status ------------------------------
// GET /api/training/status (auth)
async fn get_status(headers: HeaderMap, State(st): State<AppState>) -> impl IntoResponse {
    let auth = match verify_token_with_db(&headers, st.db.clone()).await {
        Ok(u) => u,
        Err(e) => return e,
    };
    let _ = auth; // user terautentikasi (tidak perlu admin untuk GET status)
    let pool = st.db.clone();
    let r = tokio::task::spawn_blocking(move || -> Result<StatusDto, String> {
        let conn = pool.blocking_lock();
        let total_sessions: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM training_history WHERE status = 'success'",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        let last_training_at: Option<String> = conn
            .query_row(
                "SELECT finished_at FROM training_history \
                 WHERE action IN ('ingest','crawl','extract') AND finished_at IS NOT NULL \
                 ORDER BY finished_at DESC LIMIT 1",
                [],
                |r| r.get(0),
            )
            .ok();
        let last_eval: Option<(f64, String)> = conn
            .query_row(
                "SELECT similarity_score, run_at FROM evaluation_results \
                 ORDER BY run_at DESC LIMIT 1",
                [],
                |r| {
                    Ok((
                        r.get::<_, Option<f64>>(0)?.unwrap_or(0.0),
                        r.get::<_, String>(1)?,
                    ))
                },
            )
            .ok();
        let total_docs = count_tantivy_docs();
        Ok(StatusDto {
            model: MODEL_NAME.to_string(),
            active: true,
            total_docs,
            total_sessions,
            last_training_at,
            last_eval_score: last_eval.map(|(s, _)| s),
        })
    })
    .await;
    match r {
        Ok(Ok(dto)) => (StatusCode::OK, Json(json!(dto))),
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

// ------------------------------ Handlers: materials ------------------------------
// GET /api/training/materials (auth)
async fn list_materials(headers: HeaderMap, State(st): State<AppState>) -> impl IntoResponse {
    let _auth = match verify_token_with_db(&headers, st.db.clone()).await {
        Ok(u) => u,
        Err(e) => return e,
    };
    let pool = st.db.clone();
    let r = tokio::task::spawn_blocking(move || -> Result<Vec<MaterialDto>, String> {
        let conn = pool.blocking_lock();
        let mut stmt = conn
            .prepare(
                "SELECT id, title, source, category, priority, doc_count, \
                 file_path, status, created_at, indexed_at \
                 FROM training_materials ORDER BY id DESC",
            )
            .map_err(|e| format!("db: {e}"))?;
        let rows = stmt
            .query_map([], |r| {
                Ok(MaterialDto {
                    id: r.get(0)?,
                    title: r.get(1)?,
                    source: r.get(2)?,
                    category: r.get(3)?,
                    priority: r.get(4)?,
                    doc_count: r.get(5)?,
                    file_path: r.get(6)?,
                    status: r.get(7)?,
                    created_at: r.get(8)?,
                    indexed_at: r.get(9)?,
                })
            })
            .map_err(|e| format!("query_map: {e}"))?
            .filter_map(|x| x.ok())
            .collect::<Vec<_>>();
        Ok(rows)
    })
    .await;
    match r {
        Ok(Ok(rows)) => (StatusCode::OK, Json(json!({ "materials": rows }))),
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

// POST /api/training/materials/upload (admin, multipart)
async fn upload_material(
    headers: HeaderMap,
    State(st): State<AppState>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let auth = match verify_token_with_db(&headers, st.db.clone()).await {
        Ok(u) => u,
        Err(e) => return e,
    };
    if let Err(e) = require_admin(&auth) {
        return e;
    }

    // Form fields: title, source, category, priority, file
    let mut title: Option<String> = None;
    let mut source: Option<String> = None;
    let mut category: Option<String> = None;
    let mut priority: Option<String> = None;
    let mut file_name: Option<String> = None;
    let mut file_bytes: Vec<u8> = Vec::new();

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "file" => {
                file_name = field.file_name().map(|s| s.to_string());
                let bytes = match field.bytes().await {
                    Ok(b) => b.to_vec(),
                    Err(e) => {
                        return (
                            StatusCode::BAD_REQUEST,
                            Json(json!({ "error": format!("baca file gagal: {e}") })),
                        );
                    }
                };
                file_bytes = bytes;
            }
            "title" => title = field.text().await.ok().map(|s| s.trim().to_string()),
            "source" => source = field.text().await.ok().map(|s| s.trim().to_string()),
            "category" => category = field.text().await.ok().map(|s| s.trim().to_string()),
            "priority" => priority = field.text().await.ok().map(|s| s.trim().to_string()),
            _ => {
                // field tidak dikenal — skip
                let _ = field.bytes().await;
            }
        }
    }

    let title = match title {
        Some(t) if !t.is_empty() => t,
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "field 'title' wajib" })),
            );
        }
    };
    let source = source.unwrap_or_else(|| "custom_upload".to_string());
    let category = category.unwrap_or_else(|| "umum".to_string());
    let priority = priority.unwrap_or_else(|| "sedang".to_string());
    let file_name = file_name.unwrap_or_else(|| "material.jsonl".to_string());

    if file_bytes.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "file kosong" })),
        );
    }

    let safe_name = sanitize_filename(&file_name);
    let dest = format!("{UPLOAD_DIR}/{safe_name}");
    if let Err(e) = fs::write(&dest, &file_bytes) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": format!("tulis file gagal: {e}") })),
        );
    }

    let now = now_rfc();
    let pool = st.db.clone();
    let file_path_for_db = dest.clone();
    let r = tokio::task::spawn_blocking(move || -> Result<MaterialDto, String> {
        let conn = pool.blocking_lock();
        conn.execute(
            "INSERT INTO training_materials \
             (title, source, category, priority, doc_count, file_path, status, created_at) \
             VALUES (?1, ?2, ?3, ?4, 0, ?5, 'queued', ?6)",
            rusqlite::params![&title, &source, &category, &priority, &file_path_for_db, &now],
        )
        .map_err(|e| format!("db insert: {e}"))?;
        let id = conn.last_insert_rowid();
        Ok(MaterialDto {
            id,
            title,
            source,
            category,
            priority,
            doc_count: 0,
            file_path: Some(file_path_for_db),
            status: "queued".to_string(),
            created_at: now,
            indexed_at: None,
        })
    })
    .await;
    match r {
        Ok(Ok(dto)) => (StatusCode::CREATED, Json(json!({ "material": dto }))),
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

// POST /api/training/ingest/:materialId (admin)
async fn trigger_ingest(
    headers: HeaderMap,
    State(st): State<AppState>,
    AxumPath(material_id): AxumPath<i64>,
) -> impl IntoResponse {
    let auth = match verify_token_with_db(&headers, st.db.clone()).await {
        Ok(u) => u,
        Err(e) => return e,
    };
    if let Err(e) = require_admin(&auth) {
        return e;
    }

    // 1. load material
    let pool = st.db.clone();
    let material = tokio::task::spawn_blocking(move || -> Result<(String, String), Value> {
        let conn = pool.blocking_lock();
        let row: Option<(Option<String>, String)> = conn
            .query_row(
                "SELECT file_path, status FROM training_materials WHERE id = ?1",
                rusqlite::params![material_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .ok();
        match row {
            Some((Some(path), status)) if status != "ingesting" => Ok((path, status)),
            Some((Some(_), status)) => Err(json!({
                "error": format!("material sedang status '{status}', tidak bisa trigger ingest")
            })),
            Some((None, _)) => Err(json!({ "error": "material tidak punya file_path" })),
            None => Err(json!({ "error": "material tidak ditemukan" })),
        }
    })
    .await;
    let (file_path, _prev_status) = match material {
        Ok(Ok(x)) => x,
        Ok(Err(v)) => return (StatusCode::BAD_REQUEST, Json(v)),
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": format!("spawn: {e}") })),
            );
        }
    };

    // 2. set status='ingesting', insert history row (action='ingest', status='running')
    let now = now_rfc();
    let now_for_history = now.clone();
    let pool = st.db.clone();
    let fp_for_log = file_path.clone();
    let history_id = tokio::task::spawn_blocking(move || -> Result<i64, String> {
        let conn = pool.blocking_lock();
        conn.execute(
            "UPDATE training_materials SET status = 'ingesting' WHERE id = ?1",
            rusqlite::params![material_id],
        )
        .map_err(|e| format!("db update: {e}"))?;
        conn.execute(
            "INSERT INTO training_history \
             (started_at, action, material_id, doc_count, status, log) \
             VALUES (?1, 'ingest', ?2, 0, 'running', ?3)",
            rusqlite::params![&now_for_history, material_id, format!("spawn lawu-ingest --file {fp_for_log}")],
        )
        .map_err(|e| format!("db insert history: {e}"))?;
        Ok(conn.last_insert_rowid())
    })
    .await;
    let history_id = match history_id {
        Ok(Ok(id)) => id,
        Ok(Err(e)) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e })),
            );
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": format!("spawn: {e}") })),
            );
        }
    };

    // 3. spawn lawu-ingest (background, fire-and-forget)
    let pid = match spawn_ingest(file_path.clone()).await {
        Ok(p) => p,
        Err(e) => {
            // rollback status + history
            let err_msg = e.clone();
            let pool2 = st.db.clone();
            let _ = tokio::task::spawn_blocking(move || {
                let conn = pool2.blocking_lock();
                let _ = conn.execute(
                    "UPDATE training_materials SET status = 'failed' WHERE id = ?1",
                    rusqlite::params![material_id],
                );
                let _ = conn.execute(
                    "UPDATE training_history SET status = 'failed', log = ?1, \
                     finished_at = ?2 WHERE id = ?3",
                    rusqlite::params![format!("spawn gagal: {e}"), now_rfc(), history_id],
                );
            })
            .await;
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": err_msg })),
            );
        }
    };

    // 4. spawn watchdog: poll pid, update status when done
    let pool_w = st.db.clone();
    let started_clone = now.clone();
    tokio::spawn(async move {
        // Poll pid alive setiap 5 detik, maks 30 menit
        let max_iters = (30 * 60) / 5;
        let mut alive = true;
        for _ in 0..max_iters {
            if !Path::new(&format!("/proc/{pid}")).exists() {
                alive = false;
                break;
            }
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        }
        let pool_inner = pool_w.clone();
        let now2 = now_rfc();
        let status_final = if alive { "running" } else { "success" };
        let _ = tokio::task::spawn_blocking(move || {
            let conn = pool_inner.blocking_lock();
            let _ = conn.execute(
                "UPDATE training_materials SET status = ?1, indexed_at = ?2 WHERE id = ?3",
                rusqlite::params![status_final, &now2, material_id],
            );
            let _ = conn.execute(
                "UPDATE training_history SET status = ?1, finished_at = ?2, \
                 duration_sec = CAST((julianday(?2) - julianday(?3)) * 86400 AS INTEGER) \
                 WHERE id = ?4",
                rusqlite::params![status_final, &now2, &started_clone, history_id],
            );
        })
        .await;
    });

    (
        StatusCode::ACCEPTED,
        Json(json!(IngestResp {
            started: true,
            pid,
            material_id,
        })),
    )
}

// ------------------------------ Handlers: schedule ------------------------------
// GET /api/training/schedule (auth)
async fn list_schedule(headers: HeaderMap, State(st): State<AppState>) -> impl IntoResponse {
    let _auth = match verify_token_with_db(&headers, st.db.clone()).await {
        Ok(u) => u,
        Err(e) => return e,
    };
    let pool = st.db.clone();
    let r = tokio::task::spawn_blocking(move || -> Result<Vec<ScheduleDto>, String> {
        let conn = pool.blocking_lock();
        let mut stmt = conn
            .prepare(
                "SELECT id, name, cron_expr, action, enabled, \
                 last_run_at, next_run_at, created_at \
                 FROM training_schedule ORDER BY id ASC",
            )
            .map_err(|e| format!("db: {e}"))?;
        let rows = stmt
            .query_map([], |r| {
                Ok(ScheduleDto {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    cron_expr: r.get(2)?,
                    action: r.get(3)?,
                    enabled: row_bool(r, 4)?,
                    last_run_at: r.get(5)?,
                    next_run_at: r.get(6)?,
                    created_at: r.get(7)?,
                })
            })
            .map_err(|e| format!("query_map: {e}"))?
            .filter_map(|x| x.ok())
            .collect::<Vec<_>>();
        Ok(rows)
    })
    .await;
    match r {
        Ok(Ok(rows)) => (StatusCode::OK, Json(json!({ "schedule": rows }))),
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

// POST /api/training/schedule (admin) body {name, cronExpr, action}
async fn create_schedule(
    headers: HeaderMap,
    State(st): State<AppState>,
    Json(req): Json<CreateScheduleReq>,
) -> impl IntoResponse {
    let auth = match verify_token_with_db(&headers, st.db.clone()).await {
        Ok(u) => u,
        Err(e) => return e,
    };
    if let Err(e) = require_admin(&auth) {
        return e;
    }
    // Validasi cron 5-field sederhana
    if req.cron_expr.split_whitespace().count() != 5 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "cronExpr harus 5 field: menit jam dom month dow" })),
        );
    }
    let now = now_rfc();
    let next = next_run_at(&req.cron_expr, chrono::Utc::now()).unwrap_or_else(|| now.clone());
    let pool = st.db.clone();
    let r = tokio::task::spawn_blocking(move || -> Result<ScheduleDto, String> {
        let conn = pool.blocking_lock();
        conn.execute(
            "INSERT INTO training_schedule \
             (name, cron_expr, action, enabled, last_run_at, next_run_at, created_at) \
             VALUES (?1, ?2, ?3, 1, NULL, ?4, ?5)",
            rusqlite::params![&req.name, &req.cron_expr, &req.action, &next, &now],
        )
        .map_err(|e| format!("db insert: {e}"))?;
        let id = conn.last_insert_rowid();
        Ok(ScheduleDto {
            id,
            name: req.name,
            cron_expr: req.cron_expr,
            action: req.action,
            enabled: true,
            last_run_at: None,
            next_run_at: Some(next),
            created_at: now,
        })
    })
    .await;
    match r {
        Ok(Ok(dto)) => (StatusCode::CREATED, Json(json!({ "schedule": dto }))),
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

// PATCH /api/training/schedule/:id (admin) body {enabled?}
async fn patch_schedule(
    headers: HeaderMap,
    State(st): State<AppState>,
    AxumPath(id): AxumPath<i64>,
    Json(req): Json<PatchScheduleReq>,
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
        if let Some(enabled) = req.enabled {
            let n = conn
                .execute(
                    "UPDATE training_schedule SET enabled = ?1 WHERE id = ?2",
                    rusqlite::params![if enabled { 1i64 } else { 0 }, id],
                )
                .map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        json!({ "error": format!("db: {e}") }),
                    )
                })?;
            if n == 0 {
                return Err((
                    StatusCode::NOT_FOUND,
                    json!({ "error": "schedule tidak ditemukan" }),
                ));
            }
        }
        let row: Option<(String, String, String, bool, Option<String>, Option<String>, String)> = conn
            .query_row(
                "SELECT name, cron_expr, action, enabled, last_run_at, next_run_at, created_at \
                 FROM training_schedule WHERE id = ?1",
                rusqlite::params![id],
                |r| {
                    Ok((
                        r.get(0)?,
                        r.get(1)?,
                        r.get(2)?,
                        row_bool(r, 3)?,
                        r.get(4)?,
                        r.get(5)?,
                        r.get(6)?,
                    ))
                },
            )
            .ok();
        match row {
            Some((name, cron_expr, action, enabled, last_run_at, next_run_at, created_at)) => {
                Ok(json!({
                    "schedule": ScheduleDto {
                        id, name, cron_expr, action, enabled, last_run_at, next_run_at, created_at,
                    }
                }))
            }
            None => Err((
                StatusCode::NOT_FOUND,
                json!({ "error": "schedule tidak ditemukan" }),
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

// DELETE /api/training/schedule/:id (admin)
async fn delete_schedule(
    headers: HeaderMap,
    State(st): State<AppState>,
    AxumPath(id): AxumPath<i64>,
) -> impl IntoResponse {
    let auth = match verify_token_with_db(&headers, st.db.clone()).await {
        Ok(u) => u,
        Err(e) => return e,
    };
    if let Err(e) = require_admin(&auth) {
        return e;
    }
    let pool = st.db.clone();
    let r = tokio::task::spawn_blocking(move || -> Result<(), (StatusCode, Value)> {
        let conn = pool.blocking_lock();
        match conn.execute(
            "DELETE FROM training_schedule WHERE id = ?1",
            rusqlite::params![id],
        ) {
            Ok(n) if n > 0 => Ok(()),
            Ok(_) => Err((
                StatusCode::NOT_FOUND,
                json!({ "error": "schedule tidak ditemukan" }),
            )),
            Err(e) => Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({ "error": format!("db: {e}") }),
            )),
        }
    })
    .await;
    match r {
        Ok(Ok(())) => (StatusCode::OK, Json(json!({ "ok": true }))),
        Ok(Err((s, v))) => (s, Json(v)),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": format!("spawn: {e}") })),
        ),
    }
}

// ------------------------------ Handlers: history ------------------------------
// GET /api/training/history?limit=50 (auth)
async fn list_history(
    headers: HeaderMap,
    State(st): State<AppState>,
    Query(q): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let _auth = match verify_token_with_db(&headers, st.db.clone()).await {
        Ok(u) => u,
        Err(e) => return e,
    };
    let limit: i64 = q
        .get("limit")
        .and_then(|s| s.parse().ok())
        .unwrap_or(50)
        .clamp(1, 500);
    let pool = st.db.clone();
    let r = tokio::task::spawn_blocking(move || -> Result<Vec<HistoryDto>, String> {
        let conn = pool.blocking_lock();
        let mut stmt = conn
            .prepare(
                "SELECT id, started_at, finished_at, action, material_id, \
                 doc_count, status, duration_sec \
                 FROM training_history ORDER BY started_at DESC LIMIT ?1",
            )
            .map_err(|e| format!("db: {e}"))?;
        let rows = stmt
            .query_map(rusqlite::params![limit], |r| {
                Ok(HistoryDto {
                    id: r.get(0)?,
                    started_at: r.get(1)?,
                    finished_at: r.get(2)?,
                    action: r.get(3)?,
                    material_id: r.get(4)?,
                    doc_count: r.get(5)?,
                    status: r.get(6)?,
                    duration_sec: r.get(7)?,
                })
            })
            .map_err(|e| format!("query_map: {e}"))?
            .filter_map(|x| x.ok())
            .collect::<Vec<_>>();
        Ok(rows)
    })
    .await;
    match r {
        Ok(Ok(rows)) => (StatusCode::OK, Json(json!({ "history": rows }))),
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

// ------------------------------ Handlers: evaluations ------------------------------
// GET /api/training/evaluations (auth)
async fn list_evaluations(headers: HeaderMap, State(st): State<AppState>) -> impl IntoResponse {
    let _auth = match verify_token_with_db(&headers, st.db.clone()).await {
        Ok(u) => u,
        Err(e) => return e,
    };
    let pool = st.db.clone();
    let r = tokio::task::spawn_blocking(move || -> Result<Vec<QaDto>, String> {
        let conn = pool.blocking_lock();
        let mut stmt = conn
            .prepare(
                "SELECT q.id, q.question, q.expected_answer, q.category, q.created_at \
                 FROM evaluation_qa q ORDER BY q.id ASC",
            )
            .map_err(|e| format!("db: {e}"))?;
        let mut rows: Vec<QaDto> = Vec::new();
        let qa_iter = stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, Option<String>>(3)?,
                    r.get::<_, String>(4)?,
                ))
            })
            .map_err(|e| format!("query_map: {e}"))?;
        for qa in qa_iter.flatten() {
            let (id, question, expected_answer, category, created_at) = qa;
            // last result untuk QA ini
            let last_result: Option<EvalResultDto> = conn
                .query_row(
                    "SELECT actual_answer, similarity_score, latency_ms, run_at, status \
                     FROM evaluation_results WHERE qa_id = ?1 ORDER BY run_at DESC LIMIT 1",
                    rusqlite::params![id],
                    |r| {
                        Ok(EvalResultDto {
                            actual_answer: r.get(0)?,
                            similarity_score: r.get(1)?,
                            latency_ms: r.get(2)?,
                            run_at: r.get(3)?,
                            status: r.get(4)?,
                        })
                    },
                )
                .ok();
            rows.push(QaDto {
                id,
                question,
                expected_answer,
                category,
                created_at,
                last_result,
            });
        }
        Ok(rows)
    })
    .await;
    match r {
        Ok(Ok(rows)) => (StatusCode::OK, Json(json!({ "qa": rows }))),
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

// POST /api/training/evaluations (admin)
async fn create_evaluation(
    headers: HeaderMap,
    State(st): State<AppState>,
    Json(req): Json<CreateQaReq>,
) -> impl IntoResponse {
    let auth = match verify_token_with_db(&headers, st.db.clone()).await {
        Ok(u) => u,
        Err(e) => return e,
    };
    if let Err(e) = require_admin(&auth) {
        return e;
    }
    let now = now_rfc();
    let pool = st.db.clone();
    let r = tokio::task::spawn_blocking(move || -> Result<QaDto, String> {
        let conn = pool.blocking_lock();
        conn.execute(
            "INSERT INTO evaluation_qa (question, expected_answer, category, created_at) \
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![&req.question, &req.expected_answer, &req.category, &now],
        )
        .map_err(|e| format!("db insert: {e}"))?;
        let id = conn.last_insert_rowid();
        Ok(QaDto {
            id,
            question: req.question,
            expected_answer: req.expected_answer,
            category: req.category,
            created_at: now,
            last_result: None,
        })
    })
    .await;
    match r {
        Ok(Ok(dto)) => (StatusCode::CREATED, Json(json!({ "qa": dto }))),
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

// DELETE /api/training/evaluations/:id (admin)
async fn delete_evaluation(
    headers: HeaderMap,
    State(st): State<AppState>,
    AxumPath(id): AxumPath<i64>,
) -> impl IntoResponse {
    let auth = match verify_token_with_db(&headers, st.db.clone()).await {
        Ok(u) => u,
        Err(e) => return e,
    };
    if let Err(e) = require_admin(&auth) {
        return e;
    }
    let pool = st.db.clone();
    let r = tokio::task::spawn_blocking(move || -> Result<(), (StatusCode, Value)> {
        let conn = pool.blocking_lock();
        match conn.execute("DELETE FROM evaluation_qa WHERE id = ?1", rusqlite::params![id]) {
            Ok(n) if n > 0 => Ok(()),
            Ok(_) => Err((
                StatusCode::NOT_FOUND,
                json!({ "error": "qa tidak ditemukan" }),
            )),
            Err(e) => Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({ "error": format!("db: {e}") }),
            )),
        }
    })
    .await;
    match r {
        Ok(Ok(())) => (StatusCode::OK, Json(json!({ "ok": true }))),
        Ok(Err((s, v))) => (s, Json(v)),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": format!("spawn: {e}") })),
        ),
    }
}

// POST /api/training/evaluations/run (admin)
// Jalankan semua Q&A via /api/chat, compare similarity, simpan hasil.
async fn run_evaluations(
    headers: HeaderMap,
    State(st): State<AppState>,
) -> impl IntoResponse {
    let auth = match verify_token_with_db(&headers, st.db.clone()).await {
        Ok(u) => u,
        Err(e) => return e,
    };
    if let Err(e) = require_admin(&auth) {
        return e;
    }

    // Forward Authorization header ke chat_api — chat_api butuh auth valid.
    let bearer = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_default();

    // 1. Load semua QA
    let pool = st.db.clone();
    let qas = tokio::task::spawn_blocking(move || -> Result<Vec<(i64, String, String)>, String> {
        let conn = pool.blocking_lock();
        let mut stmt = conn
            .prepare("SELECT id, question, expected_answer FROM evaluation_qa ORDER BY id ASC")
            .map_err(|e| format!("db: {e}"))?;
        let rows = stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                ))
            })
            .map_err(|e| format!("query_map: {e}"))?
            .filter_map(|x| x.ok())
            .collect::<Vec<_>>();
        Ok(rows)
    })
    .await;
    let qas = match qas {
        Ok(Ok(v)) => v,
        Ok(Err(e)) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e })),
            );
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": format!("spawn: {e}") })),
            );
        }
    };

    if qas.is_empty() {
        return (
            StatusCode::OK,
            Json(json!(EvalRunSummaryDto {
                results: vec![],
                average_score: 0.0,
                total_soal: 0,
            })),
        );
    }

    // 2. Buat conversation sementara untuk eval run (via /api/conversations)
    let conv_id: Option<String> = match st
        .http
        .post("http://127.0.0.1:3005/api/conversations")
        .header("Authorization", &bearer)
        .json(&json!({ "title": "training-eval-run" }))
        .send()
        .await
    {
        Ok(r) if r.status().is_success() => r
            .json::<Value>()
            .await
            .ok()
            .and_then(|v| {
                v.get("session")
                    .and_then(|s| s.get("id"))
                    .and_then(|x| x.as_str())
                    .map(|s| s.to_string())
            }),
        _ => None,
    };
    let conv_id = match conv_id {
        Some(c) => c,
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({
                    "error": "gagal buat conversation via chat_api (:3005). Pastikan lawu-chat berjalan."
                })),
            );
        }
    };

    // 3. Insert history row 'evaluate' running
    let now_start = now_rfc();
    let pool_h = st.db.clone();
    let total_qa = qas.len() as i64;
    let conv_id_for_log = conv_id.clone();
    let now_start_clone = now_start.clone();
    let history_id = tokio::task::spawn_blocking(move || -> Result<i64, String> {
        let conn = pool_h.blocking_lock();
        conn.execute(
            "INSERT INTO training_history \
             (started_at, action, material_id, doc_count, status, log) \
             VALUES (?1, 'evaluate', NULL, ?2, 'running', ?3)",
            rusqlite::params![
                &now_start_clone,
                total_qa,
                format!("eval run conv_id={conv_id_for_log} total_soal={total_qa}")
            ],
        )
        .map_err(|e| format!("db insert history: {e}"))?;
        Ok(conn.last_insert_rowid())
    })
    .await;
    let history_id = match history_id {
        Ok(Ok(id)) => id,
        Ok(Err(e)) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e })),
            );
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": format!("spawn: {e}") })),
            );
        }
    };

    // 4. Untuk tiap QA: POST /api/chat dengan message=question, ambil assistantMessage.content
    let chat_url = chat_url();
    let mut results: Vec<EvalRunResultDto> = Vec::new();
    let mut log_lines: Vec<String> = Vec::new();

    for (qa_id, question, expected) in qas.iter() {
        let started = Instant::now();
        let body = json!({ "conversationId": conv_id, "message": question });
        let resp = st
            .http
            .post(&chat_url)
            .header("Authorization", &bearer)
            .json(&body)
            .send()
            .await;
        let latency_ms = started.elapsed().as_millis() as i64;
        let actual = match resp {
            Ok(r) if r.status().is_success() => match r.json::<Value>().await {
                Ok(v) => v
                    .get("assistantMessage")
                    .and_then(|m| m.get("content"))
                    .and_then(|x| x.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| {
                        format!("[error: response tanpa assistantMessage.content — {}]", v)
                    }),
                Err(_) => "[error: response tidak ter-parse]".to_string(),
            },
            Ok(r) => format!("[error: chat_api status {}]", r.status()),
            Err(e) => format!("[error: gagal connect chat_api — {e}]"),
        };
        let score = similarity_jaccard(expected, &actual);
        let status = eval_status_for_score(score).to_string();
        log_lines.push(format!(
            "[qa#{}] score={:.3} status={} latency={}ms",
            qa_id, score, status, latency_ms
        ));

        // simpan ke evaluation_results
        let pool_r = st.db.clone();
        let now_rfc = now_rfc();
        let actual_clone = actual.clone();
        let status_clone = status.clone();
        let qa_id_val = *qa_id;
        let _ = tokio::task::spawn_blocking(move || {
            let conn = pool_r.blocking_lock();
            let _ = conn.execute(
                "INSERT INTO evaluation_results \
                 (qa_id, actual_answer, similarity_score, latency_ms, run_at, status) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![
                    qa_id_val,
                    &actual_clone,
                    score,
                    latency_ms,
                    &now_rfc,
                    &status_clone,
                ],
            );
        })
        .await;

        results.push(EvalRunResultDto {
            qa_id: *qa_id,
            question: question.clone(),
            actual_answer: actual,
            expected_answer: expected.clone(),
            similarity_score: score,
            latency_ms,
            status,
        });
    }

    // 5. Hitung average + update history
    let average = if results.is_empty() {
        0.0
    } else {
        results.iter().map(|r| r.similarity_score).sum::<f64>() / results.len() as f64
    };
    let total_soal = results.len();

    let pool_f = st.db.clone();
    let log_text = log_lines.join("\n");
    let final_status = if average >= 0.60 {
        "success"
    } else if average >= 0.30 {
        "partial"
    } else {
        "failed"
    };
    let _ = tokio::task::spawn_blocking(move || {
        let conn = pool_f.blocking_lock();
        let now_end = now_rfc();
        let _ = conn.execute(
            "UPDATE training_history SET status = ?1, finished_at = ?2, log = ?3, \
             duration_sec = CAST((julianday(?2) - julianday(?4)) * 86400 AS INTEGER) \
             WHERE id = ?5",
            rusqlite::params![final_status, &now_end, &log_text, &now_start, history_id,],
        );
    })
    .await;

    let _ = auth;
    (
        StatusCode::OK,
        Json(json!(EvalRunSummaryDto {
            results,
            average_score: average,
            total_soal,
        })),
    )
}

// ------------------------------ Schedule runner (background) ------------------------------
fn spawn_schedule_runner(state: AppState) {
    tokio::spawn(async move {
        // Cek setiap 60 detik.
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        // Lewati tick pertama (interval langsung fire saat start).
        interval.tick().await;
        loop {
            interval.tick().await;
            let now = chrono::Utc::now();
            // Ambil semua schedule enabled
            let pool = state.db.clone();
            let due = tokio::task::spawn_blocking(move || -> Vec<(i64, String, String)> {
                let conn = pool.blocking_lock();
                let mut stmt = match conn.prepare(
                    "SELECT id, cron_expr, action FROM training_schedule WHERE enabled = 1",
                ) {
                    Ok(s) => s,
                    Err(_) => return vec![],
                };
                let rows = stmt
                    .query_map([], |r| {
                        Ok((
                            r.get::<_, i64>(0)?,
                            r.get::<_, String>(1)?,
                            r.get::<_, String>(2)?,
                        ))
                    })
                    .ok()
                    .map(|iter| iter.filter_map(|x| x.ok()).collect::<Vec<_>>())
                    .unwrap_or_default();
                rows
            })
            .await;
            let due = match due {
                Ok(v) => v,
                Err(_) => continue,
            };

            for (id, cron_expr, action) in due {
                if !cron_matches(&cron_expr, now) {
                    continue;
                }
                tracing::info!(
                    "schedule_runner: schedule#{} '{}' action='{}' due — running",
                    id,
                    cron_expr,
                    action
                );
                // Cegah double-run di menit yang sama: cek last_run_at.
                let pool_l = state.db.clone();
                let last_min = tokio::task::spawn_blocking(move || -> Option<String> {
                    let conn = pool_l.blocking_lock();
                    conn.query_row(
                        "SELECT last_run_at FROM training_schedule WHERE id = ?1",
                        rusqlite::params![id],
                        |r| r.get::<_, Option<String>>(0),
                    )
                    .ok()
                    .flatten()
                })
                .await
                .ok()
                .flatten();
                if let Some(last) = last_min {
                    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&last) {
                        let dt_utc: chrono::DateTime<chrono::Utc> =
                            dt.with_timezone(&chrono::Utc);
                        if (now - dt_utc).num_seconds() < 60 {
                            continue;
                        }
                    }
                }

                // Update last_run_at + next_run_at + insert history row running.
                let now_rfc = now.to_rfc3339();
                let next = next_run_at(&cron_expr, now).unwrap_or_else(|| now_rfc.clone());
                let pool_u = state.db.clone();
                let action_for_log = action.clone();
                let _ = tokio::task::spawn_blocking(move || {
                    let conn = pool_u.blocking_lock();
                    let _ = conn.execute(
                        "UPDATE training_schedule SET last_run_at = ?1, next_run_at = ?2 WHERE id = ?3",
                        rusqlite::params![&now_rfc, &next, id],
                    );
                    let _ = conn.execute(
                        "INSERT INTO training_history \
                         (started_at, action, material_id, doc_count, status, log) \
                         VALUES (?1, ?2, NULL, 0, 'running', ?3)",
                        rusqlite::params![
                            &now_rfc,
                            &action_for_log,
                            format!("schedule_runner auto-trigger cron='{}'", cron_expr),
                        ],
                    );
                })
                .await;

                // Eksekusi action via internal HTTP (best-effort).
                // Untuk action 'crawl_web'/'extract_ebooks'/'ingest_skill' kita trigger
                // corpus_api endpoint; selain itu log saja.
                let endpoint = match action.as_str() {
                    "crawl_web" => Some((
                        "POST",
                        "http://127.0.0.1:3002/api/corpus/web/crawl".to_string(),
                    )),
                    "extract_ebooks" => Some((
                        "POST",
                        "http://127.0.0.1:3002/api/ebooks/extract".to_string(),
                    )),
                    "ingest_skill" => Some((
                        "POST",
                        "http://127.0.0.1:3002/api/corpus/ingest".to_string(),
                    )),
                    _ => None,
                };
                if let Some((method, url)) = endpoint {
                    let req = if method == "POST" {
                        state.http.post(&url).json(&json!({}))
                    } else {
                        state.http.get(&url)
                    };
                    match req.send().await {
                        Ok(r) => tracing::info!(
                            "schedule_runner: schedule#{} action='{}' -> {} status {}",
                            id,
                            action,
                            url,
                            r.status()
                        ),
                        Err(e) => tracing::warn!(
                            "schedule_runner: schedule#{} action='{}' -> error: {}",
                            id,
                            action,
                            e
                        ),
                    }
                }
            }
        }
    });
}

// ------------------------------ main ------------------------------
#[tokio::main]
async fn main() {
    ensure_dirs();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let conn = open_db();
    let pool: DbPool = Arc::new(Mutex::new(conn));
    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(180))
        .build()
        .expect("build reqwest client");
    let state = AppState {
        db: pool,
        http: http.clone(),
    };

    // Background schedule runner — cek tiap 60 detik.
    spawn_schedule_runner(state.clone());

    let app = Router::new()
        .route("/health", get(health))
        // Status
        .route("/api/training/status", get(get_status))
        // Materials
        .route("/api/training/materials", get(list_materials))
        .route("/api/training/materials/upload", post(upload_material))
        .route("/api/training/ingest/{materialId}", post(trigger_ingest))
        // Schedule
        .route(
            "/api/training/schedule",
            get(list_schedule).post(create_schedule),
        )
        .route(
            "/api/training/schedule/{id}",
            patch(patch_schedule).delete(delete_schedule),
        )
        // History
        .route("/api/training/history", get(list_history))
        // Evaluations
        .route(
            "/api/training/evaluations",
            get(list_evaluations).post(create_evaluation),
        )
        .route("/api/training/evaluations/run", post(run_evaluations))
        .route(
            "/api/training/evaluations/{id}",
            delete(delete_evaluation),
        )
        .layer(DefaultBodyLimit::max(MAX_UPLOAD))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3006")
        .await
        .expect("bind 127.0.0.1:3006");
    tracing::info!("[lawu-training] listening on http://127.0.0.1:3006");
    axum::serve(listener, app).await.expect("server error");
}
