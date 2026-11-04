// ============================================================
// ingest_jsonl.rs — Lawu-2.5-Fine JSONL → Tantivy Ingest
// ------------------------------------------------------------
// Memasukkan dokumen dari JSONL (hasil extract_ebooks.py atau
// web_ingest.py) ke index Tantivy yang LIVE (sama dengan main.rs).
// Dedup otomatis: docid yang sudah ada di index tidak ditulis ulang.
//
// Build  : cargo build --release  (di ~/lawu-server)
// Pakai  : ~/lawu-server/target/release/lawu-ingest \
//            --file /mnt/data/processed_markdowns/web_corpus.jsonl \
//            [--index /mnt/data/tantivy_index/wikipedia-id-train] \
//            [--dry-run]
// ============================================================
use serde::Deserialize;
use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use tantivy::{
    collector::Count,
    index::{Index, IndexReader},
    query::{AllQuery, TermQuery},
    schema::{IndexRecordOption, Schema, Value},
    IndexWriter, ReloadPolicy, Term,
};

#[derive(Deserialize, Debug)]
struct JsonDoc {
    docid: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    text: String,
    // field lain (url, category, domain, dst.) diabaikan — index live
    // hanya menyimpan title/text/docid agar konsisten dengan main.rs
}

fn open_index(index_dir: &PathBuf) -> Result<(Index, Schema, IndexReader), Box<dyn Error>> {
    let index = Index::open_in_dir(index_dir)
        .map_err(|e| format!("Gagal buka index di {:?}: {e}", index_dir))?;
    let schema = index.schema();
    let reader: IndexReader = index
        .reader_builder()
        .reload_policy(ReloadPolicy::Manual)
        .try_into()
        .map_err(|e| format!("Gagal buat reader: {e:?}"))?;
    Ok((index, schema, reader))
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut jsonl_path: Option<PathBuf> = None;
    let mut index_dir = PathBuf::from("/mnt/data/tantivy_index/wikipedia-id-train");
    let mut dry_run = false;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--file" => {
                jsonl_path = args.next().map(PathBuf::from);
            }
            "--index" => {
                index_dir = args.next().map(PathBuf::from).unwrap_or(index_dir);
            }
            "--dry-run" => dry_run = true,
            other => return Err(format!("Argumen tidak dikenal: {other}").into()),
        }
    }

    let jsonl_path = jsonl_path
        .ok_or_else(|| "wajib: --file /path/ke/korpus.jsonl")?;
    if !jsonl_path.exists() {
        return Err(format!("file tidak ada: {:?}", jsonl_path).into());
    }

    eprintln!("[lawu-ingest] index   : {}", index_dir.display());
    eprintln!("[lawu-ingest] jsonl    : {}", jsonl_path.display());
    eprintln!("[lawu-ingest] dry-run  : {dry_run}");

    // ---- baca & validasi JSONL dulu ----
    let file = File::open(&jsonl_path)?;
    let reader = BufReader::new(file);
    let mut docs = Vec::new();
    let mut bad_lines = 0usize;
    let mut seen_in_file: std::collections::HashSet<String> = Default::default();
    for line in reader.lines() {
        let line = line?;
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        match serde_json::from_str::<JsonDoc>(line) {
            Ok(d) => {
                if seen_in_file.insert(d.docid.clone()) {
                    docs.push(d);
                }
            }
            Err(_) => bad_lines += 1,
        }
    }
    eprintln!(
        "[lawu-ingest] jsonl dibaca: {} dokumen valid, {bad_lines} baris dibuang",
        docs.len()
    );

    // ---- buka index & deteksi field by name (robust terhadap skema live) ----
    let (mut index, schema, reader) = open_index(&index_dir)?;
    let field_title = schema
        .get_field("title")
        .ok_or_else(|| "field 'title' tidak ditemukan di skema index")?;
    let field_text = schema
        .get_field("text")
        .ok_or_else(|| "field 'text' tidak ditemukan di skema index")?;
    let field_docid = schema
        .get_field("docid")
        .ok_or_else(|| "field 'docid' tidak ditemukan di skema index")?;

    let searcher = reader.searcher();
    let total_existing = searcher.search(&AllQuery, &Count)?;
    eprintln!("[lawu-ingest] dokumen existing di index: {total_existing}");

    // ---- dedup terhadap index: cek per docid via TermQuery ----
    let mut new_docs = Vec::new();
    let mut skipped = 0usize;
    for d in docs.into_iter() {
        let term = Term::from_field_text(field_docid, &d.docid);
        let q = TermQuery::new(term, IndexRecordOption::Basic);
        let hits = searcher.search(&q, &Count)?;
        if hits == 0 {
            new_docs.push(d);
        } else {
            skipped += 1;
        }
    }
    eprintln!(
        "[lawu-ingest] dedup selesai: {} dokumen baru, {skipped} sudah ada (skip)",
        new_docs.len()
    );

    if dry_run {
        eprintln!("[lawu-ingest] DRY-RUN — tidak ada yang ditulis. Selesai.");
        return Ok(());
    }
    if new_docs.is_empty() {
        eprintln!("[lawu-ingest] Tidak ada dokumen baru. Selesai.");
        return Ok(());
    }

    // ---- tulis ----
    let mut writer: IndexWriter = index.writer(64 * 1024 * 1024)?;
    for d in &new_docs {
        let mut doc = tantivy::Document::default();
        doc.add_text(field_title, &d.title);
        // truncate text panjang agar aman (500K chars sama dengan pipeline ebook)
        let text = if d.text.len() > 500_000 { &d.text[..500_000] } else { &d.text[..] };
        doc.add_text(field_text, text);
        doc.add_text(field_docid, &d.docid);
        writer.add_document(doc)?;
    }
    writer.commit()?;
    eprintln!("[lawu-ingest] commit: {} dokumen ditulis", new_docs.len());

    // ---- verifikasi ringan ----
    let reader2 = index
        .reader_builder()
        .reload_policy(ReloadPolicy::Manual)
        .try_into()?;
    let s2 = reader2.searcher();
    let total_now = s2.search(&AllQuery, &Count)?;
    eprintln!(
        "[lawu-ingest] SELESAI — total dokumen index sekarang: {total_now} (sebelum: {total_existing})"
    );

    let _ = &schema; // keep
    Ok(())
}
