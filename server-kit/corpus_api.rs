// ============================================================
// corpus_api.rs — Lawu-2.5-Fine Corpus & Ebook API (port 3002)
// ------------------------------------------------------------
// Binary TERPISAH dari main.rs produksi. main.rs tidak disentuh.
// Fitur (sesuai permintaan user):
//   - Tambah / list / hapus URL web korpus (menu "Add Web Korpus")
//   - Trigger crawl web korpus (web_ingest.py via venv)
//   - List ebook di /mnt/data/ebooks
//   - Upload ebook via UI (multipart) ke /mnt/data/ebooks
//   - Trigger extract ebook (extract_ebooks.py via venv)
//   - Status job crawl/extract
//
// Build  : cargo build --release   (di ~/lawu-server)
// Deploy : systemd lawu-corpus.service + nginx location /api/corpus, /api/ebooks
// ============================================================
use axum::{
    extract::{
        DefaultBodyLimit, Multipart, Query, State,
    },
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    process::Stdio,
};
use tokio::process::Command;
use tokio::sync::Mutex;

// ------------------------------ Konfigurasi ------------------------------
const EBOOKS_DIR: &str = "/mnt/data/ebooks";
const PROCESSED_DIR: &str = "/mnt/data/processed_markdowns";
const STATE_DIR: &str = "/var/lib/lawu";
const LOG_DIR: &str = "/var/log/lawu";
const CORPUS_LST: &str = "/etc/lawu/web-corpus.lst";
const CUSTOM_JSONL: &str = "/var/lib/lawu/web_corpus_custom.jsonl";
const CRAWL_STATE: &str = "/var/lib/lawu/web_corpus_state.json";
const JOBS_FILE: &str = "/var/lib/lawu/jobs.json";
const VENV_PY: &str = "/opt/lawu/venv/bin/python3";
const SCRIPTS_DIR: &str = "/opt/lawu/scripts";
const LAWU_INGEST: &str = "/home/lawu/lawu-server/target/release/lawu-ingest";
const SKILL_JSONL: &str = "/opt/lawu/korpus-skill/lawu-skill.jsonl";
const MAX_UPLOAD: usize = 500 * 1024 * 1024; // 500 MB

#[derive(Clone)]
struct AppState {
    crawl_lock: std::sync::Arc<Mutex<()>>,
}

// ------------------------------ DTO (camelCase) --------------------------
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CorpusEntryDto {
    id: String,
    url: String,
    category: String,
    source: String,          // "seed" (resmi) | "custom" (via UI)
    added_at: Option<String>,
    crawled_at: Option<String>,
    page_title: Option<String>,
    pages_crawled: Option<u64>,
    error_count: Option<u64>,
}

