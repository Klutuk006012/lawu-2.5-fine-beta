#!/usr/bin/env python3
# ============================================================
# web_ingest.py — Crawler Web Korpus Lawu-2.5-Fine (srv-fine)
# ------------------------------------------------------------
# Fungsi  : crawl semua URL di web-corpus.lst + custom URL dari
#           UI (web_corpus_custom.jsonl), extract teks, simpan ke
#           SATU file JSONL (hemat inode), siap di-ingest Tantivy.
# Jalankan: /opt/lawu/venv/bin/python3 /opt/lawu/scripts/web_ingest.py
# Log     : /var/log/lawu/web_ingest.log
# Output  : /mnt/data/processed_markdowns/web_corpus.jsonl
# State   : /var/lib/lawu/web_corpus_state.json
# ============================================================
import json
import os
import re
import sys
import time
import hashlib
import logging
from collections import deque
from datetime import datetime, timedelta
from urllib.parse import urljoin, urlsplit
from urllib.robotparser import RobotFileParser

import requests
from bs4 import BeautifulSoup

# ----------------------------- Konfigurasi -----------------------------
BASE_DIR      = "/mnt/data/processed_markdowns"
OUTPUT_JSONL  = os.path.join(BASE_DIR, "web_corpus.jsonl")
CORPUS_LST    = "/etc/lawu/web-corpus.lst"
CUSTOM_JSONL  = "/var/lib/lawu/web_corpus_custom.jsonl"
STATE_FILE    = "/var/lib/lawu/web_corpus_state.json"
LOG_FILE      = "/var/log/lawu/web_ingest.log"

MAX_PAGES_PER_DOMAIN   = 60      # batas halaman per domain per sesi crawl
MAX_DEPTH              = 2       # kedalaman BFS dari seed URL
POLITENESS_DELAY       = 1.0     # detik antar request ke domain yang sama
PAGE_TIMEOUT           = 20      # detik
MAX_HTML_BYTES         = 2_000_000
MIN_TEXT_LEN           = 120     # halaman dengan teks lebih pendek = skip
RESTART_AFTER_HOURS    = 24      # URL sudah dicrawl < 24 jam = skip (rescan)
UA                     = "LawuFine-CorpusBot/1.0 (+CPU AI research server; lawu@srv-fine)"

SKIP_EXT = re.compile(
    r"\.(png|jpe?g|gif|svg|ico|webp|css|js|mjs|zip|gz|tgz|bz2|xz|7z|rar|"
    r"mp3|mp4|mkv|avi|mov|flac|ogg|woff2?|ttf|eot|exe|dmg|iso|deb|rpm|"
    r"pdf|epub|docx?|xlsx?|pptx?)($|\?)", re.IGNORECASE)

HEADERS = {"User-Agent": UA, "Accept-Language": "id-ID,id;q=0.9,en;q=0.8"}

# ----------------------------- Logging ---------------------------------
os.makedirs(os.path.dirname(LOG_FILE), exist_ok=True)
os.makedirs(BASE_DIR, exist_ok=True)
os.makedirs(os.path.dirname(STATE_FILE), exist_ok=True)
logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(message)s",
    handlers=[logging.FileHandler(LOG_FILE), logging.StreamHandler(sys.stdout)],
)
log = logging.getLogger("lawu-web-ingest")


