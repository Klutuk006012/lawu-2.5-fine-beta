# Lawu-2.5-Fine — Corpus Kit (Web Korpus + Ebook UI)

Semua sumber deploy ada di **satu repo GitHub**: `Klutuk006012/lawu-2.5-fine-beta` (public).
Deploy di srv-fine = **git clone + 1 script**. Tanpa npm/node di server (dist sudah di-commit).

> ⚠️ **WAJIB dijalankan sebagai user `lawu` — BUKAN root.** Script menyentuh `$HOME/lawu-server`
> dan menjalankan `cargo build`. Sebagai root: cari `/root/lawu-server` (tidak ada) dan hasil
> build jadi milik root. Gunakan pola `su - lawu -c '...'` di bawah.
>
> ℹ️ **v3.5.1**: `corpus_api.rs` sudah diverifikasi compile + smoke test di sandbox
> (cargo 1.99.1, 8 endpoint hijau). File besar (`lawu-ebooks.tar.gz` 244MB + PDF ebook) tidak
> ada di repo — kirim langsung ke srv-fine (scp/rsync).
>
> 🆕 **v3.6.0**: 3 binary Rust baru (lawu-auth :3003, lawu-chat :3005, lawu-monitor :3004) +
> extend corpus_api :3002 dengan `/api/ebooks/search`. SQLite `/var/lib/lawu/auth.db`
> (rusqlite bundled — no system sqlite3 dep). Default login `dev@lawu.ai.id` / `12345678`.
> Lihat section "v3.6.0 Changes" di bawah.

## v3.6.0 Changes

### Binary baru (3) + extend existing (1)

| Binary | Port | Source | Fungsi |
|---|---|---|---|
| `lawu-auth` | 3003 | `src/bin/auth_api.rs` (BARU) | login/logout/me + users CRUD + api-keys CRUD |
| `lawu-monitor` | 3004 | `src/bin/monitor_api.rs` (BARU) | system metrics (sysinfo) + service status + composite health |
| `lawu-chat` | 3005 | `src/bin/chat_api.rs` (BARU) | conversations CRUD + /api/chat (proxy ke :3001 + SQLite persistence) |
| `lawu-corpus` | 3002 | `src/bin/corpus_api.rs` (EXTEND) | + `/api/ebooks/search?q=...` + `/api/ebooks/:docid` (filter docid prefix `ebook-`) |

> `lawu-server` (:3001, main.rs) **TIDAK diubah** — zero-risk. Paten 1 (Rust satu-satunya
> backend) tetap terpenuhi: semua binary baru = Rust Axum.

### Crate baru di Cargo.toml

`rusqlite` 0.32 (**bundled** — sqlite3 static link, no system `apt install sqlite3` needed),
`jsonwebtoken` 9, `argon2` 0.5, `rand` 0.8, `uuid` 1, `sha2` 0.10, `hex` 0.4, `sysinfo` 0.32,
`reqwest` 0.12, `tower-http` 0.6.

### Storage baru

- `/var/lib/lawu/auth.db` — SQLite WAL mode, 4 tabel: `users`, `api_keys`, `chat_sessions`, `chat_messages`.
  Auto-migrate saat `lawu-auth.service` first start (file milik user `lawu`, bukan root).
- `/var/lib/lawu/jwt_secret` — 32 random bytes (hex), auto-generate saat first run.

### systemd unit baru (3) — wajib enable

```bash
# Sebagai root di srv-fine:
mkdir -p /var/lib/lawu && chown lawu:lawu /var/lib/lawu
cp server-kit/systemd/lawu-auth.service    /etc/systemd/system/
cp server-kit/systemd/lawu-chat.service    /etc/systemd/system/
cp server-kit/systemd/lawu-monitor.service /etc/systemd/system/
systemctl daemon-reload
systemctl enable --now lawu-auth lawu-chat lawu-monitor
# First start auto-create /var/lib/lawu/auth.db + seed default user + generate jwt_secret
systemctl status lawu-auth lawu-chat lawu-monitor --no-pager
```

### Nginx route tambahan (snippet `/etc/nginx/snippets/lawu-v360.conf`)