#[derive(Deserialize)]
struct AddUrlReq {
    url: String,
    #[serde(default = "default_category")]
    category: String,
}
fn default_category() -> String {
    "custom".into()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EbookDto {
    name: String,
    size_bytes: u64,
    modified_at: String,
    kind: String,            // pdf | epub | docx | txt
    indexed: bool,           // sudah masuk ebooks.jsonl?
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StatusDto {
    kind: String,            // crawl | extract
    running: bool,
    pid: Option<u32>,
    started_at: Option<String>,
    finished_at: Option<String>,
    last_result: serde_json::Value,
}

// ------------------------------ Helpers ----------------------------------
fn ensure_dirs() {
    for d in [EBOOKS_DIR, PROCESSED_DIR, STATE_DIR, LOG_DIR] {
        let _ = fs::create_dir_all(d);
    }
}

fn read_custom_entries() -> Vec<serde_json::Value> {
    let mut out = Vec::new();
    if let Ok(s) = fs::read_to_string(CUSTOM_JSONL) {
        for line in s.lines() {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(line.trim()) {
                out.push(v);
            }
        }
    }
    out
}

fn append_custom_entry(url: &str, category: &str) -> serde_json::Value {
    let existing = read_custom_entries();
    let dup = existing.iter().any(|v| v.get("url").and_then(|u| u.as_str()) == url);
    if dup {
        return json!({ "duplicate": true });
    }
    let id = format!("c-{}", chrono::Utc::now().timestamp_millis());
    let entry = json!({
        "id": id,
        "url": url,
        "category": category,
        "added_at": chrono::Utc::now().to_rfc3339(),
    });
    use std::io::Write;
    let mut f = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(CUSTOM_JSONL)
        .expect("buka custom jsonl");
    writeln!(f, "{}", entry).expect("tulis custom jsonl");
    entry
}

fn delete_custom_entry(url: &str) -> usize {
    let entries = read_custom_entries();
    let kept: Vec<String> = entries
        .iter()
        .filter(|v| v.get("url").and_then(|u| u.as_str()) != Some(url))
        .map(|v| v.to_string())
        .collect();
    let removed = entries.len() - kept.len();
    if removed > 0 {
        let _ = fs::write(CUSTOM_JSONL, kept.join("\n") + (if kept.is_empty() { "" } else { "\n" }));
    }
    removed
}

fn read_json(path: &str) -> serde_json::Value {
    fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(json!({}))
}

fn pid_alive(pid: u32) -> bool {
    Path::new(&format!("/proc/{pid}")).exists()
}

// seed resmi dari /etc/lawu/web-corpus.lst  (kategori<TAB>url)
fn read_seed_entries() -> Vec<(String, String)> {
    let mut out = Vec::new();
    if let Ok(s) = fs::read_to_string(CORPUS_LST) {
        for line in s.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut it = line.splitn(2, char::is_whitespace);
            if let (Some(cat), Some(url)) = (it.next(), it.next()) {
                let url = url.trim();
                if !url.is_empty() {
                    out.push((cat.trim().to_string(), url.to_string()));
                }
            }
        }
    }
    out
}

fn state_urls() -> HashMap<String, serde_json::Value> {
    let v = read_json(CRAWL_STATE);
    let mut map = HashMap::new();
    if let Some(obj) = v.as_object() {
        for (k, val) in obj {
            if k == "_last_run" || k == "_force" {
                continue;
            }
            map.insert(k.clone(), val.clone());
        }
    }
    map
}

fn job_active(kind: &str) -> Option<(u32, String)> {
    let jobs = read_json(JOBS_FILE);
    let j = jobs.get(kind)?;
    let pid = j.get("pid")?.as_u64()? as u32;
    let started = j
        .get("started_at")
        .and_then(|s| s.as_str())
        .unwrap_or("")
        .to_string();
    if pid_alive(pid) {
        Some((pid, started))
    } else {
        None
    }
}

async fn spawn_job(kind: &str, script: &str, extra_args: &[&str]) -> Result<u32, String> {
    let _ = fs::create_dir_all(STATE_DIR);
    if let Some((pid, started)) = job_active(kind) {
        return Err(format!("Job {kind} masih berjalan (pid {pid}, mulai {started})"));
    }
    let mut cmd = Command::new(VENV_PY);
    cmd.arg(format!("{SCRIPTS_DIR}/{script}"));
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(false);
    let child = cmd
        .spawn()
        .map_err(|e| format!("Gagal spawn {script}: {e}"))?;
    let pid = child.id().unwrap_or(0);
    let started = chrono::Utc::now().to_rfc3339();
    let mut jobs = read_json(JOBS_FILE);
    jobs[kind] = json!({ "pid": pid, "started_at": started, "status": "running" });
    let _ = fs::write(JOBS_FILE, serde_json::to_string_pretty(&jobs).unwrap());
    Ok(pid)
}

fn sanitized_filename(raw: &str) -> String {
    let path = Path::new(raw);
    let base = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("ebook");
    let mut s: String = base
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '.' || c == '-' || c == '_' || c == ' ' {
                c
            } else {
                '_'
            }
        })
        .collect();
    s = s.trim().to_string();
    if s.len() > 120 {
        s = s[..120].to_string();
    }
    format!("{}-{}", chrono::Utc::now().format("%Y%m%d%H%M%S"), s)
}

// ------------------------------ Handlers ---------------------------------
async fn health() -> impl IntoResponse {
    Json(json!({ "status": "ok", "service": "lawu-corpus", "ts": chrono::Utc::now().to_rfc3339() }))
}

