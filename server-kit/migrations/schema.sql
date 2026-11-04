-- ============================================================
-- Lawu-2.5-Fine auth.db schema (v3.6.0)
-- Apply via: sqlite3 /var/lib/lawu/auth.db < migrations/schema.sql
-- Or auto-applied at server startup (auth_api.rs runs migration).
-- ============================================================

PRAGMA journal_mode=WAL;
PRAGMA synchronous=NORMAL;
PRAGMA foreign_keys=ON;

CREATE TABLE IF NOT EXISTS users (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  email TEXT UNIQUE NOT NULL,
  password_hash TEXT NOT NULL,
  role TEXT NOT NULL DEFAULT 'guest' CHECK(role IN ('admin','guest')),
  created_at TEXT NOT NULL,
  last_login_at TEXT
);

CREATE TABLE IF NOT EXISTS api_keys (
  id TEXT PRIMARY KEY,
  user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  name TEXT NOT NULL,
  key_hash TEXT UNIQUE NOT NULL,
  key_prefix TEXT NOT NULL,
  scopes TEXT NOT NULL DEFAULT 'chat,search',
  created_at TEXT NOT NULL,
  last_used_at TEXT,
  revoked_at TEXT
);

CREATE TABLE IF NOT EXISTS chat_sessions (
  id TEXT PRIMARY KEY,
  user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  title TEXT NOT NULL DEFAULT 'Obrolan Baru',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  message_count INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS chat_messages (
  id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL REFERENCES chat_sessions(id) ON DELETE CASCADE,
  role TEXT NOT NULL CHECK(role IN ('user','assistant')),
  content TEXT NOT NULL,
  latency_ms INTEGER,
  created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_chat_sessions_user ON chat_sessions(user_id, updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_chat_messages_session ON chat_messages(session_id, created_at);
CREATE INDEX IF NOT EXISTS idx_api_keys_user ON api_keys(user_id, created_at DESC);

-- ============================================================
-- Lawu-2.5-Fine v3.7.0 — Training Management tables
-- (training_api.rs :3006, share auth.db)
-- "Training" = RAG augmentation (Paten 3: NO GPU, NO fine-tuning bobot).
-- ============================================================

CREATE TABLE IF NOT EXISTS training_materials (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  title TEXT NOT NULL,
  source TEXT NOT NULL,                       -- 'wikipedia' | 'ebook' | 'web_crawl' | 'skill' | 'custom_upload'
  category TEXT NOT NULL DEFAULT 'umum',      -- 'teknologi' | 'keamanan' | 'ai_ml' | 'siber' | 'indonesia' | 'custom'
  priority TEXT NOT NULL DEFAULT 'sedang',    -- 'tinggi' | 'sedang' | 'rendah'
  doc_count INTEGER NOT NULL DEFAULT 0,
  file_path TEXT,                             -- path ke file materi (kalau upload)
  status TEXT NOT NULL DEFAULT 'queued'       -- 'queued' | 'ingesting' | 'indexed' | 'failed'
    CHECK(status IN ('queued','ingesting','indexed','failed')),
  created_at TEXT NOT NULL,
  indexed_at TEXT
);

CREATE TABLE IF NOT EXISTS training_schedule (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT NOT NULL,
  cron_expr TEXT NOT NULL,                    -- '0 3 * * 1' = setiap Senin 03:00
  action TEXT NOT NULL,                       -- 'crawl_web' | 'extract_ebooks' | 'ingest_skill' | 'custom'
  enabled INTEGER NOT NULL DEFAULT 1,
  last_run_at TEXT,
  next_run_at TEXT,
  created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS training_history (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  started_at TEXT NOT NULL,
  finished_at TEXT,
  action TEXT NOT NULL,                       -- 'crawl' | 'extract' | 'ingest' | 'evaluate'
  material_id INTEGER REFERENCES training_materials(id) ON DELETE SET NULL,
  doc_count INTEGER NOT NULL DEFAULT 0,
  status TEXT NOT NULL DEFAULT 'running'      -- 'running' | 'success' | 'failed' | 'partial'
    CHECK(status IN ('running','success','failed','partial')),
  log TEXT,                                   -- multiline log
  duration_sec INTEGER
);

CREATE TABLE IF NOT EXISTS evaluation_qa (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  question TEXT NOT NULL,
  expected_answer TEXT NOT NULL,
  category TEXT,
  created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS evaluation_results (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  qa_id INTEGER REFERENCES evaluation_qa(id) ON DELETE CASCADE,
  actual_answer TEXT,
  similarity_score REAL,                      -- 0.0 - 1.0
  latency_ms INTEGER,
  run_at TEXT NOT NULL,
  status TEXT NOT NULL                        -- 'excellent' | 'good' | 'needs_improvement'
    CHECK(status IN ('excellent','good','needs_improvement'))
);

CREATE INDEX IF NOT EXISTS idx_training_materials_status ON training_materials(status, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_training_schedule_enabled ON training_schedule(enabled);
CREATE INDEX IF NOT EXISTS idx_training_history_started ON training_history(started_at DESC);
CREATE INDEX IF NOT EXISTS idx_evaluation_results_qa ON evaluation_results(qa_id, run_at DESC);
CREATE INDEX IF NOT EXISTS idx_evaluation_results_run ON evaluation_results(run_at DESC);