| Route | → Port | Catatan |
|---|---|---|
| `/api/auth/*` | 3003 | login/logout/me |
| `/api/users`, `/api/users/:id` | 3003 | admin only |
| `/api/api-keys`, `/api/api-keys/:id` | 3003 | admin only |
| `/api/conversations` | 3005 | multi-session CRUD |
| `/api/chat` | 3005 | `proxy_read_timeout 120s` — LLM bisa lama |
| `/api/monitor/*` | 3004 | metrics/services/health |
| `/api/ebooks/search` | 3002 | filter docid prefix `ebook-` |

Include-kan di server block utama via single-line `sed` (anti-mangle — lihat Pelajaran #19
di PRD). `nginx -t && systemctl reload nginx`.

### ⚠️ Default login — HARUS ganti password setelah first login!

```
Email    : dev@lawu.ai.id
Password : 12345678
Role     : admin
```

Seed otomatis saat `lawu-auth.service` first start (password di-hash argon2).
**WAJIB ganti password setelah first login** via:
- UI: Dashboard → User Management → edit `dev@lawu.ai.id` → set password baru
- API: `PATCH /api/users/1 -d '{"password":"..."}'` (admin JWT)

Atau buat admin baru + revoke default user. Jangan biarkan `dev@lawu.ai.id`/`12345678`
aktif saat go-live (Langkah 20 checklist).

## Isi Repo (yang relevan untuk deploy)

| Path | Fungsi |
|---|---|
| `deploy-from-github.sh` | **Script deploy utama** — dijalankan di srv-fine |
| `deploy/dashboard-dist/` | Dashboard Vue v3.6.0 hasil build final (emerald + markdown + login + users + api-keys) |
| `server-kit/corpus_api.rs` | Binary Axum **:3002** — API korpus & ebook + `/api/ebooks/search` (src/bin/, main.rs tidak disentuh) |
| `server-kit/auth_api.rs` | Binary Axum **:3003** (v3.6.0 BARU) — login/users CRUD/api-keys CRUD |
| `server-kit/chat_api.rs` | Binary Axum **:3005** (v3.6.0 BARU) — conversations CRUD + /api/chat (proxy ke :3001) |
| `server-kit/monitor_api.rs` | Binary Axum **:3004** (v3.6.0 BARU) — metrics (sysinfo) + services + health |
| `server-kit/ingest_jsonl.rs` | Binary `lawu-ingest` — JSONL → Tantivy (dedup docid) |
| `server-kit/web_ingest.py` | Crawler web korpus (BFS, robots.txt, 1 JSONL) |
| `server-kit/extract_ebooks.py` | Extract PDF/EPUB/DOCX/TXT → ebooks.jsonl |
| `server-kit/web-corpus.lst` | 32+ URL seed resmi user, terkategori |
| `server-kit/korpus-skill/` | Materi **Advanced Google Search** (skill wajib Lawu Fine) |
| `server-kit/lawu-corpus.service` | systemd unit untuk :3002 |
| `server-kit/systemd/lawu-auth.service` | systemd unit untuk :3003 (v3.6.0 BARU) |
| `server-kit/systemd/lawu-chat.service` | systemd unit untuk :3005 (v3.6.0 BARU) |
| `server-kit/systemd/lawu-monitor.service` | systemd unit untuk :3004 (v3.6.0 BARU) |
| `server-kit/schema.sql` | SQLite schema (users/api_keys/chat_sessions/chat_messages) — auto-migrate, ada untuk backup |

## Deploy di srv-fine (3 perintah)

```bash
# sebagai root di srv-fine — satu baris:
rm -rf /tmp/lawu-deploy && su - lawu -c 'git clone --depth 1 https://github.com/Klutuk006012/lawu-2.5-fine-beta.git /tmp/lawu-deploy && cd /tmp/lawu-deploy && bash deploy-from-github.sh'
```

- **Bagian A (otomatis, user lawu)**: patch Cargo.toml → `cargo build --release` →
  salin script python → konversi korpus skill.
- **Bagian B (cetak blok perintah root)**: `su -` → tempel sekali jalan →
  web-corpus.lst, systemd `lawu-corpus.service`, dashboard baru ke
  `/var/www/lawu-dashboard`, nginx snippet + reload, ingest skill, test API.

