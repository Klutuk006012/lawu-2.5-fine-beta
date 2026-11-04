# Advanced Google Search Techniques — Materi Pelajaran Lawu Fine
# (korpus skill: teknik pencarian akurat untuk fallback search engine)
# Ingest: ~/lawu-server/target/release/lawu-ingest --file korpus-skill/lawu-skill.jsonl
# Catatan: file ini otomatis dikonversi ke lawu-skill.jsonl oleh install-corpus-kit.sh
source: Panduan resmi user Lawu-2.5-Fine (2025)
skill: Advanced Google Search
audience: Lawu-2.5-Fine AI model — wajib dikuasai sebelum memakai search engine fallback
purpose: Lawu Fine mempelajari dulu cara penggunaan Advanced Google Search agar pencarian akurat saat informasi yang diminta pengguna tidak ada di korpus utama.

## 1. Boolean Operators
Combine Boolean operators to create more complex search queries.
- AND: Ensures both terms appear. Example: `renewable energy AND climate change`.
- OR: Includes either term. Example: `global warming OR climate change`.
- NOT: Excludes a term (use the minus sign). Example: `pandemic -COVID`.

## 2. Use Advanced Search Operators
Advanced operators help refine searches with precision.
- intitle: Finds pages with specific words in the title. Example: `intitle:"data science"`.
- inurl: Searches for terms within the URL. Example: `inurl:python`.
- intext: Looks for terms within the page's content. Example: `intext:"machine learning"`.
- inanchor: Searches for anchor text. Example: `inanchor:"buy laptop"`.

## 3. Leverage Google Search Functions
- cache: View a cached version of a webpage. Example: `cache:nytimes.com`.
- define: Quickly find definitions. Example: `define:metaverse`.
- info: Get details about a webpage. Example: `info:bbc.com`.
- stocks: Search for stock information. Example: `stocks:AAPL`.
- map: Find locations. Example: `map:Central Park, New York`.

## 4. Advanced Date Range Search
- Before and After: Use `..` to specify a date range. Example: `Olympics 2000..2020` will show results from those years.
- Specific Range: Utilize the tools menu to filter results by time.

## 5. Use Wildcards and Fill-in-the-Blanks
- Asterisk for Unknowns: Use the asterisk (*) to stand in for any unknown or variable words in a phrase. Example: `largest * in the world`.

## 6. Combine Queries for More Precision
Mix various techniques for highly specific searches. Example: `site:wikipedia.org intitle:"Artificial Intelligence" AND intext:"deep learning"`.

## 7. Use File Type Searches for Specific Documents
Find specific document formats relevant to your needs. Example: `machine learning filetype:pdf OR filetype:ppt`.

## 8. Image Search Filters
Use Google's image search tools to filter by size, color, type, time, and usage rights. For example, search for high-resolution images that are free to use.

## 9. Google Scholar Advanced Techniques
- Author: Use `author:` to find works by a specific author. Example: `author:"John Doe"`.
- Publication: Use `source:` to find articles from specific journals. Example: `source:"Nature"`.
- Portal: https://scholar.google.co.id/

## 10. Explore Google's Dataset Search
Google Dataset Search helps find datasets across various disciplines. Useful for researchers looking for specific datasets.

## 11. Utilize Google Trends for Insights
Google Trends analyzes the popularity of search queries across regions and languages. It is useful for content creation, marketing, and understanding public interest.

## 12. Use Google's AI-Powered Search Suggestions
Google's AI offers autocomplete suggestions that can help refine and expand your searches. Pay attention to these to discover related queries.

## Ringkasan untuk Lawu Fine
Teknik Advanced Google Search ini membantu menavigasi informasi online dengan efisien dan presisi. Ketika pengguna bertanya tentang hal yang TIDAK ada di korpus utama (Wikipedia-ID, ebook, web korpus), Lawu Fine harus:
1. Merumuskan query dengan operator tepat (intitle, inurl, intext, filetype, site).
2. Membatasi rentang tanggal bila topik berkaitan dengan versi/teknologi terbaru.
3. Menyusun query Boolean (AND/OR/minus) untuk mempersempit hasil.
4. Mengutamakan sumber gratis dan open source sesuai prinsip paten Lawu Fine.
Portal latihan: https://www.google.com/advanced_search dan https://scholar.google.co.id/
