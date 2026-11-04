#!/usr/bin/env bash
# ============================================================
# deploy-from-github.sh — Deploy Lawu-2.5-Fine v3.6.0 di srv-fine
# SATU SUMBER: GitHub repo lawu-2.5-fine-beta (Klutuk006012)
# ------------------------------------------------------------
# Jalankan di srv-fine sebagai user LAWU:
#   cd /tmp
#   git clone --depth 1 https://github.com/Klutuk006012/lawu-dashboard.git lawu-deploy
#   cd lawu-deploy && bash deploy-from-github.sh
#   (atau kalau sudah pernah clone: cd /tmp/lawu-deploy && git pull && bash deploy-from-github.sh)
#
# Yang dikerjakan otomatis (Bagian A, user lawu):
#   A1. Salin corpus_api.rs + ingest_jsonl.rs -> ~/lawu-server/src/bin/
#   A2. Patch Cargo.toml (axum multipart + tokio process) + cargo build --release
#   A3. Salin web_ingest.py + extract_ebooks.py -> /opt/lawu/scripts (kalau izinkan)
#   A4. Konversi korpus-skill -> lawu-skill.jsonl
#   A5. Verifikasi binary
# Lalu mencetak BLOK PERINTAH ROOT (Bagian B) — tinggal su - dan tempel:
#   B1. web-corpus.lst -> /etc/lawu + korpus-skill -> /opt/lawu/korpus-skill
#   B2. python scripts + venv deps + systemd lawu-corpus.service enable
#   B3. Dashboard dist -> /var/www/lawu-dashboard (logo + menu baru)
#   B4. nginx snippet -> reload
#   B5. Ingest korpus skill + test API
# ============================================================
set -euo pipefail

REPO_URL="https://github.com/Klutuk006012/lawu-dashboard.git"
SRC_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SRV_DIR="$HOME/lawu-server"

log()  { printf '\n\033[1;32m==>\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33m[!]\033[0m %s\n' "$*"; }
die()  { printf '\033[1;31m[x]\033[0m %s\n' "$*"; exit 1; }

# ---------- Pemeriksaan awal ----------
log "Pemeriksaan awal"
test -d "$SRV_DIR" || die "$SRV_DIR tidak ada — jalankan dari repo hasil clone di srv-fine"
test -f "$SRC_DIR/server-kit/corpus_api.rs" || die "server-kit/corpus_api.rs tidak ditemukan — repo lama? git pull dulu"
test -f "$SRC_DIR/deploy/dashboard-dist/index.html" || die "deploy/dashboard-dist tidak ditemukan — repo lama? git pull dulu"
echo "  OK: repo v3.6.0 + lawu-server ditemukan"

# ---------- A1. Salin source binary baru ----------
log "A1. Salin binary baru ke $SRV_DIR/src/bin/ (main.rs produksi TIDAK disentuh)"
mkdir -p "$SRV_DIR/src/bin"
cp "$SRC_DIR/server-kit/corpus_api.rs"   "$SRV_DIR/src/bin/corpus_api.rs"
cp "$SRC_DIR/server-kit/ingest_jsonl.rs" "$SRV_DIR/src/bin/ingest_jsonl.rs"
cp "$SRC_DIR/server-kit/auth_api.rs"     "$SRV_DIR/src/bin/auth_api.rs"
cp "$SRC_DIR/server-kit/chat_api.rs"     "$SRV_DIR/src/bin/chat_api.rs"
cp "$SRC_DIR/server-kit/monitor_api.rs"  "$SRV_DIR/src/bin/monitor_api.rs"
cp "$SRC_DIR/server-kit/auth_middleware.rs" "$SRV_DIR/src/auth_middleware.rs"
echo "  OK: corpus_api + ingest_jsonl + auth_api + chat_api + monitor_api + auth_middleware"

# ---------- A2. Patch Cargo.toml + build ----------
log "A2. Patch Cargo.toml + cargo build --release (±1-2 menit)"
python3 - "$SRV_DIR/Cargo.toml" <<'PYEOF'
import re, sys
p = sys.argv[1]
s = open(p).read(); changed = []
def patch_axum(m):
    line = m.group(0)
    if 'multipart' in line: return line
    if 'features = [' in line: return line.replace('features = [', 'features = ["multipart", ', 1)
    return re.sub(r'(axum\s*=\s*)"([^"]+)"', r'\1{ version = "\2", features = ["multipart"] }', line)