// GET /api/corpus/web
async fn list_web_corpus() -> impl IntoResponse {
    ensure_dirs();
    let state = state_urls();
    let mut out: Vec<CorpusEntryDto> = Vec::new();

    for (i, (cat, url)) in read_seed_entries().into_iter().enumerate() {
        let st = state.get(&url);
        out.push(CorpusEntryDto {
            id: format!("seed-{i}"),
            url,
            category: cat,
            source: "seed".into(),
            added_at: None,
            crawled_at: st.and_then(|v| v.get("crawled_at")).and_then(|x| x.as_str()).map(String::from),
            page_title: st.and_then(|v| v.get("title")).and_then(|x| x.as_str()).map(String::from),
            pages_crawled: st.and_then(|v| v.get("chars")).and_then(|x| x.as_u64()),
            error_count: None,
        });
    }
    for v in read_custom_entries() {
        let url = v.get("url").and_then(|u| u.as_str()).unwrap_or("").to_string();
        let st = state.get(&url);
        out.push(CorpusEntryDto {
            id: v.get("id").and_then(|x| x.as_str()).unwrap_or("custom").to_string(),
            url,
            category: v.get("category").and_then(|x| x.as_str()).unwrap_or("custom").to_string(),
            source: "custom".into(),
            added_at: v.get("added_at").and_then(|x| x.as_str()).map(String::from),
            crawled_at: st.and_then(|s| s.get("crawled_at")).and_then(|x| x.as_str()).map(String::from),
            page_title: st.and_then(|s| s.get("title")).and_then(|x| x.as_str()).map(String::from),
            pages_crawled: st.and_then(|s| s.get("chars")).and_then(|x| x.as_u64()),
            error_count: None,
        });
    }
    Json(json!({
        "total": out.len(),
        "entries": out,
    }))
}

// POST /api/corpus/web  {url, category}
async fn add_web_corpus(State(_st): State<AppState>, Json(req): Json<AddUrlReq>) -> impl IntoResponse {
    let url = req.url.trim().to_string();
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "URL harus diawali http:// atau https://" })),
        );
    }
    let entry = append_custom_entry(&url, &req.category.trim());
    let status = if entry.get("duplicate").is_some() {
        StatusCode::CONFLICT
    } else {
        StatusCode::CREATED
    };
    (status, Json(entry))
}

// DELETE /api/corpus/web?url=...
async fn delete_web_corpus(Query(q): Query<HashMap<String, String>>) -> impl IntoResponse {
    match q.get("url") {
        Some(url) => {
            let removed = delete_custom_entry(url);
            if removed > 0 {
                (StatusCode::OK, Json(json!({ "removed": removed })))
            } else {
                (
                    StatusCode::NOT_FOUND,
                    Json(json!({ "error": "URL tidak ditemukan di daftar custom (seed resmi tidak bisa dihapus via API)" })),
                )
            }
        }
        None => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "parameter ?url= wajib" })),
        ),
    }
}

// POST /api/corpus/web/crawl   (?force=true untuk re-crawl paksa)
async fn crawl_web(
    State(st): State<AppState>,
    Query(q): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let _guard = st.crawl_lock.lock().await;
    let force = matches!(q.get("force").map(|s| s.as_str()), Some("true") | Some("1"));
    let args: Vec<&str> = if force { vec!["--force"] } else { vec![] };
    match spawn_job("crawl", "web_ingest.py", &args).await {
        Ok(pid) => (
            StatusCode::ACCEPTED,
            Json(json!({ "started": true, "kind": "crawl", "pid": pid,
                         "force": force, "ts": chrono::Utc::now().to_rfc3339() })),
        ),
        Err(e) => (StatusCode::CONFLICT, Json(json!({ "started": false, "error": e }))),
    }
}