# ----------------------------- Helpers ---------------------------------
def load_seed_urls():
    """Baca seed dari web-corpus.lst (kategori<TAB>url, '#' = komentar)."""
    seeds = []
    if not os.path.exists(CORPUS_LST):
        log.warning("Seed list tidak ada: %s", CORPUS_LST)
        return seeds
    with open(CORPUS_LST, "r", encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line or line.startswith("#"):
                continue
            parts = line.split(None, 1)
            if len(parts) == 2:
                seeds.append((parts[0].strip(), parts[1].strip()))
    return seeds


def load_custom_urls():
    """Baca custom URL yang ditambahkan via UI (corpus_api :3002)."""
    entries = []
    if not os.path.exists(CUSTOM_JSONL):
        return entries
    try:
        with open(CUSTOM_JSONL, "r", encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if not line:
                    continue
                try:
                    obj = json.loads(line)
                    if obj.get("url"):
                        entries.append((obj.get("category", "custom"), obj["url"]))
                except json.JSONDecodeError:
                    continue
    except OSError:
        pass
    return entries


def load_state():
    try:
        with open(STATE_FILE, "r", encoding="utf-8") as f:
            return json.load(f)
    except (OSError, json.JSONDecodeError):
        return {}


def save_state(state):
    tmp = STATE_FILE + ".tmp"
    with open(tmp, "w", encoding="utf-8") as f:
        json.dump(state, f, indent=1, ensure_ascii=False)
    os.replace(tmp, STATE_FILE)


def domain_of(url):
    n = urlsplit(url).netloc.lower()
    return n[4:] if n.startswith("www.") else n


def slugify(text):
    text = re.sub(r"[^a-z0-9]+", "-", (text or "web").lower()).strip("-")
    return text[:40] or "web"


def extract_text(html, base_url):
    """Extract title + teks bersih dari HTML."""
    soup = BeautifulSoup(html, "html.parser")
    for tag in soup(["script", "style", "noscript", "nav", "footer", "header",
                     "aside", "form", "iframe", "button", "svg"]):
        tag.decompose()
    main = (soup.find("main") or soup.find("article")
            or soup.find(role="main") or soup.body or soup)
    title = ""
    if soup.title and soup.title.string:
        title = soup.title.string.strip()
    if not title:
        h1 = soup.find("h1")
        if h1:
            title = h1.get_text(" ", strip=True)[:200]
    text = main.get_text(" ", strip=True)
    text = re.sub(r"\s{3,}", "  ", text).strip()
    links = set()
    for a in soup.find_all("a", href=True):
        href = urljoin(base_url, a["href"].split("#")[0]).rstrip("/")
        if href.startswith(("http://", "https://")):
            links.add(href)
    return title[:300], text, links


def robots_allows(url, session, cache={}):
    d = domain_of(url)
    if d in cache:
        rp = cache[d]
    else:
        scheme = urlsplit(url).scheme or "https"
        rp = RobotFileParser()
        rp.set_url(f"{scheme}://{d}/robots.txt")
        try:
            r = session.get(f"{scheme}://{d}/robots.txt", timeout=8,
                            headers=HEADERS)
            if r.status_code == 200 and r.text:
                rp.parse(r.text.splitlines())
            else:
                rp.allow_all = True
        except requests.RequestException:
            rp.allow_all = True
        cache[d] = rp
    try:
        return rp.can_fetch(UA, url)
    except Exception:
        return True


def crawl_seed(category, seed_url, session, state, jsonl_f, stats):
    """BFS satu domain mulai dari seed URL."""
    dom = domain_of(seed_url)
    now = datetime.utcnow()
    st = state.get(seed_url, {})
    last = st.get("crawled_at")
    if last:
        try:
            dt = datetime.fromisoformat(last)
            if now - dt < timedelta(hours=RESTART_AFTER_HOURS) and not state.get("_force"):
                log.info("[SKIP-24h] %s", seed_url)
                return
        except ValueError:
            pass

    if not robots_allows(seed_url, session):
        log.info("[ROBOTS-DENY] %s", seed_url)
        return

    q = deque([(seed_url, 0)])
    seen = {seed_url}
    pages_ok, pages_fail = 0, 0

    while q:
        url, depth = q.popleft()
        try:
            r = session.get(url, timeout=PAGE_TIMEOUT, headers=HEADERS,
                            stream=True)
            ctype = r.headers.get("Content-Type", "")
            if "html" not in ctype.lower():
                r.close()
                continue
            clen = int(r.headers.get("Content-Length") or MAX_HTML_BYTES)
            if clen > MAX_HTML_BYTES:
                r.close()
                continue
            html = r.raw.read(MAX_HTML_BYTES + 1, decode_content=True)
            r.close()
            html = html.decode(r.encoding or "utf-8", errors="replace")[:MAX_HTML_BYTES]
        except requests.RequestException as e:
            log.info("[FAIL] %s -> %s", url, e.__class__.__name__)
            pages_fail += 1
            continue

        title, text, links = extract_text(html, url)
        if len(text) < MIN_TEXT_LEN:
            continue

        docid = "web-{}-{}".format(slugify(dom), hashlib.sha1(url.encode()).hexdigest()[:8])
        rec = {
            "docid": docid,
            "title": title or dom,
            "text": text,
            "url": url,
            "domain": dom,
            "category": category,
            "crawled_at": datetime.utcnow().isoformat() + "Z",
        }
        jsonl_f.write(json.dumps(rec, ensure_ascii=False) + "\n")
        pages_ok += 1
        state[url] = {"crawled_at": rec["crawled_at"], "category": category,
                      "title": rec["title"], "chars": len(text)}
        log.info("[OK %d/%d d%d] %s (%d chars)", pages_ok, MAX_PAGES_PER_DOMAIN,
                 depth, url, len(text))

        if depth < MAX_DEPTH and pages_ok < MAX_PAGES_PER_DOMAIN:
            for link in links:
                if len(seen) >= MAX_PAGES_PER_DOMAIN * 3:
                    break
                if link in seen or SKIP_EXT.search(link):
                    continue
                if domain_of(link) != dom:
                    continue
                seen.add(link)
                q.append((link, depth + 1))

        time.sleep(POLITENESS_DELAY)

    stats["pages"] += pages_ok
    stats["fail"] += pages_fail
    log.info("[SEED-DONE] %s -> %d halaman, %d gagal", seed_url, pages_ok, pages_fail)


def main():
    from_ts = datetime.utcnow().isoformat()
    log.info("=== Lawu Fine web_ingest mulai %s ===", from_ts)
    session = requests.Session()
    session.headers.update(HEADERS)

    state = load_state()
    state.pop("_force", None)
    seeds = load_seed_urls() + load_custom_urls()
    log.info("Total seed URL: %d (seed resmi + custom UI)", len(seeds))

    stats = {"pages": 0, "fail": 0, "seeds": len(seeds)}
    os.makedirs(BASE_DIR, exist_ok=True)
    tmp_out = OUTPUT_JSONL + ".tmp"
    with open(tmp_out, "w", encoding="utf-8") as jsonl_f:
        for i, (cat, url) in enumerate(seeds, 1):
            log.info("--- [%d/%d] kategori=%s %s", i, len(seeds), cat, url)
            try:
                crawl_seed(cat, url, session, state, jsonl_f, stats)
            except Exception as e:
                log.error("[SEED-ERROR] %s -> %s: %s", url, e.__class__.__name__, e)
                continue
            save_state(state)  # simpan progres tiap seed

    os.replace(tmp_out, OUTPUT_JSONL)
    done_ts = datetime.utcnow().isoformat()
    log.info("=== SELESAI %s | halaman=%d gagal=%d output=%s ===",
             done_ts, stats["pages"], stats["fail"], OUTPUT_JSONL)
    state["_last_run"] = {"finished_at": done_ts, "pages": stats["pages"],
                          "fail": stats["fail"], "seeds": stats["seeds"]}
    save_state(state)


if __name__ == "__main__":
    # Dukungan force re-crawl: python3 web_ingest.py --force
    if "--force" in sys.argv:
        import json as _j
        try:
            with open(STATE_FILE) as f:
                s = _j.load(f)
            s["_force"] = True
            with open(STATE_FILE, "w") as f:
                _j.dump(s, f)
        except OSError:
            pass
    main()