> Repo sudah pernah clone? `cd /tmp/lawu-deploy && git pull && bash deploy-from-github.sh`

## Pemakaian Pertama via UI

1. **Web Korpus** → seed resmi sudah terisi dari `/etc/lawu/web-corpus.lst` → **Mulai Crawl**
2. **Ebook & Korpus** → upload/extract `lawu-ebooks.tar.gz` (30 ebook) → **Ingest ke Index**
3. Balik ke **Web Korpus** → **Ingest Web ke Index** + **Ingest Skill**
4. `systemctl restart lawu-server` (via `su -`) bila pencarian belum menampilkan konten baru
5. Uji: "Vue.js 3 design pattern", "Onno Purbo UU PDP", "advanced google search", "mikrotik firewall"

## API Baru (via nginx)

| Endpoint | Fungsi |
|---|---|
| `GET  /api/corpus/web` | Daftar sumber (seed + custom + status crawl) |
| `POST /api/corpus/web` | **Tambah URL** `{url, category}` |
| `DELETE /api/corpus/web?url=` | Hapus URL custom |
| `POST /api/corpus/web/crawl?force=true` | Crawler (background job) |
| `POST /api/corpus/ingest` `{kind: web|ebooks|skill}` | JSONL → Tantivy |
| `GET  /api/ebooks` | Daftar ebook `/mnt/data/ebooks` |
| `POST /api/ebooks/upload` | **Unggah ebook** (multipart, field `file`) |
| `POST /api/ebooks/extract` | Extract semua ebook (background job) |
| `GET  /api/corpus/status` | Status job crawl/extract/ingest |

## API v3.6.0 (Auth + Chat + Monitor + Ebook Search)

Semua endpoint v3.6.0 butuh `Authorization: Bearer <token>` — JWT (`eyJ...`) untuk user
login, atau `lawu_<32hex>` untuk API key (machine-to-machine, misal Opencode).

| Endpoint | Port | Fungsi |
|---|---|---|
| `POST /api/auth/login` | 3003 | Login → `{token, user}` (default `dev@lawu.ai.id`/`12345678`) |
| `POST /api/auth/logout` | 3003 | Logout (revoke JWT) |
| `GET  /api/auth/me` | 3003 | Profil user dari token |
| `GET  /api/users` | 3003 | List users (admin only) |
| `POST /api/users` | 3003 | Buat user (admin only) |
| `PATCH /api/users/:id` | 3003 | Edit user/password/role (admin only) |
| `DELETE /api/users/:id` | 3003 | Hapus user (admin only) |
| `GET  /api/api-keys` | 3003 | List API keys (admin only) |
| `POST /api/api-keys` | 3003 | Generate key baru → `{key: "lawu_..."}` (sekali tampil, hash SHA-256 stored) |
| `DELETE /api/api-keys/:id` | 3003 | Revoke key (admin only) |
| `GET  /api/conversations` | 3005 | List chat sessions (multi-session seperti chat.z.ai) |
| `POST /api/conversations` | 3005 | Buat conversation baru |
| `GET/PATCH/DELETE /api/conversations/:id` | 3005 | Detail / rename / hapus conversation |
| `POST /api/chat` | 3005 | Kirim pesan `{conversationId, message}` → AI response (markdown, proxy ke :3001) |
| `GET  /api/monitor/metrics` | 3004 | CPU/RAM/disk/load real (sysinfo, bukan simulasi) |
| `GET  /api/monitor/services` | 3004 | Status semua backend (:3001-3005 + :8080) |
| `GET  /api/monitor/health` | 3004 | Composite health (semua backend dalam 1 call) |
| `GET  /api/ebooks/search?q=...&limit=20` | 3002 | Search ebook (filter docid prefix `ebook-`) |
| `GET  /api/ebooks/:docid` | 3002 | Detail ebook by docid |

### Integrasi Opencode (fitur #8 v3.6.0)

Base URL: `http://192.168.111.103` (LAN IP srv-fine). Generate API key via dashboard
→ menu "API Keys" → "Generate Key Baru" → set di Opencode config sebagai
`Authorization: Bearer lawu_<32hex>`. Detail lengkap: section 18.C di panduan HTML.