new = re.sub(r'^axum\s*=.*$', patch_axum, s, flags=re.M)
if new != s: changed.append('axum(multipart)'); s = new
def patch_tokio(m):
    line = m.group(0)
    if '"full"' in line: return line
    if 'features = [' in line:
        inner = re.search(r'features = \[(.*?)\]', line).group(1)
        need = [f for f in ('"process"', '"sync"', '"fs"') if f not in inner]
        if not need: return line
        return line.replace(']', ', ' + ', '.join(need) + ']', 1)
    return re.sub(r'(tokio\s*=\s*)"([^"]+)"', r'\1{ version = "\2", features = ["process","sync","fs","macros","rt-multi-thread"] }', line)
new = re.sub(r'^tokio\s*=.*$', patch_tokio, s, flags=re.M)
if new != s: changed.append('tokio(process,sync,fs)'); s = new
open(p, 'w').write(s)
print('  Cargo.toml: ' + (', '.join(changed) if changed else 'sudah lengkap (axum+tokio)'))
PYEOF
# Patch v3.6.0: tambah crate baru (rusqlite, jsonwebtoken, argon2, sysinfo, reqwest, dll)
python3 - "$SRV_DIR/Cargo.toml" <<'PYEOF'
import re, sys
p = sys.argv[1]
s = open(p).read(); changed = []
needed = {
    'rusqlite': 'rusqlite = { version = "0.32", features = ["bundled"] }',
    'jsonwebtoken': 'jsonwebtoken = "9"',
    'argon2': 'argon2 = "0.5"',
    'rand': 'rand = "0.8"',
    'uuid': 'uuid = { version = "1", features = ["v4", "serde"] }',
    'sha2': 'sha2 = "0.10"',
    'hex': 'hex = "0.4"',
    'sysinfo': 'sysinfo = "0.32"',
    'reqwest': 'reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }',
    'tower-http': 'tower-http = { version = "0.6", features = ["cors", "trace"] }',
    'tracing': 'tracing = "0.1"',
    'tracing-subscriber': 'tracing-subscriber = { version = "0.3", features = ["env-filter"] }',
}
for k, v in needed.items():
    # cek apakah crate sudah ada di dependencies (regex sederhana)
    if not re.search(rf'^{re.escape(k)}\s*=', s, flags=re.M):
        # tambahkan sebelum [profile.xxx] atau di akhir [dependencies]
        if '[dependencies]' in s:
            s = s.replace('[dependencies]', '[dependencies]\n' + v, 1)
        else:
            s += '\n' + v + '\n'
        changed.append(k)
if changed:
    open(p, 'w').write(s)
    print('  Cargo.toml: + ' + ', '.join(changed))
else:
    print('  Cargo.toml: v3.6.0 deps sudah lengkap')
PYEOF
cd "$SRV_DIR"
cargo build --release 2>&1 | tail -8
# Cargo menamai binary dari nama file di src/bin/ -> corpus_api, ingest_jsonl, auth_api, chat_api, monitor_api.
# systemd unit memakai nama lawu-* — buat alias copy (idempoten).
for pair in corpus_api:lawu-corpus ingest_jsonl:lawu-ingest auth_api:lawu-auth chat_api:lawu-chat monitor_api:lawu-monitor; do
  src="target/release/${pair%%:*}"
  dst="target/release/${pair##*:}"
  if [ ! -x "$src" ]; then
    ls -1 target/release/ 2>/dev/null | grep -Ei 'corpus|ingest|auth|chat|monitor|lawu' || true
    die "binary ${pair%%:*} tidak ditemukan setelah build!"
  fi
  rm -f "$dst" && cp "$src" "$dst"
done
echo "  OK: 5 binary + alias lawu-{corpus,ingest,auth,chat,monitor}"

