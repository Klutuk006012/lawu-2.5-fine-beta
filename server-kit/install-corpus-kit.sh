#!/usr/bin/env bash
# ============================================================
# install-corpus-kit.sh — Installer Web Korpus + Ebook UI Kit
# Lawu-2.5-Fine @ srv-fine (192.168.111.103)
# ------------------------------------------------------------
# Jalankan sebagai user LAWU (bukan root):
#   cd /tmp/corpus-kit && bash install-corpus-kit.sh
#
# Yang dilakukan otomatis:
#   A1. Salin corpus_api.rs + ingest_jsonl.rs -> ~/lawu-server/src/bin/
#   A2. Patch Cargo.toml (axum feature "multipart", tokio process/sync)
#   A3. cargo build --release (binary lawu-corpus + lawu-ingest)
#   A4. Salin web_ingest.py + extract_ebooks.py -> /opt/lawu/scripts
#       (jika izin ditolak, perintah root dicetak di bagian akhir)
#   A5. Konversi korpus-skill/*.md -> lawu-skill.jsonl (siap ingest)
#
# Yang dilakukan MANUAL sebagai root (instruksi dicetak di akhir):
#   B1. web-corpus.lst -> /etc/lawu/web-corpus.lst
#   B2. systemd unit lawu-corpus.service + enable
#   B3. nginx snippet (location /api/corpus/ + /api/ebooks)
#   B4. Ingest korpus skill + web corpus
# ============================================================
set -euo pipefail

KIT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SRV_DIR="$HOME/lawu-server"
SCRIPTS_DIR="/opt/lawu/scripts"
VENV_PY="/opt/lawu/venv/bin/python3"

log() { printf '\n\033[1;32m==>\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33m[!]\033[0m %s\n' "$*"; }
die() { printf '\033[1;31m[x]\033[0m %s\n' "$*"; exit 1; }

# ---------- A1. Salin source bin ----------
log "A1. Salin source binary baru ke $SRV_DIR/src/bin/"
mkdir -p "$SRV_DIR/src/bin"
cp "$KIT_DIR/corpus_api.rs"   "$SRV_DIR/src/bin/corpus_api.rs"
cp "$KIT_DIR/ingest_jsonl.rs" "$SRV_DIR/src/bin/ingest_jsonl.rs"
echo "  OK: src/bin/corpus_api.rs + src/bin/ingest_jsonl.rs (main.rs TIDAK disentuh)"

# ---------- A2. Patch Cargo.toml ----------
log "A2. Patch Cargo.toml (axum multipart + tokio process/sync/fs)"
python3 - "$SRV_DIR/Cargo.toml" <<'PYEOF'
import re, sys
p = sys.argv[1]
s = open(p).read()
changed = []

# axum: pastikan feature multipart ada
def patch_axum(m):
    line = m.group(0)
    if 'multipart' in line:
        return line
    if 'features = [' in line:
        return line.replace('features = [', 'features = ["multipart", ', 1)
    return re.sub(r'(axum\s*=\s*)"([^"]+)"', r'\1{ version = "\2", features = ["multipart"] }', line)

new = re.sub(r'^axum\s*=.*$', patch_axum, s, flags=re.M)
if new != s: changed.append('axum(multipart)'); s = new

# tokio: pastikan process+sync+fs ada (atau sudah "full")
def patch_tokio(m):
    line = m.group(0)
    if 'features = ["full"]' in line or '"full"' in line:
        return line
    if 'features = [' in line:
        inner = re.search(r'features = \[(.*?)\]', line).group(1)
        need = [f for f in ('"process"', '"sync"', '"fs"') if f not in inner]
        if not need:
            return line
        add = ', ' + ', '.join(need)
        return line.replace('features = [', 'features = [', 1).replace(']', add + ']', 1) \
            if 'features = [' in line else line
    return re.sub(r'(tokio\s*=\s*)"([^"]+)"', r'\1{ version = "\2", features = ["process","sync","fs","macros","rt-multi-thread"] }', line)

new = re.sub(r'^tokio\s*=.*$', patch_tokio, s, flags=re.M)
if new != s: changed.append('tokio(process,sync,fs)'); s = new

open(p, 'w').write(s)
print('  Cargo.toml diperbarui: ' + (', '.join(changed) if changed else 'sudah lengkap, tidak ada perubahan'))
PYEOF

# ---------- A3. Build ----------
log "A3. cargo build --release (binary: corpus_api + ingest_jsonl, alias lawu-corpus + lawu-ingest)"
cd "$SRV_DIR"
cargo build --release 2>&1 | tail -5
# Cargo menamai binary dari nama file di src/bin/ (corpus_api, ingest_jsonl).
# systemd unit (ExecStart) & corpus_api.rs (spawn lawu-ingest) memakai nama
# lawu-corpus / lawu-ingest — buat alias copy setelah build (rm dulu supaya
# aman dijalankan ulang saat binary lama masih berjalan).
for pair in corpus_api:lawu-corpus ingest_jsonl:lawu-ingest; do
  src="target/release/${pair%%:*}"
  dst="target/release/${pair##*:}"
  if [ ! -x "$src" ]; then
    ls -1 target/release/ 2>/dev/null | grep -Ei 'corpus|ingest|lawu' || true
    die "binary ${pair%%:*} tidak ditemukan setelah build!"
  fi
  rm -f "$dst" && cp "$src" "$dst"
done
echo "  OK: target/release/lawu-corpus + target/release/lawu-ingest"

