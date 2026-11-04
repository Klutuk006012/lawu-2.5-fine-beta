#!/usr/bin/env bash
# ============================================================
# fix-nginx-corpus.sh — fix location /api/corpus + /api/ebooks
# ------------------------------------------------------------
# Insiden v3.5.4: deploy B4 pakai sed multi-line dengan escape \
# collapse jadi 1 baris saat paste SSH. Karena diawali '#', nginx
# baca sebagai komentar -> location /api/corpus dan /api/ebooks
# tidak terdaftar -> browser dapat HTTP 404 di menu Web Korpus &
# Ebook & Korpus.
#
# Fix: tulis location blocks ke file snippet terpisah via cat
# heredoc (literal, no escape), lalu include-kan ke config utama
# via sed satu baris (single-line include statement, anti-mangle).
#
# Jalankan sebagai root. Idempoten.
# ============================================================
set -e

NGINX_CONF=/etc/nginx/sites-available/lawu-dashboard
SNIPPET_FILE=/etc/nginx/snippets/lawu-corpus.conf

echo "=== [1/6] backup config lama ==="
cp "$NGINX_CONF" "${NGINX_CONF}.bak.$(date +%s)"
echo "  OK: backup ke ${NGINX_CONF}.bak.*"

echo
echo "=== [2/6] hapus baris komentar error (sed v3.5.2 yang gagal) ==="
sed -i '/# ---- Corpus & Ebook API/d' "$NGINX_CONF" || true
echo "  OK: baris komentar lama dihapus (kalau ada)"

echo
echo "=== [3/6] tulis snippet ke $SNIPPET_FILE ==="
mkdir -p /etc/nginx/snippets
cat > "$SNIPPET_FILE" <<'NGINXEOF'
# ---- Corpus & Ebook API (Axum :3002) ----
location /api/corpus/ {
    proxy_pass http://127.0.0.1:3002;
    proxy_http_version 1.1;
    proxy_set_header Host $host;
    proxy_set_header X-Real-IP $remote_addr;
    proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    proxy_read_timeout 60s;
}

location /api/ebooks {
    proxy_pass http://127.0.0.1:3002;
    proxy_http_version 1.1;
    proxy_set_header Host $host;
    proxy_set_header X-Real-IP $remote_addr;
    proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    client_max_body_size 500M;
    proxy_read_timeout 300s;
    proxy_request_buffering off;
}
NGINXEOF
echo "  OK: snippet ditulis ($(wc -l < $SNIPPET_FILE) baris)"

echo
echo "=== [4/6] sisipkan include di blok server { } ==="
if grep -q 'snippets/lawu-corpus.conf' "$NGINX_CONF"; then
  echo "  (sudah ada include, skip)"
elif grep -q 'location /api/ {' "$NGINX_CONF"; then
  sed -i '/location \/api\/ {/i \    include /etc/nginx/snippets/lawu-corpus.conf;' "$NGINX_CONF"
  echo "  OK: include disisipkan sebelum 'location /api/ {'"
elif grep -q 'server_name' "$NGINX_CONF"; then
  sed -i '/server_name/a \    include /etc/nginx/snippets/lawu-corpus.conf;' "$NGINX_CONF"
  echo "  OK: include disisipkan setelah 'server_name'"
else
  echo "  WARNING: tidak ketemu anchor location /api/ { atau server_name"
  echo "  Edit manual $NGINX_CONF, tambahkan di dalam server { }:"
  echo "    include /etc/nginx/snippets/lawu-corpus.conf;"
  exit 1
fi

echo
echo "=== [5/6] test nginx config ==="
nginx -t

echo
echo "=== [6/6] reload + verifikasi via port 80 ==="
systemctl reload nginx
echo "--- GET /health (proxy ke :3002):"
curl -sf http://127.0.0.1/health; echo
echo "--- GET /api/corpus/web (sebelumnya 404 di UI):"
curl -s -o /tmp/r.json -w "HTTP %{http_code}, body %{size_download} byte\n" http://127.0.0.1/api/corpus/web
head -c 200 /tmp/r.json; echo
echo "--- GET /api/ebooks (sebelumnya 404 di UI):"
curl -s -o /tmp/r.json -w "HTTP %{http_code}, body %{size_download} byte\n" http://127.0.0.1/api/ebooks
head -c 200 /tmp/r.json; echo
echo
echo "=== SELESAI — refresh browser, menu Web Korpus & Ebook harus jalan ==="
