# Qdrant Evaluation Plan — Lawu-2.5-Fine v3.7.0+

> Status: **DRAFT** untuk v3.7.0. Tidak ada kode yang dipasang sekarang —
> dokumen ini menjelaskan roadmap integrasi Qdrant (vector database) untuk
> mengganti / melengkapi pencarian Tantivy BM25 yang sudah berjalan.
>
> Paten 3 (CPU-only): semua komponen harus berjalan tanpa GPU.

## 1. Apa itu Qdrant?

[Qdrant](https://qdrant.tech/) adalah **vector search engine** berbasis Rust
(license Apache 2.0 — memenuhi Paten 5 open source). Qdrant menyimpan vektor
berdimensi tinggi (mis. 1024 dim dari embedding model BGE-m3) beserta payload
JSON, lalu menjawab query "top-k nearest neighbor" dengan cosine / dot-product /
euclidean distance secara efisien menggunakan HNSW (Hierarchical Navigable
Small World graph).

Berbeda dengan Tantivy (BM25 keyword match), Qdrant melakukan **semantic
search**: pertanyaan "apa itu kecerdasan buatan?" dapat menemukan dokumen yang
membahas "AI / artificial intelligence" tanpa harus mengandung kata yang sama
persis.

## 2. Kenapa Lawu butuh Qdrant?

| Aspek                     | Tantivy (sekarang)             | Qdrant (rencana)                          |
|---------------------------|--------------------------------|-------------------------------------------|
| Cara match                | BM25 keyword (token overlap)   | Vector cosine similarity (semantic)       |
| Cocok untuk               | Exact keyword, judul, kode     | Pertanyaan natural, parafase, multi-bahasa|
| Recall pada parafrase     | Lemah (miss kalau kata beda)   | Kuat (cosine similarity tetap tinggi)     |
| Resource                  | Disk + RAM kecil (~50MB RAM)   | RAM ~512MB-2GB, disk proporsional vector  |
| Latency                   | <5ms untuk 1.2M doc            | 5-30ms untuk 1.2M doc                     |
| Indexing speed            | Cepat (~5k doc/s)              | Lama (~500 doc/s dengan embedding)        |
| Model yang dipakai        | Tokenizer + BM25              | Embedding model (BGE-m3, 1024 dim)        |
| License                   | MIT (Tantivy 0.24)             | Apache 2.0 (Qdrant)                       |

**Use case utama Lawu**: jawaban LLM jauh lebih akurat bila RAG bisa menemukan
dokumen yang **secara semantik** relevan, bukan hanya yang match keyword.
Contoh: user tanya "cara mengamankan server Linux" — Tantivy akan miss dokumen
"hardening Linux" karena kata "mengamankan" ≠ "hardening". Qdrant akan match.

## 3. Cara install Qdrant di Debian 13 (Trixie)

### Opsi A: Docker (rekomendasi untuk cepat)

```bash
sudo apt-get install -y docker.io docker-compose-v2
sudo mkdir -p /var/lib/qdrant/storage /var/lib/qdrant/snapshots
sudo chown -R 1000:1000 /var/lib/qdrant   # qdrant container UID 1000

# /etc/qdrant/config.yaml (lihat docs.qdrant.tech untuk opsi lengkap)
sudo tee /etc/qdrant/config.yaml <<EOF
storage:
  storage_path: /var/lib/qdrant/storage
  snapshots_path: /var/lib/qdrant/snapshots
service:
  host: 127.0.0.1
  http_port: 6333
  grpc_port: 6334
telemetry_disabled: true
EOF

# /etc/systemd/system/qdrant.service
sudo tee /etc/systemd/system/qdrant.service <<EOF
[Unit]
Description=Qdrant Vector Search Engine
After=network.target
[Service]
Type=simple
User=lawu
ExecStart=/usr/bin/docker run --rm --name qdrant \
  -p 127.0.0.1:6333:6333 -p 127.0.0.1:6334:6334 \
  -v /var/lib/qdrant:/qdrant/storage \
  -v /etc/qdrant/config.yaml:/qdrant/config/config.yaml:ro \
  qdrant/qdrant:v1.12.4
Restart=always
RestartSec=5
[Install]
WantedBy=multi-user.target
EOF
sudo systemctl enable --now qdrant
```

### Opsi B: Binary native (tanpa Docker)

```bash
# Qdrant publish binary static musl untuk x86_64.
cd /tmp
wget https://github.com/qdrant/qdrant/releases/download/v1.12.4/qdrant-x86_64-unknown-linux-gnu-libc.tar.gz
tar xzf qdrant-x86_64-unknown-linux-gnu-libc.tar.gz
sudo mv qdrant /usr/local/bin/qdrant
sudo useradd -r -s /usr/sbin/nologin -d /var/lib/qdrant qdrant
sudo mkdir -p /var/lib/qdrant/storage /var/lib/qdrant/snapshots
sudo chown -R qdrant:qdrant /var/lib/qdrant
# pakai systemd unit yang sama, ExecStart=/usr/local/bin/qdrant --config-path /etc/qdrant/config.yaml
```

Pastikan Qdrant **hanya listen 127.0.0.1** (bukan 0.0.0.0) — nginx proxy
akses publik, dan Qdrant tidak ter-expose langsung.

## 4. Integrasi dengan Lawu

### 4.1 Embedding model

Dua opsi, keduanya CPU-only:

**Opsi A (rekomendasi v3.7.0): `llama.cpp` /v1/embeddings**

`llama-server` (sudah running di `:8080` untuk Qwen2.5-Coder-7B Q4_K_M)
dapat juga memuat **BGE-m3** sebagai embedding model terpisah:

```bash
# Spawn instance kedua llama-server khusus embedding, port 8081
/home/lawu/llama.cpp/build/bin/llama-server \
  --model /mnt/data/models/bge-m3-Q8_0.gguf \
  --embedding --port 8081 --host 127.0.0.1 \
  --threads 8 --ctx-size 8192
```

API:
```
POST http://127.0.0.1:8081/v1/embeddings
{ "model": "bge-m3", "input": ["teks yang di-embed"] }
→ { "data": [{ "embedding": [0.0123, -0.0456, ...1024 floats...] }] }
```

Dimensi vektor BGE-m3: **1024**. Multilingual (Indonesia + English).

**Opsi B (alternatif v3.8.0+): `fastembed-rs` crate (Rust native)**

`fastembed = "4"` adalah crate Rust murni (Apache 2.0 / MIT) yang memakai ONNX
Runtime CPU. Tidak butuh llama-server instance terpisah, embedding dihitung
in-process di dalam `training_api.rs` / `chat_api.rs`:

```toml
fastembed = "4"
```

```rust
use fastembed::{InitOptions, TextEmbedding};
let model = TextEmbedding::try_new(InitOptions::new(
    fastembed::EmbeddingModel::BGEBaseENV15   // dim 768, atau BGE-large = 1024
))?;
let embeddings: Vec<Vec<f32>> = model.embed(vec!["teks"], None)?;
```

Trade-off: lebih lambat dari llama.cpp (CPU ONNX vs GGML Q8), tetapi lebih
simple deployment (no second process). **Direkomendasikan untuk korpus < 100k
dokumen**.

### 4.2 Storage layout

```
Qdrant collection:  lawu_corpus
  vector size:      1024 (BGE-m3)
  distance:         Cosine
  on-disk:          true              # hemat RAM
  payload schema:
    docid:          keyword (string)
    source:         keyword ("wikipedia" | "ebook" | "web_crawl" | "skill" | "custom_upload")
    category:       keyword
    title:          text
    text_preview:   text (first 200 chars)
    created_at:     datetime
    tantivy_docid:  keyword           # cross-reference untuk hybrid search
```

```bash
# Buat collection (curl, idempotent)
curl -X PUT http://127.0.0.1:6333/collections/lawu_corpus \
  -H 'Content-Type: application/json' \
  -d '{
    "vectors": {
      "size": 1024,
      "distance": "Cosine",
      "on_disk": true
    },
    "optimizers_config": {
      "default_segment_number": 4,
      "indexing_threshold": 50000
    }
  }'
```

### 4.3 Workflow RAG hibrida (Phase 3, v3.9.0)

```
User question (HTTP POST /api/chat)
   │
   ├─ 1. Tantivy BM25 search  → top-N docids  (cepat, keyword match)
   ├─ 2. Qdrant semantic search → top-N docids (kuat untuk parafase)
   ├─ 3. Union + re-rank by (BM25_score * 0.4 + cosine_sim * 0.6)
   ├─ 4. Take top-K (K=5), ambil text dari Tantivy
   └─ 5. Inject sebagai context ke prompt LLM
```

## 5. Migration plan

### Phase 1 (v3.7.0): PoC Qdrant + 30 ebook

- [ ] Install Qdrant via binary native di `/usr/local/bin/qdrant`
- [ ] Spawn embedding model `bge-m3` via `llama-server :8081`
- [ ] Buat collection `lawu_corpus` (vector 1024, cosine, on_disk=true)
- [ ] Extend `ingest_jsonl.rs` dengan flag `--embeddings-url http://127.0.0.1:8081/v1/embeddings`
      yang untuk tiap dokumen menghitung embedding + upsert ke Qdrant
- [ ] Bulk embed **30 ebook** (sudah ada di `/mnt/data/processed_markdowns/ebooks.jsonl`)
- [ ] Tes recall: bandingkan top-5 Tantivy vs top-5 Qdrant untuk 10 pertanyaan sample
- [ ] Resource budget: Qdrant < 256MB RAM, llama-server embedder < 1GB CPU/RAM
- [ ] Update `training_api.rs /api/training/evaluations/run` untuk log both scores

### Phase 2 (v3.8.0): Embeddings 1.245.903 dokumen Wikipedia

- [ ] Skrip batch offline `embed_wikipedia_offline.py` / Rust CLI:
      baca `wikipedia-id-train.jsonl` per chunk (10k doc), POST ke `:8081/v1/embeddings`
      dengan batch 32, upsert ke Qdrant.
- [ ] Estimasi waktu: 1.2M doc × ~50ms/embed / 8 threads = ~2 jam
- [ ] Disk: 1.2M × 1024 × 4 byte (float32) = **~5 GB** raw vector, ~3 GB dengan kuantisasi
- [ ] RAM peak: Qdrant HNSW index ~2 GB (cold), on-disk swap aman
- [ ] Schedule weekly rebuild via `training_schedule` (action `embed_wikipedia`)

### Phase 3 (v3.9.0): Hybrid search Tantivy + Qdrant di `main.rs`

- [ ] Endpoint baru `/api/search?hybrid=true&q=...&limit=10`
- [ ] Implement rerank: `combined_score = 0.4 * bm25_norm + 0.6 * cosine_sim`
- [ ] Fallback otomatis: kalau Qdrant down, pakai Tantivy saja (graceful degradation)
- [ ] Update `chat_api.rs` agar RAG context dari hybrid search
- [ ] A/B evaluation: 50 pertanyaan, bandingkan jawaban LLM dengan BM25-only vs hybrid

## 6. Resource impact

| Komponen                   | CPU        | RAM (idle) | RAM (peak)  | Disk               |
|----------------------------|------------|------------|-------------|--------------------|
| Qdrant daemon (Phase 1, 30 ebook)     | ~0%      | ~80 MB     | ~150 MB     | ~5 MB              |
| Qdrant daemon (Phase 2, 1.2M doc)     | ~0%      | ~512 MB    | ~1.5 GB     | ~5 GB              |
| llama-server embedder (BGE-m3 Q8)     | 1 core   | ~700 MB    | ~1.2 GB     | ~600 MB (model)    |
| `fastembed-rs` in-process (alternatif)| 1-2 core | ~600 MB    | ~1.5 GB     | ~1.5 GB (model)   |

Server srv-fine (asumsi 8 vCPU / 16 GB RAM / 100 GB SSD) **cukup** untuk
Phase 2. Bila ingin Phase 3 dengan rerank realtime di setiap chat, perlu
tambahan throttling (batch embedding queue, cache 5-menit).

## 7. Trade-off vs Tantivy (ringkasan)

- **Kelebihan Qdrant**: recall tinggi pada pertanyaan natural/parafase,
  multilingual (BGE-m3 cross-lingual ID-EN), filter payload fleksibel
  (category, source, date range).
- **Kekurangan Qdrant**: lebih lambat dari BM25 untuk exact keyword match
  (mis. cari `docid="wiki-12345"`), butuh resource RAM lebih besar,
  embedding model terpisah menambah point-of-failure.
- **Solusi**: **hybrid** (Phase 3) — pakai keduanya, combine score. Tantivy
  tetap jadi source-of-truth untuk full-text + doc retrieval; Qdrant jadi
  semantic layer untuk rerank.

## 8. Risiko & mitigasi

| Risiko                                   | Mitigasi                                             |
|------------------------------------------|------------------------------------------------------|
| Qdrant crash korupsi index               | Snapshot nightly ke `/var/lib/qdrant/snapshots`     |
| Embedding model versi drift              | Pin GGUF hash + simpan metadata `embedder_version`  |
| Embedding untuk 1.2M doc lama            | Batch offline, throttling 8 thread, checkpoint tiap 10k |
| RAM OOM saat peak (chat + embed)         | `systemd MemoryMax=4G` untuk qdrant.service         |
| Cosine similarity bias untuk teks pendek | Filter dokumen < 50 kata keluar dari candidate pool  |

## 9. Open questions (perlu diskusi sebelum Phase 1)

1. Apakah BGE-m3 (multilingual) cukup, atau perlu model Indonesia-spesifik
   seperti `indobenchmark/indobert-base`? BGE-m3 lebih general-purpose dan
   sudah teruji di MTEB. Rekomendasi awal: BGE-m3.
2. Apakah perlu re-embed saat prompt chat_api berubah? **Tidak** — embedding
   dokumen independen dari prompt chat.
3. Apakah Qdrant perlu HA (replica 3)? **Tidak untuk v3.7-3.9** — single-node
   cukup, snapshot harian untuk recovery.