# ---------- A4. Salin python scripts ----------
log "A4. Salin web_ingest.py + extract_ebooks.py ke $SCRIPTS_DIR"
if cp "$KIT_DIR/web_ingest.py" "$KIT_DIR/extract_ebooks.py" "$SCRIPTS_DIR/" 2>/dev/null; then
  chmod +x "$SCRIPTS_DIR/web_ingest.py" "$SCRIPTS_DIR/extract_ebooks.py"
  echo "  OK: scripts tersalin"
  COPY_OK=1
else
  warn "Izin ditolak (normal) — akan disalin via langkah root di bagian B"
  COPY_OK=0
fi

# ---------- A5. Konversi korpus skill -> JSONL ----------
log "A5. Konversi korpus-skill/*.md -> lawu-skill.jsonl"
python3 - "$KIT_DIR/korpus-skill" <<'PYEOF'
import json, sys, hashlib, os, re, glob
kit = sys.argv[1]
out = os.path.join(kit, 'lawu-skill.jsonl')
docs = []
for md in sorted(glob.glob(os.path.join(kit, '*.md'))):
    txt = open(md, encoding='utf-8').read()
    txt = re.sub(r'^#.*$', '', txt, flags=re.M)  # buang baris komentar '#'
    txt = re.sub(r'\n{3,}', '\n\n', txt).strip()
    title = os.path.splitext(os.path.basename(md))[0].replace('-', ' ').title()
    slug = re.sub(r'[^a-z0-9]+', '-', title.lower()).strip('-')[:40]
    h = hashlib.sha1(md.encode()).hexdigest()[:8]
    docs.append({
        'docid': f'skill-{slug}-{h}',
        'title': f'[Skill] {title} — Materi Pelajaran Lawu Fine',
        'text': txt,
    })
with open(out, 'w', encoding='utf-8') as f:
    for d in docs:
        f.write(json.dumps(d, ensure_ascii=False) + '\n')
print(f'  OK: {len(docs)} dokumen skill -> {out}')
PYEOF

# ---------- Verifikasi akhir ----------
log "Verifikasi cepat"
echo "  - src/bin/ : $(ls "$SRV_DIR/src/bin" 2>/dev/null | tr '\n' ' ')"
echo "  - binary   : $(ls "$SRV_DIR/target/release/lawu-corpus" "$SRV_DIR/target/release/lawu-ingest" 2>/dev/null | tr '\n' ' ')"

# ---------- Instruksi root ----------
ROOT_CMDS=$(cat <<ROOTCMDS
# Jalankan: su -  lalu tempel blok berikut
# ------------------------------------------------------------
# B1. Web corpus list
install -m 644 -o root -g root $KIT_DIR/web-corpus.lst /etc/lawu/web-corpus.lst

# B1b. Korpus skill (materi Advanced Google Search) untuk tombol Ingest di UI
mkdir -p /opt/lawu/korpus-skill
cp $KIT_DIR/korpus-skill/*.md $KIT_DIR/korpus-skill/lawu-skill.jsonl /opt/lawu/korpus-skill/ 2>/dev/null || true
chown -R lawu:lawu /opt/lawu/korpus-skill

# B2. Python scripts + systemd service (jika A4 gagal)
mkdir -p /opt/lawu/scripts
cp $KIT_DIR/web_ingest.py $KIT_DIR/extract_ebooks.py /opt/lawu/scripts/
chmod +x /opt/lawu/scripts/*.py
cp $KIT_DIR/lawu-corpus.service /etc/systemd/system/lawu-corpus.service

# venv dependensi crawler (sekali saja)
/opt/lawu/venv/bin/pip install -q requests beautifulsoup4 lxml pypdf ebooklib python-docx

# izin tulis untuk user lawu (state crawler + custom URL + log)
mkdir -p /var/lib/lawu /var/log/lawu /mnt/data/ebooks /mnt/data/processed_markdowns
chown -R lawu:lawu /var/lib/lawu /var/log/lawu /mnt/data/ebooks /mnt/data/processed_markdowns

systemctl daemon-reload
systemctl enable --now lawu-corpus.service
systemctl status lawu-corpus.service --no-pager -l | head -12

# B3. nginx — buka config dan tempel isi nginx-corpus-snippet.conf
#     DI DALAM blok server { }
# nano /etc/nginx/sites-available/lawu-dashboard
nginx -t && systemctl reload nginx

# B4. Ingest korpus skill (materi Advanced Google Search) — sebagai lawu
#     (BUKAN root — agar file Tantivy index tidak jadi milik root):
su - lawu -c "$SRV_DIR/target/release/lawu-ingest --file $KIT_DIR/korpus-skill/lawu-skill.jsonl"

#     Ingest hasil crawl web korpus (setelah crawl pertama selesai):
# su - lawu -c "$SRV_DIR/target/release/lawu-ingest --file /mnt/data/processed_markdowns/web_corpus.jsonl"
#     Ingest ebook (setelah extract selesai):
# su - lawu -c "$SRV_DIR/target/release/lawu-ingest --file /mnt/data/processed_markdowns/ebooks.jsonl"

# Test API:
curl -s http://127.0.0.1:3002/health
curl -s http://127.0.0.1:3002/api/corpus/web | head -c 400
ROOTCMDS
)

printf '\n\033[1;36m%s\033[0m\n' "================ BAGIAN B (MANUAL sebagai root) ================"
printf '%s\n' "$ROOT_CMDS"
printf '\n\033[1;32mSELESAI BAGIAN A.\033[0m Lanjutkan Bagian B di atas, lalu uji UI:\n'
echo "  Dashboard -> menu 'Web Korpus' (add URL + crawl)"
echo "  Dashboard -> menu 'Ebook' (upload + extract)"