# ---------- A3. Salin python scripts (kalau izin) ----------
log "A3. Salin web_ingest.py + extract_ebooks.py ke /opt/lawu/scripts"
if cp "$SRC_DIR/server-kit/web_ingest.py" "$SRC_DIR/server-kit/extract_ebooks.py" /opt/lawu/scripts/ 2>/dev/null; then
  chmod +x /opt/lawu/scripts/*.py 2>/dev/null || true
  echo "  OK: scripts tersalin"
else
  warn "izin ditolak (normal) — nanti disalin oleh Bagian B"
fi

# ---------- A4. Korpus skill JSONL ----------
log "A4. Konversi korpus-skill/*.md -> lawu-skill.jsonl"
python3 - "$SRC_DIR/server-kit/korpus-skill" <<'PYEOF'
import json, sys, hashlib, os, re, glob
kit = sys.argv[1]
out = os.path.join(kit, 'lawu-skill.jsonl')
docs = []
for md in sorted(glob.glob(os.path.join(kit, '*.md'))):
    txt = open(md, encoding='utf-8').read()
    txt = re.sub(r'^#.*$', '', txt, flags=re.M)
    txt = re.sub(r'\n{3,}', '\n\n', txt).strip()
    title = os.path.splitext(os.path.basename(md))[0].replace('-', ' ').title()
    slug = re.sub(r'[^a-z0-9]+', '-', title.lower()).strip('-')[:40]
    h = hashlib.sha1(md.encode()).hexdigest()[:8]
    docs.append({'docid': f'skill-{slug}-{h}',
                 'title': f'[Skill] {title} — Materi Pelajaran Lawu Fine',
                 'text': txt})
with open(out, 'w', encoding='utf-8') as f:
    for d in docs: f.write(json.dumps(d, ensure_ascii=False) + '\n')
print(f'  OK: {len(docs)} dokumen skill -> {out}')
PYEOF

# ---------- BLOK PERINTAH ROOT ----------
KIT="$SRC_DIR/server-kit"
DIST="$SRC_DIR/deploy/dashboard-dist"
WWW="/var/www/lawu-dashboard"

printf '\n\033[1;36m%s\033[0m\n' "================= BAGIAN B — JALANKAN SEBAGAI ROOT ================="
printf '\033[0;36m# Ketik: su -   lalu tempel SELURUH blok di bawah ini sekali jalan:\033[0m\n\n'
cat <<ROOTCMDS
# ---------- B1. Konfigurasi korpus + skill ----------
install -m 644 -o root -g root $KIT/web-corpus.lst /etc/lawu/web-corpus.lst
mkdir -p /opt/lawu/korpus-skill
cp $KIT/korpus-skill/*.md $KIT/korpus-skill/lawu-skill.jsonl /opt/lawu/korpus-skill/
chown -R lawu:lawu /opt/lawu/korpus-skill

# ---------- B2. Python venv + scripts + deps + systemd ----------
mkdir -p /opt/lawu/scripts /var/lib/lawu /var/log/lawu /mnt/data/ebooks /mnt/data/processed_markdowns
cp $KIT/web_ingest.py $KIT/extract_ebooks.py /opt/lawu/scripts/ && chmod +x /opt/lawu/scripts/*.py
# Buat venv kalau belum ada (corpus_api.rs hardcode /opt/lawu/venv/bin/python3):
if [ ! -x /opt/lawu/venv/bin/python3 ]; then
  python3 -c 'import venv' 2>/dev/null || apt-get install -y -qq python3-venv
  python3 -m venv /opt/lawu/venv
fi
/opt/lawu/venv/bin/python3 -m pip install --quiet --upgrade pip
/opt/lawu/venv/bin/pip install -q requests beautifulsoup4 lxml pypdf ebooklib python-docx
/opt/lawu/venv/bin/python3 -c 'import requests, bs4, lxml, pypdf, ebooklib, docx'  # smoke import
chown -R lawu:lawu /opt/lawu/venv /var/lib/lawu /var/log/lawu /mnt/data/ebooks /mnt/data/processed_markdowns
# v3.6.0: 3 systemd unit baru (lawu-auth + lawu-chat + lawu-monitor)
cp $KIT/lawu-corpus.service $KIT/lawu-auth.service $KIT/lawu-chat.service $KIT/lawu-monitor.service /etc/systemd/system/
systemctl daemon-reload && systemctl enable --now lawu-corpus.service lawu-auth.service lawu-chat.service lawu-monitor.service
# v3.6.0: SQLite migration (auto-create tables kalau belum ada)
# auth_api.rs sudah auto-migrate di first start, tapi pastikan dir + permission ready
chown -R lawu:lawu /var/lib/lawu

# ---------- B3. Dashboard v3.6.0 (auth + multi-chat + monitoring + UI emerald) ----------
cp -r $DIST/. $WWW/
chown -R www-data:www-data $WWW

# ---------- B4. Nginx: snippet include v3.6.0 (anti-escape-mangle) ----------
# v3.6.0: snippet sekarang berisi 9 location block (auth/users/api-keys/conversations/chat/monitor/corpus/ebooks)
mkdir -p /etc/nginx/snippets
cp $KIT/nginx-corpus-snippet.conf /etc/nginx/snippets/lawu-corpus.conf
# Hapus baris komentar lama (kalau ada — dari sed v3.5.2 yang gagal)
sed -i '/# ---- Corpus & Ebook API/d' /etc/nginx/sites-available/lawu-dashboard 2>/dev/null || true
# Sisipkan include di dalam blok server { }
if ! grep -q 'snippets/lawu-corpus.conf' /etc/nginx/sites-available/lawu-dashboard; then
  if grep -q 'location /api/ {' /etc/nginx/sites-available/lawu-dashboard; then
    sed -i '/location \/api\/ {/i \    include /etc/nginx/snippets/lawu-corpus.conf;' /etc/nginx/sites-available/lawu-dashboard
  elif grep -q 'server_name' /etc/nginx/sites-available/lawu-dashboard; then
    sed -i '/server_name/a \    include /etc/nginx/snippets/lawu-corpus.conf;' /etc/nginx/sites-available/lawu-dashboard
  else
    echo "WARNING: tidak ketemu anchor location /api/ { atau server_name — insert manual include snippet di blok server { }"
  fi
fi
nginx -t && systemctl reload nginx
echo "  OK: nginx snippet v3.6.0 (9 route) di-include"

# ---------- B5. Verifikasi v3.6.0 ----------
echo "--- v3.6.0 health check (4 backend + nginx):"
for p in 3001:lawu-server 3002:lawu-corpus 3003:lawu-auth 3004:lawu-monitor 3005:lawu-chat; do
  PORT=${p%%:*}; NAME=${p##*:}
  if curl -sf http://127.0.0.1:$PORT/health >/dev/null 2>&1; then
    echo "  OK: $NAME (:$PORT) healthy"
  else
    echo "  [!] $NAME (:$PORT) tidak responsif — cek: journalctl -u $NAME -n 20"
  fi
done
echo
echo "--- Test login default (dev@lawu.ai.id / 12345678):"
TOKEN=$(curl -s -X POST http://127.0.0.1:3003/api/auth/login -H 'Content-Type: application/json' -d '{"email":"dev@lawu.ai.id","password":"12345678"}' | python3 -c 'import json,sys; d=json.load(sys.stdin); print(d.get("token","(FAIL)"))' 2>/dev/null)
echo "  Token: ${TOKEN:0:40}..."
echo
echo "=== DEPLOY v3.6.0 SELESAI ==="
echo "Buka dashboard → login dev@lawu.ai.id / 12345678 → verifikasi:"
echo "  - Multi-session chat (sidebar: + Obrolan Baru)"
echo "  - Monitoring real (CPU/RAM/disk/service status)"
echo "  - Ebook search (search box di halaman Ebook)"
echo "  - User management (admin only, menu Manajemen User)"
echo "  - API Keys (menu API Keys, generate lawu_<hex32>)"
echo "  - Markdown render rapi di jawaban AI (code block, list, paragraf)"
echo
echo "WAJIB: ganti password default dev@lawu.ai.id setelah login pertama!"
ROOTCMDS

printf '\n\033[1;32mBAGIAN A SELESAI.\033[0m Lanjutkan BAGIAN B di atas.\n'
printf 'Setelah itu di UI: \033[1mWeb Korpus\033[0m → Mulai Crawl → Ingest Web ke Index;\n'
printf '\033[1mEbook & Korpus\033[0m → upload/extract 30 ebook (lawu-ebooks.tar.gz) → Ingest.\n'
