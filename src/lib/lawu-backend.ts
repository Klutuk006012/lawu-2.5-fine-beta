export const LAWU_BACKEND_URL =
  import.meta.env.VITE_LAWU_BACKEND_URL ?? 'http://127.0.0.1:3001'

export const LAWU_LLM_URL =
  import.meta.env.VITE_LAWU_LLM_URL ?? 'http://127.0.0.1:8080'

export interface WikiResult {
  docid: string
  title: string
  preview: string
  score: number
}

export interface WikiSearchResponse {
  query: string
  total: number
  latency_ms: number
  results: WikiResult[]
}

export interface WikiStatsResponse {
  index_dir: string
  total_docs: number
  segments: number
}

export async function isBackendReachable(): Promise<boolean> {
  try {
    const controller = new AbortController()
    const timeout = setTimeout(() => controller.abort(), 500)
    const res = await fetch(`${LAWU_BACKEND_URL}/health`, {
      signal: controller.signal,
    })
    clearTimeout(timeout)
    return res.ok
  } catch {
    return false
  }
}

export async function searchWikipedia(
  query: string,
  limit = 10
): Promise<WikiSearchResponse | null> {
  if (!query.trim()) return null
  try {
    const controller = new AbortController()
    const timeout = setTimeout(() => controller.abort(), 2000)
    const url = `${LAWU_BACKEND_URL}/api/search?q=${encodeURIComponent(query)}&limit=${limit}`
    const res = await fetch(url, {
      signal: controller.signal,
      headers: { Accept: 'application/json' },
    })
    clearTimeout(timeout)
    if (!res.ok) return null
    return (await res.json()) as WikiSearchResponse
  } catch {
    return null
  }
}

export async function fetchWikiStats(): Promise<WikiStatsResponse | null> {
  try {
    const controller = new AbortController()
    const timeout = setTimeout(() => controller.abort(), 2000)
    const res = await fetch(`${LAWU_BACKEND_URL}/api/stats`, {
      signal: controller.signal,
      headers: { Accept: 'application/json' },
    })
    clearTimeout(timeout)
    if (!res.ok) return null
    return (await res.json()) as WikiStatsResponse
  } catch {
    return null
  }
}
