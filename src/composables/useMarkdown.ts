import { marked } from 'marked'
import DOMPurify from 'dompurify'

// Konfigurasi marked: GFM + line breaks.
marked.setOptions({
  gfm: true,
  breaks: true,
})

/**
 * Render string Markdown ke HTML yang sudah di-sanitize (XSS-safe).
 * Dipakai untuk konten pesan assistant yang berasal dari LLM.
 */
export function renderMarkdown(s: string): string {
  if (!s) return ''
  const raw = marked.parse(s)
  // marked.parse bisa return string | Promise<string>; dengan opsi default (async:false) → string.
  const html = typeof raw === 'string' ? raw : ''
  return DOMPurify.sanitize(html, {
    ALLOWED_TAGS: [
      'p', 'br', 'hr', 'strong', 'em', 'del', 'code', 'pre', 'blockquote',
      'ul', 'ol', 'li', 'a', 'table', 'thead', 'tbody', 'tr', 'th', 'td',
      'h1', 'h2', 'h3', 'h4', 'h5', 'h6', 'span', 'div', 'img', 'sup', 'sub',
    ],
    ALLOWED_ATTR: ['href', 'title', 'alt', 'src', 'class', 'target', 'rel'],
  })
}

export function useMarkdown() {
  return { renderMarkdown }
}

export default useMarkdown
