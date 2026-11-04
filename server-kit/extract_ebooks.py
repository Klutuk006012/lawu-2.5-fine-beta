#!/usr/bin/env python3
# ============================================================
# extract_ebooks.py — Lawu-2.5-Fine Ebook Extraction (srv-fine)
# ------------------------------------------------------------
# Extract teks dari semua PDF/EPUB/DOCX/TXT di /mnt/data/ebooks
# ke SATU file JSONL (hemat inode): /mnt/data/processed_markdowns/ebooks.jsonl
# Jalankan: /opt/lawu/venv/bin/python3 /opt/lawu/scripts/extract_ebooks.py
# Log     : stdout (dijalankan manual atau via corpus_api :3002)
# ============================================================
import hashlib
import json
import os
import re
import sys
from datetime import datetime, timezone

EBOOKS_DIR = "/mnt/data/ebooks"
OUT_JSONL = "/mnt/data/processed_markdowns/ebooks.jsonl"
MAX_CHARS = 500_000


def parse_title(filename):
    """Bersihkan nama file jadi judul yang layak baca."""
    t = os.path.splitext(os.path.basename(filename))[0]
    t = re.sub(r"\s*--\s*", " — ", t)
    t = re.sub(r"isbn13\s*\d+.*$", "", t, flags=re.IGNORECASE)
    t = re.sub(r"Anna'?s Archive.*$", "", t, flags=re.IGNORECASE)
    t = re.sub(r"z-?librar?y\.sk.*$", "", t, flags=re.IGNORECASE)
    t = re.sub(r"1lib\.sk.*$", "", t, flags=re.IGNORECASE)
    t = re.sub(r"z-?lib\.sk.*$", "", t, flags=re.IGNORECASE)
    t = re.sub(r"\(.*?\)", "", t)
    t = re.sub(r"[_\s]+", " ", t).strip(" -—.,")
    return t[:200] or os.path.basename(filename)


def make_docid(filename):
    slug = re.sub(r"[^a-z0-9]+", "-", parse_title(filename).lower()).strip("-")[:40]
    h = hashlib.sha1(filename.encode()).hexdigest()[:8]
    return f"ebook-{slug or 'doc'}-{h}"


def extract_pdf(path):
    from pypdf import PdfReader
    reader = PdfReader(path)
    parts = []
    for i, page in enumerate(reader.pages):
        try:
            txt = page.extract_text() or ""
        except Exception:
            txt = ""
        if txt.strip():
            parts.append(txt.strip())
        if sum(len(p) for p in parts) > MAX_CHARS:
            break
    return "\n\n".join(parts)


def extract_epub(path):
    from ebooklib import epub, ITEM_DOCUMENT
    from bs4 import BeautifulSoup
    book = epub.read_epub(path, options={"ignore_ncx": True})
    parts = []
    for item in book.get_items_of_type(ITEM_DOCUMENT):
        soup = BeautifulSoup(item.get_content(), "html.parser")
        for tag in soup(["script", "style", "nav"]):
            tag.decompose()
        txt = soup.get_text(" ", strip=True)
        if txt:
            parts.append(txt)
        if sum(len(p) for p in parts) > MAX_CHARS:
            break
    return "\n\n".join(parts)


def extract_docx(path):
    from docx import Document
    doc = Document(path)
    parts = [p.text for p in doc.paragraphs if p.text.strip()]
    return "\n".join(parts)


def extract_txt(path):
    with open(path, "r", encoding="utf-8", errors="replace") as f:
        return f.read(MAX_CHARS)


EXTRACTORS = {
    ".pdf": extract_pdf,
    ".epub": extract_epub,
    ".docx": extract_docx,
    ".txt": extract_txt,
}


def main():
    os.makedirs(os.path.dirname(OUT_JSONL), exist_ok=True)
    files = []
    for fn in sorted(os.listdir(EBOOKS_DIR)):
        p = os.path.join(EBOOKS_DIR, fn)
        ext = os.path.splitext(fn)[1].lower()
        if fn.startswith(".") or ext not in EXTRACTORS or not os.path.isfile(p):
            continue
        files.append((fn, p, ext))

    print(f"[extract] ditemukan {len(files)} ebook di {EBOOKS_DIR}")
    done, failed = 0, 0
    tmp = OUT_JSONL + ".tmp"
    with open(tmp, "w", encoding="utf-8") as out:
        for i, (fn, p, ext) in enumerate(files, 1):
            print(f"[extract {i}/{len(files)}] {fn}")
            try:
                text = EXTRACTORS[ext](p)
                if len(text.strip()) < 100:
                    raise ValueError("teks terlalu pendek / scan gambar")
                rec = {
                    "docid": make_docid(fn),
                    "title": parse_title(fn),
                    "text": text[:MAX_CHARS],
                    "source_file": fn,
                    "kind": ext.lstrip("."),
                    "extracted_at": datetime.now(timezone.utc).isoformat(),
                }
                out.write(json.dumps(rec, ensure_ascii=False) + "\n")
                done += 1
            except Exception as e:
                failed += 1
                print(f"  [GAGAL] {e.__class__.__name__}: {e}")
    os.replace(tmp, OUT_JSONL)
    print(f"[extract] SELESAI — {done} sukses, {failed} gagal -> {OUT_JSONL}")
    if done > 0:
        print("[extract] Lanjutkan dengan:")
        print(f"  ~/lawu-server/target/release/lawu-ingest --file {OUT_JSONL}")
    return 0 if done > 0 else 1


if __name__ == "__main__":
    sys.exit(main())