// GET /api/ebooks
async fn list_ebooks() -> impl IntoResponse {
    ensure_dirs();
    let mut out: Vec<EbookDto> = Vec::new();
    // set docid per source_file dari ebooks.jsonl (hasil extract)
    let mut indexed_files: std::collections::HashSet<String> = Default::default();
    let jsonl = format!("{PROCESSED_DIR}/ebooks.jsonl");
    if let Ok(s) = fs::read_to_string(&jsonl) {
        for line in s.lines() {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
                if let Some(sf) = v.get("source_file").and_then(|x| x.as_str()) {
                    indexed_files.insert(sf.to_string());
                }
            }
        }
    }
    if let Ok(rd) = fs::read_dir(EBOOKS_DIR) {
        for e in rd.flatten() {
            let p: PathBuf = e.path();
            let name = e.file_name().to_string_lossy().to_string();
            if name.starts_with('.') {
                continue;
            }
            let kind = match p.extension().and_then(|x| x.to_str()).map(|x| x.to_ascii_lowercase()) {
                Some(ext) => match ext.as_str() {
                    "pdf" => "pdf",
                    "epub" => "epub",
                    "docx" => "docx",
                    "txt" => "txt",
                    _ => continue,
                }
                .to_string(),
                None => continue,
            };
            let meta = match fs::metadata(&p) {
                Ok(m) => m,
                Err(_) => continue,
            };
            if !meta.is_file() {
                continue;
            }
            let modified = meta
                .modified()
                .ok()
                .and_then(|t| {
                    t.duration_since(std::time::UNIX_EPOCH)
                        .ok()
                        .map(|d| chrono::DateTime::<chrono::Utc>::from_timestamp(d.as_secs() as i64, 0))
                })
                .flatten()
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_default();
            out.push(EbookDto {
                name,
                size_bytes: meta.len(),
                modified_at: modified,
                kind,
                indexed: indexed_files.contains(&name),
            });
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    let indexed_count = out.iter().filter(|e| e.indexed).count();
    Json(json!({
        "total": out.len(),
        "indexed": indexed_count,
        "ebooks": out,
    }))
}

// POST /api/ebooks/upload  (multipart, field name "file")
async fn upload_ebook(mut multipart: Multipart) -> impl IntoResponse {
    ensure_dirs();
    let mut saved: Vec<serde_json::Value> = Vec::new();
    while let Ok(Some(field)) = multipart.next_field().await {
        if field.name() != Some("file") {
            continue;
        }
        let raw_name = field.file_name().unwrap_or("ebook").to_string();
        let safe = sanitized_filename(&raw_name);
        let dest = format!("{EBOOKS_DIR}/{safe}");
        let data = match field.bytes().await {
            Ok(d) => d,
            Err(e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "saved": false, "error": format!("gagal baca file: {e}") })),
                )
            }
        };
        if data.len() > MAX_UPLOAD {
            return (
                StatusCode::PAYLOAD_TOO_LARGE,
                Json(json!({ "saved": false, "error": "file terlalu besar (maks 500MB)" })),
            );
        }
        match fs::write(&dest, &data) {
            Ok(_) => saved.push(json!({
                "name": safe,
                "sizeBytes": data.len(),
                "path": dest,
            })),
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "saved": false, "error": format!("gagal simpan: {e}") })),
                )
            }
        }
    }
    if saved.is_empty() {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({ "saved": false, "error": "tidak ada field 'file' ditemukan" })),
        )
    } else {
        (StatusCode::CREATED, Json(json!({ "saved": true, "files": saved })))
    }
}

// POST /api/ebooks/extract
async fn extract_ebooks(State(st): State<AppState>) -> impl IntoResponse {
    let _guard = st.crawl_lock.lock().await;
    match spawn_job("extract", "extract_ebooks.py", &[]).await {
        Ok(pid) => (
            StatusCode::ACCEPTED,
            Json(json!({ "started": true, "kind": "extract", "pid": pid,
                         "ts": chrono::Utc::now().to_rfc3339() })),
        ),
        Err(e) => (StatusCode::CONFLICT, Json(json!({ "started": false, "error": e }))),
    }
}

