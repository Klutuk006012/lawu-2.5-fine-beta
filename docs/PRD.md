# PRD — Lawu-2.5-Fine · Server Setup & Deployment

**Versi:** 3.6.0
**Update:** 2026-11-04
**Perubahan major v3.1.0:** Hardening server dipindah ke LANGKAH TERAKHIR (sebelum go-live checklist). Setup: **jalan dulu → lalu amankan**.
**Perubahan v3.1.1:** HDD ikut terformat saat install ulang OS. Langkah 3 diperkuat: fstab AUTOMOUNT wajib dengan `nofail` + validasi `findmnt --verify` + TEST REBOOT sebelum lanjut.
**Perubahan v3.2.0:** Korpus baru **Wikipedia-ID** (`indonesian-nlp/wikipedia-id` — 1,44 juta artikel, parquet ~345MB) sebagai dataset pelatihan Indonesia. Langkah 12 baru; langkah 12-19 lama bergeser ke 13-20. Total **21 langkah (0-20)**.
**Perubahan v3.3.0 (2026-11-04):** (1) RAM di-upgrade dari 8GB ke **16GB DDR4** — semua tuning dilonggarkan kembali ke nilai optimal; (2) Model di-upgrade dari Q3_K_M (3.55GB) ke **Q4_K_M (4.4GB)** untuk kualitas lebih baik; (3) Korpus Wikipedia-ID berhasil terindex (1,38 juta artikel, query < 1ms); (4) Langkah 12 & 13 selesai di server asli.
**Perubahan v3.4.0 (2026-11-04):** REVERT Paten 2 kembali ke **Vue.js 3 (Vite + Pinia)** — sesuai tegaskan user. Sebelumnya v3.3.0 sempat salah ubah ke Next.js karena dashboard development pakai Next.js (eksperimen). Dashboard Vue.js 3 akan dibangun ulang menggantikan Next.js. Backend Rust Axum + Tantivy + Wikipedia-ID tetap (tidak berubah).
**Perubahan v3.6.0 (2026-11-04):** Dashboard berubah dari "view-only" menjadi **aplikasi penuh**: auth (login/JWT) + multi-session chat (SQLite persistence) + monitoring real (sysinfo) + user & API-key management. 9 fitur baru, 3 binary Rust baru (lawu-auth :3003, lawu-chat :3005, lawu-monitor :3004) + extend corpus_api :3002 dengan `/api/ebooks/search`. SQLite `/var/lib/lawu/auth.db` via rusqlite bundled (no system sqlite3 dep). JWT HS256 + argon2 password hash. UI redesign ke emerald (`#10b981`, ganti Google Blue). Lihat pelajaran #20.

---

## 1. Ringkasan Proyek

Lawu-2.5-Fine adalah AI model Indonesia ("AI Model Cerdas, Irit Budget") untuk search engine ebook dengan 7 Paten terkunci yang TIDAK BOLEH dilanggar:

| # | Paten | Status |
|---|---|---|
| 1 | **Rust** = satu-satunya backend (Axum + Tantivy) | ✅ Terpenuhi |
| 2 | **Vue.js 3** = satu-satunya frontend (Vite + Pinia) | ✅ Terpenuhi (v3.4.0 — revert ke Vue.js, sesuai tegaskan user) |
| 3 | **Pure CPU+RAM+SSD** = NO GPU, NO cloud API | ✅ Terverifikasi (`ldd` bersih) |
| 4 | **Linux only** = Debian 13 (trixie) | ✅ Terpenuhi |
| 5 | **Open source** = MIT/Apache/BSD/GPL saja | ✅ Terpenuhi |
| 6 | **Parallel RARE** = 5-way retrieval + 1 LLM call | ✅ Terpenuhi |
| 7 | **Islam** = Ahlussunnah wal Jama'ah, permanen | ✅ Terpenuhi |

