#!/usr/bin/env bash
# ============================================================
# deploy-from-github.sh — Deploy Lawu-2.5-Fine v3.5.0 di srv-fine
# SATU SUMBER: GitHub repo lawu-dashboard (Klutuk006012)
# ------------------------------------------------------------
# Jalankan di srv-fine sebagai user LAWU:
#   cd /tmp
#   git clone --depth 1 https://github.com/Klutuk006012/lawu-2.5-fine-beta.git lawu-deploy
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

REPO_URL="https://github.com/Klutuk006012/lawu-2.5-fine-beta.git"
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
echo "  OK: repo v3.5.0 + lawu-server ditemukan"

# ---------- A1. Salin source binary baru ----------
log "A1. Salin binary baru ke $SRV_DIR/src/bin/ (main.rs produksi TIDAK disentuh)"
mkdir -p "$SRV_DIR/src/bin"
cp "$SRC_DIR/server-kit/corpus_api.rs"   "$SRV_DIR/src/bin/corpus_api.rs"
cp "$SRC_DIR/server-kit/ingest_jsonl.rs" "$SRV_DIR/src/bin/ingest_jsonl.rs"
echo "  OK: corpus_api.rs + ingest_jsonl.rs"

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
print('  Cargo.toml: ' + (', '.join(changed) if changed else 'sudah lengkap'))
PYEOF
cd "$SRV_DIR"
cargo build --release 2>&1 | tail -4
for b in lawu-corpus lawu-ingest; do
  test -x "target/release/$b" || die "binary $b gagal dibangun!"
done
echo "  OK: target/release/lawu-corpus + lawu-ingest"

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

# ---------- B2. Python scripts + venv deps + systemd ----------
mkdir -p /opt/lawu/scripts /var/lib/lawu /var/log/lawu /mnt/data/ebooks /mnt/data/processed_markdowns
cp $KIT/web_ingest.py $KIT/extract_ebooks.py /opt/lawu/scripts/ && chmod +x /opt/lawu/scripts/*.py
/opt/lawu/venv/bin/pip install -q requests beautifulsoup4 lxml pypdf ebooklib python-docx
chown -R lawu:lawu /var/lib/lawu /var/log/lawu /mnt/data/ebooks /mnt/data/processed_markdowns
cp $KIT/lawu-corpus.service /etc/systemd/system/lawu-corpus.service
systemctl daemon-reload && systemctl enable --now lawu-corpus.service

# ---------- B3. Dashboard v3.5.0 (logo + menu Web Korpus & Ebook) ----------
cp -r $DIST/. $WWW/
chown -R www-data:www-data $WWW

# ---------- B4. Nginx: location /api/corpus + /api/ebooks ----------
if ! grep -q 'api/corpus' /etc/nginx/sites-available/lawu-dashboard; then
  sed -i '/location \/api\/ {/i \
    # ---- Corpus & Ebook API (lawu-corpus :3002) ----\
    location /api/corpus/ { proxy_pass http://127.0.0.1:3002; proxy_http_version 1.1; proxy_set_header Host \$host; proxy_read_timeout 60s; }\
    location /api/ebooks { proxy_pass http://127.0.0.1:3002; proxy_http_version 1.1; proxy_set_header Host \$host; client_max_body_size 500M; proxy_read_timeout 300s; proxy_request_buffering off; }\
' /etc/nginx/sites-available/lawu-dashboard
fi
nginx -t && systemctl reload nginx

# ---------- B5. Ingest korpus skill + verifikasi ----------
$SRV_DIR/target/release/lawu-ingest --file /opt/lawu/korpus-skill/lawu-skill.jsonl
sleep 1
curl -s http://127.0.0.1:3002/health && echo
systemctl status lawu-corpus --no-pager -l | head -8
echo "=== DEPLOY SELESAI — buka dashboard, menu 'Web Korpus' & 'Ebook & Korpus' siap ==="
ROOTCMDS

printf '\n\033[1;32mBAGIAN A SELESAI.\033[0m Lanjutkan BAGIAN B di atas.\n'
printf 'Setelah itu di UI: \033[1mWeb Korpus\033[0m → Mulai Crawl → Ingest Web ke Index;\n'
printf '\033[1mEbook & Korpus\033[0m → upload/extract 30 ebook (lawu-ebooks.tar.gz) → Ingest.\n'