// POST /api/corpus/ingest  {kind: "web"|"ebooks"|"skill"}
// Spawn lawu-ingest CLI -> masukkan JSONL ke Tantivy (dedup docid).
// Single-writer: terlindungi crawl_lock + job_active("ingest").
#[derive(Deserialize)]
struct IngestReq {
    kind: String,
}
async fn ingest_corpus(State(st): State<AppState>, Json(req): Json<IngestReq>) -> impl IntoResponse {
    let file = match req.kind.as_str() {
        "web" => format!("{PROCESSED_DIR}/web_corpus.jsonl"),
        "ebooks" => format!("{PROCESSED_DIR}/ebooks.jsonl"),
        "skill" => SKILL_JSONL.to_string(),
        other => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "started": false, "error": format!("kind tidak dikenal: {other} (pakai web|ebooks|skill)") })),
            )
        }
    };
    if !Path::new(&file).exists() {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "started": false, "error": format!("file belum ada: {file} — jalankan crawl/extract dulu") })),
        );
    }
    let _guard = st.crawl_lock.lock().await;
    if let Some((pid, started)) = job_active("ingest") {
        return (
            StatusCode::CONFLICT,
            Json(json!({ "started": false, "error": format!("Job ingest masih berjalan (pid {pid}, mulai {started})") })),
        );
    }
    let mut cmd = Command::new(LAWU_INGEST);
    cmd.arg("--file").arg(&file).stdout(Stdio::null()).stderr(Stdio::null());
    let child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "started": false, "error": format!("gagal spawn lawu-ingest: {e}") })),
            )
        }
    };
    let pid = child.id().unwrap_or(0);
    let started = chrono::Utc::now().to_rfc3339();
    let mut jobs = read_json(JOBS_FILE);
    jobs["ingest"] = json!({ "pid": pid, "started_at": started, "status": "running", "file": file });
    let _ = fs::write(JOBS_FILE, serde_json::to_string_pretty(&jobs).unwrap());
    (
        StatusCode::ACCEPTED,
        Json(json!({ "started": true, "kind": "ingest", "pid": pid, "file": file,
                     "hint": "setelah selesai, restart lawu-server bila hasil belum muncul di pencarian" })),
    )
}

// GET /api/corpus/status
async fn status() -> impl IntoResponse {
    let jobs = read_json(JOBS_FILE);
    let crawl = jobs.get("crawl").cloned().unwrap_or(json!({}));
    let extract = jobs.get("extract").cloned().unwrap_or(json!({}));
    let ingest = jobs.get("ingest").cloned().unwrap_or(json!({}));
    let last_crawl = read_json(CRAWL_STATE).get("_last_run").cloned().unwrap_or(json!({}));
    Json(json!({
        "crawl": {
            "running": crawl.get("pid").and_then(|p| p.as_u64()).map(pid_alive).unwrap_or(false),
            "pid": crawl.get("pid"),
            "startedAt": crawl.get("started_at"),
            "lastResult": last_crawl,
        },
        "extract": {
            "running": extract.get("pid").and_then(|p| p.as_u64()).map(pid_alive).unwrap_or(false),
            "pid": extract.get("pid"),
            "startedAt": extract.get("started_at"),
        },
        "ingest": {
            "running": ingest.get("pid").and_then(|p| p.as_u64()).map(pid_alive).unwrap_or(false),
            "pid": ingest.get("pid"),
            "startedAt": ingest.get("started_at"),
            "file": ingest.get("file"),
        },
    }))
}

// ------------------------------ main -------------------------------------
#[tokio::main]
async fn main() {
    ensure_dirs();
    let state = AppState {
        crawl_lock: std::sync::Arc::new(Mutex::new(())),
    };

    let app = Router::new()
        .route("/health", get(health))
        // ---- Web Korpus (menu "Add Web Korpus" di UI) ----
        .route(
            "/api/corpus/web",
            get(list_web_corpus).post(add_web_corpus).delete(delete_web_corpus),
        )
        .route("/api/corpus/web/crawl", post(crawl_web))
        .route("/api/corpus/status", get(status))
        // ---- Ebook (menu "Add Ebook" di UI) ----
        .route("/api/ebooks", get(list_ebooks))
        .route("/api/ebooks/upload", post(upload_ebook))
        .route("/api/ebooks/extract", post(extract_ebooks))
        // ---- Ingest JSONL -> Tantivy (tombol UI "Ingest ke Index") ----
        .route("/api/corpus/ingest", post(ingest_corpus))
        .layer(DefaultBodyLimit::max(MAX_UPLOAD))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3002")
        .await
        .expect("bind 127.0.0.1:3002");
    println!("[lawu-corpus] listening on http://127.0.0.1:3002");
    axum::serve(listener, app).await.expect("server error");
}