**Target hardware:**
- CPU: Intel Core i5-10400 (6C/12T, AVX2)
- RAM: **16 GB DDR4** (di-upgrade dari 8GB pada 2026-11-04 — lihat Pelajaran #10)
- Disk: SSD 128 GB (OS) + HDD 500 GB (DATA — ebook, index, models, backup)

---

## 2. Prinsip Setup yang BENAR (Pelajaran Penting v3.1.0)

> ⚠️ **Pelajaran mahal dari incident 2026-11-03:**
> Hardening (disable SSH password) di awal setup menyebabkan user terkunci dari server
> sendiri → reinstall OS. Ini kesalahan urutan, bukan kesalahan teknik.

**Prinsip deployment yang benar:**

```
1. JALANKAN DULU (get it running)
   ↓ Install semua service, test endpoint, debug dari SSH password biasa
2. LALU AMANKAN (lock it down)
   ↓ Setelah SEMUA service jalan + test sukses → baru hardening
3. TERAKHIR: GO-LIVE CHECKLIST
   ↓ Audit final sebelum buka ke publik
```

**Alasan teknis:**
- SSH password aktif = debugging mudah (bisa SSH dari laptop kapan saja)
- UFW belum aktif = tidak ada risiko port blocked salah konfigurasi
- fail2ban belum jalan = tidak ada risiko IP sendiri ke-ban
- Kalau ada error setup, fix-nya cepat tanpa harus akses konsol fisik

---

## 3. Urutan Langkah Setup yang BENAR (v3.1.0)

### FASE 1 — Persiapan Server (Langkah 0-3)

| Langkah | Judul | Isi |
|---|---|---|
| **0** | Bersihkan Server | Audit → stop service lama → backup → hapus → verify |
| **1** | Prerequisites | apt update + install build deps, nginx, python venv |
| **2** | User lawu & Direktori | useradd lawu (no sudo), /opt/lawu (SSD), /mnt/data (HDD), symlink ~/models → /mnt/data/models |
| **3** | Mount & Format HDD | parted GPT → mkfs.ext4 → **fstab automount (nofail + device-timeout)** → findmnt --verify → **TEST REBOOT** → TRIM → SSD tuning |

### FASE 2 — Build & Deploy Services (Langkah 4-13)

| Langkah | Judul | Isi |
|---|---|---|
| **4** | Arsitektur | Diagram end-to-end alur data |
| **5** | Kernel Tuning | sysctl (vm.swappiness=20 utk RAM 16GB) + limits.conf |
| **6** | Install Rust | rustup stable, RUSTFLAGS=-C target-cpu=native, verify ldd murni CPU |
| **7** | Project Cargo | cargo new lawu-server + Cargo.toml (axum 0.8, tantivy 0.24) + build release |
| **8** | Tantivy schema | schema.rs multi-field BM25 (docid, title, text, split) |
| **9** | Axum API | main.rs: /health, /api/search, /api/stats |
| **10** | Build llama.cpp | clone ggml-org/llama.cpp tag b12069+, cmake STATIC linking, -j 6 (RAM 16GB) |
| **11** | Model GGUF | wget pre-quantized Qwen2.5-Coder-7B **Q4_K_M (4.4GB)** ke HDD |
| **12** | Korpus Wikipedia-ID | Download dataset `indonesian-nlp/wikipedia-id` → JSONL streaming → Tantivy index |
| **13** | Serve + systemd | llama-server -t 6 -c 4096, test /v1/chat/completions, systemd auto-start |

### FASE 3 — Otomasi & Dashboard (Langkah 14-18)

| Langkah | Judul | Isi |
|---|---|---|
| **13** | Serve + systemd | llama-server -t 6 -c 4096, test /v1/chat/completions, systemd auto-start |
| **14** | Otomasi | watch_ebooks.sh, backup_lawu.sh, send_error_report.sh, cron, /etc/lawu/env |
| **15** | Load Testing | k6: p95 /api/search < 300ms dengan 20 VUs |
| **16** | Nginx Reverse Proxy | port 80 → Axum :3001 + llama :8080 (/v1/) |
| **17** | Integrasi Dashboard | Vue.js 3 dashboard (Vite + Pinia) deploy ke srv-fine, proxy ke Rust backend |
| **18** | ⭐ **HARDENING** | UFW (deny incoming, allow 22/80/443) + SSH key-only + fail2ban — **DIPINDAH KE SINI (terakhir sebelum go-live)** |
| **19** | Go-Live Checklist | Audit final: sistem, disk, firewall, services, ports, API health, cron, backup, memory |

### FASE 4 — Amankan & Go-Live (Langkah 18-19)

| Langkah | Judul | Isi |
|---|---|---|
| **18** | ⭐ **HARDENING** | UFW (deny incoming, allow 22/80/443) + SSH key-only + fail2ban — **TERAKHIR sebelum go-live** |
| **19** | Go-Live Checklist | Audit final: sistem, disk, firewall, services, ports, API health, cron, backup, memory |

---

## 4. Hardening — Detail Langkah 18 (v3.3.0)

Hardening HANYA dijalankan setelah:
- ✅ Semua service jalan (systemd active, HTTP 200)
- ✅ Test endpoint sukses (chat completion bisa respon)
- ✅ Load test lulus (p95 < 300ms)
- ✅ Dashboard Vue.js connect ke API

**Urutan hardening yang aman:**

```bash
# 1) PASTIKAN SSH KEY SUDAH TERSETUP + TEST LOGIN DENGAN KEY
#    (di laptop: ssh -i ~/.ssh/lawu_admin admin@server — harus sukses)
# 2) BARU disable password auth
# 3) Enable UFW (test rule 22 dulu sebelum deny)
# 4) fail2ban TERAKHIR (setelah semua lainnya aman)
```

**Jangan pernah:**
- ❌ Disable SSH password sebelum SSH key terverifikasi
- ❌ Enable UFW default-deny sebelum test akses current
- ❌ Hardening saat masih ada langkah setup yang belum selesai

---

## 5. Stack Teknologi & Versi (Terverifikasi di Server Asli)

| Komponen | Versi | Catatan |
|---|---|---|
| Debian | 13 (trixie) | Kernel 6.x |
| Rust | 1.99.1 (2026-10-13) | via rustup, stable |
| Axum | 0.8 | (bukan 0.7 — incompatible dengan Rust 1.99) |
| Tantivy | 0.24.2 | (bukan 0.22 — API berubah) |
| tokio | 1.x | full features |
| llama.cpp | b12069 (build 12069, commit e6192de0a) | ggml-org repo, static linking |
| Model | Qwen2.5-Coder-7B **Q4_K_M** | 4.4GB, ~7 tok/s di i5-10400 (RAM 16GB) |
| Python | 3.13 | venv untuk pyarrow + huggingface-cli |
| Nginx | 1.26+ | reverse proxy |
| Frontend | **Vue.js 3 (Vite + Pinia)** | (Paten 2 — WAJIB Vue.js, BUKAN Next.js) |

---

## 6. Konfigurasi RAM 16GB (Di-upgrade dari 8GB pada 2026-11-04)

**RAM budget saat inference (7B Q4_K_M + 16GB):**
```
Model mmap load    : ~4.4GB
KV cache ctx 4096  : ~0.5GB
Axum + Tantivy     : ~0.6GB
OS + page cache    : ~2.5GB
─────────────────────────────────
TOTAL              : ~8.0GB (headroom ~8GB)
```

**Parameter yang disesuaikan (16GB — dilonggarkan dari tuning 8GB):**

| Parameter | Nilai (RAM 16GB, v3.3.0) | Nilai lama (RAM 8GB, v3.0-3.2) |
|---|---|---|
| `vm.swappiness` | **20** | 40 |
| `cmake --build -j` | **6** | 3 |
| `llama-server -t` | **6** | 4 |
| `llama-server -c` | **4096** | 2048 |
| Model default | **Q4_K_M (4.4GB)** | Q3_K_M (3.6GB) |

---

## 7. Struktur Direktori

```
SSD 128GB (OS):
├── /opt/lawu/              # scripts, binary, logs (755/640)
├── /home/lawu/
│   ├── src/llama.cpp/      # source tree (build di sini — cepat)
│   ├── .cargo/bin/lawu-server  # binary Axum API
│   └── models -> /mnt/data/models  # SYMLINK ke HDD

HDD 500GB (DATA — /dev/sdb1, ext4, automount via fstab: `defaults,noatime,nofail,x-systemd.device-timeout=10s`):
# nofail = WAJIB — server tetap boot walau HDD gagal/cabut
# WAJIB test reboot setelah konfigurasi: mount manual HILANG saat reboot!
├── /mnt/data/
│   ├── datasets/           # dataset mentah (parquet)
│   │   └── wikipedia-id/   # train/test parquet (~345MB, indonesian-nlp)
│   ├── ebooks/             # PDF·EPUB·DOCX koleksi
│   │   ├── incoming/       # file watcher (inotifywait)
│   │   └── processed/
│   ├── processed_markdowns/  # hasil ekstraksi MarkltDown + korpus Wikipedia-ID
│   │   └── wikipedia-id/{train,test}/{a..z,_}/*.md  # 1,44jt artikel (Langkah 12)
│   ├── tantivy_index/      # BM25 index
│   ├── backup/             # retensi 7 hari
│   └── models/             # GGUF (fisik di sini!)
```

---

## 8. Status Setup Saat Ini (Server Asli)

> ⚠️ **RESET — server di-reinstall ulang (HDD ikut terformat).** Semua progress sebelumnya
> hilang. Mulai ulang dari Langkah 0. Model GGUF Q4_K_M (4.4GB) download ulang di Langkah 11.
>
> ✅ **UPDATE v3.6.0 (2026-11-04):** Dashboard menjadi aplikasi penuh dengan auth + multi-session chat + monitoring real + management API. 9 fitur baru: (1) multi-session chat seperti chat.z.ai dengan SQLite persistence (/api/conversations + /api/chat dengan conversationId); (2) UI redesign ke emerald/teal (ganti Google Blue), pakai @tailwindcss/typography untuk prose; (3) monitoring real CPU/RAM/disk/service via sysinfo crate, label "(Simulasi)" dihapus; (4) AI chat markdown rendering via marked + DOMPurify (paragraf rapi, code block, list); (5) ebook search working (/api/ebooks/search?q=... filter docid prefix `ebook-`); (6) login dashboard (default dev@lawu.ai.id / 12345678, guest hanya akses /chat + /guide); (7) user management (CRUD users, admin only); (8) Base URL open ke LAN (192.168.111.103) untuk integrasi Opencode; (9) API key management (generate lawu_<random32>, hash SHA-256, revoke). 3 binary baru: lawu-auth :3003, lawu-chat :3005, lawu-monitor :3004. SQLite /var/lib/lawu/auth.db (users, api_keys, chat_sessions, chat_messages). JWT HS256 + argon2 password hash. Lihat pelajaran #20.
>
> ✅ **UPDATE v3.5.0 (2026-11-04):** Web Korpus + Ebook UI — semua sumber aset sandbox diimplementasikan
> ke server asli: (1) 30+ ebook dikemas `lawu-ebooks.tar.gz` 244MB; (2) logo brand resmi
> Lawu-2.5-Fine (gunung Lawu + elang + Merah Putih) tertanam di sidebar + favicon; (3) menu UI
> **Web Korpus** & **Ebook & Korpus** (Vite build → lawu-dashboard-v3.tar.gz); (4) backend corpus
> baru `lawu-corpus` (:3002, binary TERPISAH — main.rs produksi tidak disentuh); (5) crawler
> `web_ingest.py` + `web-corpus.lst` 32+ URL seed resmi (prinsip: sumber BELAJAR wajib GRATIS
> & OPEN SOURCE — paten user); (6) materi skill **Advanced Google Search** sebagai korpus skill;
> (7) binary `lawu-ingest` (JSONL → Tantivy, dedup docid).
>
> ✅ **UPDATE v3.5.1 (2026-11-04):** Workflow deploy jadi **GitHub murni** — repo
> `Klutuk006012/lawu-2.5-fine-beta` (public, tag v3.5.0) sumber tunggal deploy: di srv-fine
> cukup `git clone --depth 1` + `bash deploy-from-github.sh` (Bagian A otomatis sebagai user
> lawu, Bagian B blok root siap tempel). File besar (`lawu-ebooks.tar.gz` 244MB + 19 PDF ebook)
> TIDAK masuk repo — dikirim langsung ke srv-fine (scp/rsync). Bugfix terverifikasi di sandbox:
> `corpus_api.rs` gagal compile di srv-fine (5 error: E0308/E0382/E0631×3) → di-reproduksi PERSIS
> di sandbox (cargo 1.99.1), diperbaiki, rebuild + smoke test 8 endpoint hijau semua. Lihat
> pelajaran #16 & #17.
>
> ✅ **UPDATE v3.5.2 (2026-11-04):** cargo build di srv-fine sukses tapi script mati
> `binary lawu-corpus gagal dibangun!` — Cargo menamai binary dari nama file `src/bin/`
> (`corpus_api`, `ingest_jsonl`) sedangkan unit systemd & konstanta `LAWU_INGEST` memanggil
> `lawu-corpus` / `lawu-ingest`. Solusi: script deploy & install-kit kini membuat **alias copy**
> setelah build (idempoten), ingest di Bagian B jalan sebagai `lawu` (bukan root), health check
> retry 5×. Lihat pelajaran #18.
>
> ✅ **UPDATE v3.3.0 (2026-11-04):** RAM di-upgrade dari 8GB ke 16GB. Model di-upgrade dari
> Q3_K_M ke Q4_K_M. Korpus Wikipedia-ID 1,38 juta artikel sudah ter-index (Langkah 12 selesai).
> Search API lawu-server jalan sebagai systemd (Langkah 13 selesai). Test serve llama-server
> berhasil (7.91 tok/s dengan Q3_K_M, akan re-test dengan Q4_K_M).

| Langkah | Status | Catatan |
|---|---|---|
| 0. Bersihkan server | ✅ | server fresh hasil reinstall |
| 1. Prerequisites | ✅ | apt deps terinstall |
| 2. User & direktori | ✅ | user lawu + symlinks |
| 3. Mount HDD + AUTOMOUNT | ✅ | fstab nofail verified, test reboot OK |
| 4. Arsitektur | ✅ | /mnt/data/* struktur siap |
| 5. sysctl | ✅ | swappiness=20 (16GB) |
| 6. Rust toolchain | ✅ | rustc 1.99.1 |
| 7. Cargo project | ✅ | lawu-server v0.1.0 (Paten 3 ✓) |
| 8. Tantivy schema | ✅ | 4 fields: docid, title, text, split |
| 9. Axum API | ✅ | /health, /api/search, /api/stats |
| 10. llama.cpp build | ✅ | b12069, gcc 14.2.0, -j 6 |
| 11. Model GGUF | ✅ | Q4_K_M (4.4GB) terdownload, service updated |
| 12. Korpus Wikipedia-ID | ✅ | 1.38jt artikel terindex, query < 1ms |
| 13. Serve llama.cpp | ✅ | test OK, Q4_K_M aktif, systemd auto-start |
| 13b. systemd lawu-server | ✅ | auto-start post-reboot verified |
| 14. systemd llama-server | ✅ | lawu-llama.service active, Q4_K_M |
| 15. Otomasi | ✅ | backup harian 03:00, health check */5 + auto-restart, watch_ebooks */30 |
| 15b. **Web Korpus + Ebook UI** | ✅ | deploy v3.5.4 sukses di srv-fine: 31 ebook terindex (1.245.735 -> 1.245.766 dok), nginx snippet fix (incident v3.5.4), venv Python siap |
| 15c. **Auth + User Management + API Keys** | ✅ | v3.6.0 — lawu-auth :3003, SQLite /var/lib/lawu/auth.db, login dev@lawu.ai.id default, JWT HS256, argon2 hash, user CRUD, API key generate/revoke |
| 15d. **Multi-session Chat + Markdown** | ✅ | v3.6.0 — lawu-chat :3005, SQLite chat_sessions/chat_messages, multi-session seperti chat.z.ai, markdown rendering (marked + DOMPurify) |
| 15e. **Monitoring Real** | ✅ | v3.6.0 — lawu-monitor :3004, sysinfo crate, CPU/RAM/disk/load/service status live, label "(Simulasi)" dihapus |
| 15f. **Ebook Search** | ✅ | v3.6.0 — /api/ebooks/search?q=...&limit=20 (filter docid prefix ebook-), /api/ebooks/:docid detail |
| 15g. **UI Redesign (Emerald)** | ✅ | v3.6.0 — primary ganti #1a73e8 → #10b981 (emerald), @tailwindcss/typography untuk prose, token destructive defined, responsive mobile drawer |
| 16. k6 load test | ✅ | **p(95)=20.32ms** (target <300ms — 14× lebih cepat), 0% error, 1565 reqs dalam 60s, 20 VUs |
| 17. Nginx reverse proxy | ✅ | port 80 → Vue.js static + Axum :3001 + llama :8080 |
| 18. Integrasi Dashboard | ✅ | Vue.js 3 (Vite + Pinia) deploy ke srv-fine via GitHub, nginx serve |
| 19. **Hardening** | ⏳ | **TERAKHIR!** |
| 20. Go-Live | ⏳ | audit final |

**Incident log:**
- 2026-11-03: User terkunci dari server setelah hardening di Langkah 4 lama (SSH password disabled sebelum semua service jalan). Solusi: reinstall OS + susun ulang urutan (v3.1.0 — hardening pindah ke Langkah 18).
- 2026-11-03 (v3.1.1): Saat install ulang OS, HDD 500GB ikut terformat — model GGUF, ebook, dan index hilang. Pelajaran: (1) saat installer minta partisi, PILIH HANYA SSD OS dan pastikan HDD tidak tercentang; (2) Langkah 3.C fstab WAJIB pakai `nofail` + `findmnt --verify` + TEST REBOOT sebelum lanjut — mount manual hilang saat reboot dan fstab salah bisa bikin server gagal boot.
- 2026-11-03 (v3.2.0): Konversi parquet → markdown per-artikel memicu inode exhaustion (file system habis di 119K file, padahal size masih 473MB). Penyebab: `mkfs.ext4 -T largefile4` di Langkah 3.C mengoptimasi untuk file besar tapi mengurangi inode count drastis (4MB/inode → ~119K total inodes). Solusi: ubah strategi dari 600K file `.md` ke 1 file JSONL per split (1 inode per split). Pelajaran: untuk corpus 600K+ dokumen kecil, gunakan JSONL/parquet, BUKAN file individual.
- 2026-11-04 (v3.4.0): REVERT Paten 2 ke Vue.js 3. Penyebab: developer sempat salah ubah Paten 2 dari Vue.js ke Next.js di v3.3.0 (alasan: dashboard dev pakai Next.js), padahal user sudah tegaskan Vue.js adalah Paten permanen. Solusi: revert PRD, hapus dashboard Next.js di srv-fine, rencanakan rebuild dengan Vue.js 3 (Vite + Pinia). Pelajaran #12: **Paten tidak boleh diubah tanpa explicit approval user — walau ada alasan teknis.** Backend Rust + Tantivy + Wikipedia-ID + Q4_K_M tetap.
- 2026-11-04 (v3.5.1): `corpus_api.rs` gagal compile di srv-fine — 5 error (E0308 `Option<&str> == &str`, E0382 borrow of moved value `name`, E0631×3 `map(pid_alive)` u64→u32) karena file tidak pernah di-compile sebelum dikirim. Solusi: install Rust toolchain di sandbox, reproduksi error PERSIS 5/5, perbaiki (banding `== Some(url)`, hitung `indexed` sebelum move, cast `p as u32`), rebuild bersih + smoke test 8 endpoint hijau. Pelajaran #16: Rust source WAJIB compile dulu di sandbox. Ditambah: deploy script pertama dijalankan sebagai root → mencari `/root/lawu-server` (tidak ada). Pelajaran #17: script deploy wajib jalan sebagai user pemilik home-nya.
- 2026-11-04 (v3.5.2): cargo build di srv-fine sukses (21s) tapi script deploy mati — `binary lawu-corpus gagal dibangun!`. Penyebab: Cargo menamai binary dari stem file `src/bin/` → `corpus_api` + `ingest_jsonl`, sedangkan script, unit systemd (ExecStart), dan konstanta `LAWU_INGEST` di corpus_api.rs memanggil `lawu-corpus`/`lawu-ingest`. Solusi: script deploy & install-kit membuat alias copy setelah build (rm+cp, idempoten, aman saat binary lama masih jalan); bonus: ingest di Bagian B diganti `su - lawu -c` (bukan root) agar file Tantivy index tidak jadi milik root + health check retry 5×.
- 2026-11-04 (v3.5.3): venv Python `/opt/lawu/venv` tidak dibuat di Bagian B — `pip: No such file or directory`. Penyebab: script pakai `[ -x $VENV/bin/python3 ] || python3 -m venv` (silent saat sukses, tidak ada feedback). Solusi: buat venv idempoten eksplisit + apt install python3-venv fallback + smoke import 6 deps.
- 2026-11-04 (v3.5.4): browser kirim `/api/corpus/web` dan `/api/ebooks` ke port 80 dapat HTTP 404 padahal API :3002 jalan. Penyebab: B4 deploy pakai `sed -i '/location \/api\/ {/i \...'` (multi-line escape) — saat paste SSH escape collapse jadi 1 baris dengan prefix `#`, nginx baca sebagai komentar. `nginx -t` PASS (comment valid) tapi location block tidak terdaftar. Solusi: tulis location blocks ke file snippet terpisah `/etc/nginx/snippets/lawu-corpus.conf` via `cat <<'NGINXEOF'` (literal heredoc), lalu include-kan via sed satu baris (single-line include, anti-mangle).
- 2026-11-04 (v3.5.5 — Langkah 16 SELESAI): k6 load test **p(95)=20.32ms** (target <300ms — 14× lebih cepat dari SLO), error rate 0.00% (0/1565 reqs gagal), throughput 25.78 req/s stabil selama 60 detik dengan 20 VUs concurrent. Korpus 1.245.766 dokumen (1.38M Wikipedia-ID + 30 ebook + 1 skill) handle beban 20 VUs di CPU-only (i5-10400, 16GB RAM) tanpa sweat — Tantivy memang dirancang untuk sub-25ms query latency. Pelajaran #19: Tantivy di CPU-only bukan bottleneck — bottleneck sebenarnya adalah LLM inference (llama.cpp), bukan search.
- 2026-11-04 (v3.6.0): Dashboard dilengkapi auth + multi-session chat + monitoring real + management. 9 fitur baru dalam 1 rilis (auth_api :3003, chat_api :3005, monitor_api :3004, extend corpus_api :3002 dengan /api/ebooks/search, redesign UI ke emerald, install marked+dompurify+@tailwindcss/typography). SQLite /var/lib/lawu/auth.db dipilih untuk persistensi (vs JSONL — lebih baik untuk query + pagination + index). JWT HS256 dipilih (vs session-cookie — stateless, multi-backend compatible). argon2 dipilih (vs bcrypt — PHC winner, GPU-resistant). Pelajaran #20: dashboard yang tadinya "view-only" berubah jadi aplikasi penuh — butuh auth layer yang cross-cutting (semua backend :3001-3005 pakai middleware verify_jwt yang sama, share SQLite via Arc<Mutex<Connection>>).

---

## 9. Web Korpus Resmi (Paten Sumber Belajar — v3.5.0)

**Prinsip paten user:** Lawu Fine hanya belajar dari sumber **GRATIS dan OPEN SOURCE** —
terhindar dari API / langganan berbayar dan API nakal.

| Kategori | Sumber |
|---|---|
| utama | kernel.org, ubuntu.com, kartolo.sby.datautama.net.id, rust-lang.org (+embedded), vuejs.org, intel.co.id, amd.com, arm.com, armbian.com, onnocenter.or.id wiki, debian.org (+devel-manuals +user-manuals) |
| berita | kumparan.com, kompas.tv/terbaru |
| security | kali.org, parrotsec.org (+docs), docs.rockylinux.org/9 + /10, doddyferdiansyah.web.id/roadmap (roadmap profesi: Pentester, Cyber Security, AI Security Eng, AI Pentesting Eng, Digital Forensic, Full Stack AI Pentesting/Cyber Security Expert), ipfire.org, opnsense.org, zenarmor.com, community.f5.com, fortinet.com, community.cisco.com, mikrotik.com, community.ui.com, idn.id/training (e-learning jaringan), owasp.org, cve.org |
| ai | huggingface.co/zai-org/GLM-5.2 (utama — bedah model), huggingface.co/papers/trending, aws.amazon.com/blogs/big-data |
| custom | data.go.id/dataset, cloudkilat.com, kencengsolusindo.co.id |
| search-engine | google.com/advanced_search, scholar.google.co.id, bing advanced keywords — **wajib kuasai materi Advanced Google Search dulu** (korpus-skill/google-advanced-search.md) |

Pipeline: `web-corpus.lst` + custom URL dari UI → `web_ingest.py` (BFS depth 2, maks 60
halaman/domain, robots.txt, 1 file JSONL) → `lawu-ingest --file web_corpus.jsonl` → Tantivy.
Ebook: upload UI / SCP → `/mnt/data/ebooks` → `extract_ebooks.py` → `ebooks.jsonl` →
`lawu-ingest` → Tantivy (dedup docid otomatis).

---

## 10. Kata Kunci dari Incident (Pelajaran Terbaik)

1. **"Get it running first, then lock it down"** — bukan sebaliknya
2. **Jangan pernah paste block yang mengandung `exit`** — shell jalan tanpa syarat
3. **Sediakan jalur rollback** sebelum hardening (backup sshd_config, dst.)
4. **Test SSH key SEBELUM disable password** — selalu
5. **Dokumentasikan user admin** — siapa yang punya sudo, password-nya apa
6. **Installer Debian: jaga HDD dari partisi otomatis** — centang HANYA SSD OS saat partitioning
7. **Data disk WAJIB fstab `nofail` + test reboot** — mount manual hilang saat reboot;
   fstab salah syntax = server gagal boot (validasi dengan `findmnt --verify` dulu)
8. **`-T largefile4` cocok untuk file besar, BUKAN untuk corpus banyak file kecil** — untuk 600K+ dokumen markdown kecil, gunakan JSONL/parquet (1 inode per split) alih-alih file individual
9. **Saat beralih dari manual run ke systemd service, WAJIB kill instance manual dulu** (`pkill -9 -f binary_name`) — kalau tidak, port conflict membuat service flapping dengan restart counter tinggi
10. **Verifikasi hardware aktual via `free -h` / `lscpu` / `lsblk` sebelum finalisasi PRD** — jangan percaya spec awal 100%. RAM yang dikira 8GB ternyata 16GB → semua tuning konservatif terbukti berlebihan
11. **Terminal SSH konsisten bermasalah dengan heredoc `cat << 'EOF'` panjang** — gunakan `nano` untuk edit file konfigurasi (service, .toml, .rs, .env)
12. **Paten tidak boleh diubah tanpa explicit approval user** — walau ada alasan teknis (misal: dashboard dev pakai Next.js). Paten 2 Vue.js diubah ke Next.js tanpa konfirmasi → revert. Paten adalah kontrak permanen proyek.
13. **Kontrak API backend↔frontend harus eksplisit** — chat gagal tampil di browser padahal curl OK: backend kirim string flat, frontend (ChatView.vue) expect objek `ChatMessage {id, role, content, latencyMs, createdAt}`. Solusi: `#[serde(rename_all = "camelCase")]` + DTO nested. Pelajaran: tulis kontrak DTO di PRD SEBELUM implementasi.
14. **Pekerjaan user tidak boleh sia-sia** — semua aset yang pernah diberikan user (ebook, logo, URL korpus, materi skill) wajib terimplementasi di server asli. Inventaris aset sebelum diputuskan "selesai".
15. **Arsitektur service terpisah = deploy zero-risk** — fitur baru (corpus :3002) dibuat sebagai binary TERPISAH di `src/bin/`, bukan edit `main.rs` produksi. Chat yang sudah jalan tidak pernah terganggu oleh rilis fitur baru.
16. **Compile dulu di sandbox, baru kirim ke server asli** — Rust source yang tidak pernah di-build menyimpan bug tersembunyi (E0308/E0382/E0631). Sandbox wajib punya toolchain yang sama (cargo 1.99.1): reproduksi error → fix → rebuild → smoke test endpoint, SEMUA hijau baru deploy.
17. **Deploy script sadar-user** — script yang menyentuh `$HOME/lawu-server` wajib dijalankan sebagai user `lawu` (bukan root): sebagai root script mencari `/root/lawu-server` dan hasil `cargo build` jadi milik root. Pola aman: `su - lawu -c 'git clone … && bash deploy-from-github.sh'`.
18. **Nama target Cargo = stem file di src/bin/** — cargo menamai binary dari nama file (`corpus_api.rs` → `corpus_api`), BUKAN nama "manis" seperti `lawu-corpus`. Kalau systemd unit / spawn path / script mengharapkan nama lain, buat alias copy di script deploy (`rm -f` + `cp`, idempoten) — verifikasi akhir: `ls target/release/{corpus_api,ingest_jsonl,lawu-corpus,lawu-ingest}` semuanya ada. Bonus: proses ingest index JANGAN dijalankan sebagai root (file index milik root → service lawu gagal tulis).
19. **Heredoc multi-line escape rusak saat paste SSH** — `sed -i '/pattern/i \\\n  line1\n  line2\n  line3'` yang dipaste via SSH sering collapse jadi 1 baris (backslash-newline diapresiasi shell secara beda). Kalau baris pertama diawali `#`, nginx baca sebagai komentar → location block tidak terdaftar, `nginx -t` masih PASS (comment valid syntax). Solusi: tulis konfig ke file snippet terpisah via `cat <<'EOF'` (heredoc quoted = literal, tidak kena escape mangle), lalu include-kan via sed satu baris `sed -i '/anchor/i \    include /path/snippet.conf;'` (single-line statement). Pelajaran: konfigurasi multi-line shell → pakai file snippet + include, JANGAN inline sed escape.
20. **Dashboard penuh butuh auth layer cross-cutting** — saat dashboard berubah dari "view-only" jadi aplikasi penuh (login, multi-session, user mgmt, API key), semua backend butuh middleware auth yang konsisten. Solusi: 1 binary khusus auth (lawu-auth :3003) jadi identity provider, middleware `verify_token` di-share via module `auth-middleware.rs` yang di-include di setiap binary lain. SQLite shared via `Arc<Mutex<Connection>>` di AppState. Token format auto-detect: JWT (`eyJ...`) untuk user login, `lawu_<hex32>` untuk API key (machine-to-machine, untuk Opencode dll). Hindari session-cookie (butuh shared session store = kompleks untuk multi-backend).
