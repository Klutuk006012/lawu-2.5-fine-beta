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

## Isi Repo (yang relevan untuk deploy)

| Path | Fungsi |
|---|---|
| `deploy-from-github.sh` | **Script deploy utama** — dijalankan di srv-fine |
| `deploy/dashboard-dist/` | Dashboard Vue v3.5.0 hasil build final (logo + menu baru) |
| `server-kit/corpus_api.rs` | Binary Axum **:3002** — API korpus & ebook (src/bin/, main.rs tidak disentuh) |
| `server-kit/ingest_jsonl.rs` | Binary `lawu-ingest` — JSONL → Tantivy (dedup docid) |
| `server-kit/web_ingest.py` | Crawler web korpus (BFS, robots.txt, 1 JSONL) |
| `server-kit/extract_ebooks.py` | Extract PDF/EPUB/DOCX/TXT → ebooks.jsonl |
| `server-kit/web-corpus.lst` | 32+ URL seed resmi user, terkategori |
| `server-kit/korpus-skill/` | Materi **Advanced Google Search** (skill wajib Lawu Fine) |
| `server-kit/lawu-corpus.service` | systemd unit untuk :3002 |

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
