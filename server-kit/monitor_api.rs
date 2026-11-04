// ============================================================
// monitor_api.rs — Lawu-2.5-Fine System Monitor API
// ------------------------------------------------------------
// Port :3004  Axum 0.8 + sysinfo + reqwest
//
// Endpoint:
//   GET  /health
//   GET  /api/monitor/metrics   (auth)   CPU/RAM/disk/load + llama tok/s
//   GET  /api/monitor/services   (auth)   systemd service status
//   GET  /api/monitor/health     (auth)   composite health check
//
// Build: cargo build --release -> cp monitor_api lawu-monitor
// ============================================================
#[path = "../auth_middleware.rs"]
mod auth_middleware;

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use auth_middleware::{verify_token_with_db, DbPool};
use rusqlite::Connection;
use serde::Serialize;
use serde_json::{json, Value};
use std::{sync::Arc, time::Duration};
use sysinfo::{Disks, System};
use tokio::sync::Mutex;

const LLAMA_LOG_PATH: &str = "/var/log/lawu/llama.log";

#[derive(Clone)]
struct AppState {
    db: DbPool,
    http: reqwest::Client,
    sys: Arc<Mutex<System>>,
}

// ------------------------------ DTO ------------------------------
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MetricsDto {
    cpu_pct: f32,
    ram_used_gb: f32,
    ram_total_gb: f32,
    ssd_used_gb: f32,
    ssd_total_gb: f32,
    hdd_used_gb: f32,
    hdd_total_gb: f32,
    load_avg: f32,
    llama_tok_per_sec: f32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ServiceDto {
    name: String,
    active: bool,
    sub: String,
    pid: Option<u32>,
    uptime_sec: Option<u64>,
    memory_mb: f32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HealthDto {
    backend: bool,
    corpus: bool,
    auth: bool,
    chat: bool,
    monitor: bool,
    llama: bool,
}

// ------------------------------ DB open ------------------------------
fn open_db() -> Connection {
    { let p = auth_middleware::auth_db_path(); let _ = std::fs::create_dir_all(std::path::Path::new(&p).parent().unwrap_or(std::path::Path::new("/var/lib/lawu"))); }
    let conn = Connection::open(auth_middleware::auth_db_path()).expect("gagal buka auth.db");
    let _ = conn.pragma_update(None, "journal_mode", "WAL");
    let _ = conn.pragma_update(None, "synchronous", "NORMAL");
    let _ = conn.pragma_update(None, "foreign_keys", "ON");
    let sql = include_str!("../../migrations/schema.sql");
    conn.execute_batch(sql).expect("migrate schema");
    conn
}

// ------------------------------ llama tok/sec parser ------------------------------
fn parse_llama_tok_per_sec() -> f32 {
    let s = match std::fs::read_to_string(LLAMA_LOG_PATH) {
        Ok(s) => s,
        Err(_) => return 0.0,
    };
    for line in s.lines().rev() {
        if let Some(idx) = line.find("tok/s") {
            let before = &line[..idx];
            let mut end = before.len();
            while end > 0 {
                let c = before.as_bytes()[end - 1];
                if c.is_ascii_digit() || c == b'.' {
                    end -= 1;
                } else {
                    break;
                }
            }
            let mut start = end;
            while start > 0 {
                let c = before.as_bytes()[start - 1];
                if c.is_ascii_digit() || c == b'.' {
                    start -= 1;
                } else {
                    break;
                }
            }
            if start < end {
                if let Ok(v) = before[start..end].parse::<f32>() {
                    return v;
                }
            }
        }
        if let Some(idx) = line.find("tokens per second") {
            let after = &line[idx + "tokens per second".len()..];
            let after_trim = after.trim_start_matches([':', ' ', '=']);
            let num: String = after_trim
                .chars()
                .take_while(|c| c.is_ascii_digit() || *c == '.')
                .collect();
            if let Ok(v) = num.parse::<f32>() {
                return v;
            }
        }
    }
    0.0
}

// ------------------------------ systemd parser ------------------------------
fn query_systemd(name: &str) -> ServiceDto {
    let out = std::process::Command::new("systemctl")
        .args([
            "show",
            &format!("{name}.service"),
            "--property=ActiveState,SubState,MainPID,ExecMainStartTimestamp,MemoryCurrent",
        ])
        .output();
    let mut active = false;
    let mut sub = String::new();
    let mut pid: Option<u32> = None;
    let mut uptime_sec: Option<u64> = None;
    let mut memory_mb: f32 = 0.0;
    if let Ok(out) = out {
        let s = String::from_utf8_lossy(&out.stdout);
        let mut exec_start_epoch: Option<i64> = None;
        for line in s.lines() {
            if let Some(v) = line.strip_prefix("ActiveState=") {
                active = v.trim() == "active";
            } else if let Some(v) = line.strip_prefix("SubState=") {
                sub = v.trim().to_string();
            } else if let Some(v) = line.strip_prefix("MainPID=") {
                if let Ok(n) = v.trim().parse::<u32>() {
                    if n > 0 {
                        pid = Some(n);
                    }
                }
            } else if let Some(v) = line.strip_prefix("ExecMainStartTimestamp=") {
                let ts = v.trim().to_string();
                if !ts.is_empty() && ts != "n/a" {
                    for fmt in &[
                        "%a %Y-%m-%d %H:%M:%S %Z",
                        "%a %Y-%m-%d %H:%M:%S",
                        "%Y-%m-%d %H:%M:%S %Z",
                    ] {
                        if let Ok(dt) =
                            chrono::NaiveDateTime::parse_from_str(&ts.replace("  ", " "), fmt)
                        {
                            exec_start_epoch = Some(dt.and_utc().timestamp());
                            break;
                        }
                    }
                    if exec_start_epoch.is_none() {
                        let d = std::process::Command::new("date")
                            .args(["-d", &ts, "+%s"])
                            .output();
                        if let Ok(d) = d {
                            if let Ok(s) = String::from_utf8(d.stdout) {
                                if let Ok(n) = s.trim().parse::<i64>() {
                                    exec_start_epoch = Some(n);
                                }
                            }
                        }
                    }
                }
            } else if let Some(v) = line.strip_prefix("MemoryCurrent=") {
                if let Ok(n) = v.trim().parse::<i64>() {
                    if n > 0 {
                        memory_mb = (n as f32) / (1024.0 * 1024.0);
                    }
                }
            }
        }
        if let Some(start) = exec_start_epoch {
            let now = chrono::Utc::now().timestamp();
            if now >= start {
                uptime_sec = Some((now - start) as u64);
            }
        }
    }
    ServiceDto {
        name: name.to_string(),
        active,
        sub,
        pid,
        uptime_sec,
        memory_mb,
    }
}

// ------------------------------ Handlers ------------------------------
async fn health() -> impl IntoResponse {
    Json(json!({
        "status": "ok",
        "service": "lawu-monitor",
        "ts": chrono::Utc::now().to_rfc3339(),
    }))
}

async fn require_auth(
    headers: &HeaderMap,
    st: &AppState,
) -> Result<(), (StatusCode, Json<Value>)> {
    match verify_token_with_db(headers, st.db.clone()).await {
        Ok(_) => Ok(()),
        Err(e) => Err(e),
    }
}

// GET /api/monitor/metrics
async fn metrics(headers: HeaderMap, State(st): State<AppState>) -> impl IntoResponse {
    if let Err(e) = require_auth(&headers, &st).await {
        return e;
    }
    // refresh CPU usage pertama (baseline)
    {
        let mut sys = st.sys.lock().await;
        sys.refresh_cpu_usage();
        sys.refresh_memory();
    }
    // beri jeda agar cpu_usage stabil
    tokio::time::sleep(Duration::from_millis(200)).await;

    let cpu_pct;
    let ram_total;
    let ram_used;
    let mut ssd_used = 0.0_f32;
    let mut ssd_total = 0.0_f32;
    let mut hdd_used = 0.0_f32;
    let mut hdd_total = 0.0_f32;
    {
        let mut sys = st.sys.lock().await;
        sys.refresh_cpu_usage();
        sys.refresh_memory();
        cpu_pct = sys.global_cpu_usage();
        ram_total = sys.total_memory() as f32 / (1024.0 * 1024.0 * 1024.0);
        ram_used = sys.used_memory() as f32 / (1024.0 * 1024.0 * 1024.0);
    }
    // disk: separate type in sysinfo 0.30+
    let disks = Disks::new_with_refreshed_list();
    for disk in disks.list() {
        let mp = disk.mount_point().to_string_lossy().to_string();
        let total = disk.total_space() as f32 / (1024.0 * 1024.0 * 1024.0);
        let used = (disk.total_space() - disk.available_space()) as f32 / (1024.0 * 1024.0 * 1024.0);
        if mp == "/" {
            ssd_used = used;
            ssd_total = total;
        } else if mp == "/mnt/data" {
            hdd_used = used;
            hdd_total = total;
        }
    }

    let load_avg = std::fs::read_to_string("/proc/loadavg")
        .ok()
        .and_then(|s| {
            s.split_whitespace()
                .next()
                .and_then(|x| x.parse::<f32>().ok())
        })
        .unwrap_or(0.0);

    let llama_tok = parse_llama_tok_per_sec();
    let dto = MetricsDto {
        cpu_pct,
        ram_used_gb: ram_used,
        ram_total_gb: ram_total,
        ssd_used_gb: ssd_used,
        ssd_total_gb: ssd_total,
        hdd_used_gb: hdd_used,
        hdd_total_gb: hdd_total,
        load_avg,
        llama_tok_per_sec: llama_tok,
    };
    (StatusCode::OK, Json(json!(dto)))
}

// GET /api/monitor/services
async fn services(headers: HeaderMap, State(st): State<AppState>) -> impl IntoResponse {
    if let Err(e) = require_auth(&headers, &st).await {
        return e;
    }
    let names = vec![
        "lawu-server",
        "lawu-corpus",
        "lawu-auth",
        "lawu-chat",
        "lawu-monitor",
        "lawu-llama",
        "nginx",
    ];
    let svcs = tokio::task::spawn_blocking(move || {
        names.into_iter().map(query_systemd).collect::<Vec<_>>()
    })
    .await
    .unwrap_or_default();
    (StatusCode::OK, Json(json!({ "services": svcs })))
}

// GET /api/monitor/health
async fn composite_health(headers: HeaderMap, State(st): State<AppState>) -> impl IntoResponse {
    if let Err(e) = require_auth(&headers, &st).await {
        return e;
    }
    let http = st.http.clone();
    let probe = move |url: &'static str| {
        let h = http.clone();
        async move {
            match h.get(url).timeout(Duration::from_secs(2)).send().await {
                Ok(r) => r.status().is_success(),
                Err(_) => false,
            }
        }
    };
    let backend = probe("http://127.0.0.1:3001/health").await;
    let corpus = probe("http://127.0.0.1:3002/health").await;
    let auth = probe("http://127.0.0.1:3003/health").await;
    let chat = probe("http://127.0.0.1:3005/health").await;
    let monitor = probe("http://127.0.0.1:3004/health").await;
    let llama = probe("http://127.0.0.1:8081/health").await; // lawu-llama port (umum)
    let dto = HealthDto {
        backend,
        corpus,
        auth,
        chat,
        monitor,
        llama,
    };
    (StatusCode::OK, Json(json!(dto)))
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
        .timeout(Duration::from_secs(5))
        .build()
        .expect("build reqwest client");
    let sys = Arc::new(Mutex::new(System::new_all()));
    let state = AppState { db: pool, http, sys };

    let app = Router::new()
        .route("/health", get(health))
        .route("/api/monitor/metrics", get(metrics))
        .route("/api/monitor/services", get(services))
        .route("/api/monitor/health", get(composite_health))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3004")
        .await
        .expect("bind 127.0.0.1:3004");
    tracing::info!("[lawu-monitor] listening on http://127.0.0.1:3004");
    axum::serve(listener, app).await.expect("server error");
}
