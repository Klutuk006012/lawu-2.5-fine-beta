import axios, { type AxiosInstance, type AxiosRequestConfig, type AxiosResponse } from 'axios'
import { useAuthStore } from '@/stores/auth'

/**
 * Axios instance untuk semua call API Lawu-2.5-Fine.
 * - baseURL kosong di production (pakai nginx reverse proxy di /api/*).
 * - Request interceptor: menyematkan Authorization: Bearer <token>.
 * - Response interceptor: jika 401, logout otomatis + redirect ke /login.
 */
export const api: AxiosInstance = axios.create({
  baseURL: import.meta.env.VITE_LAWU_BACKEND_URL ?? '',
  timeout: 30000,
  headers: { 'Content-Type': 'application/json' },
})

api.interceptors.request.use(
  (config) => {
    // Hindari akses Pinia sebelum app siap.
    try {
      const auth = useAuthStore()
      if (auth?.token) {
        config.headers = config.headers ?? {}
        config.headers.Authorization = `Bearer ${auth.token}`
      }
    } catch {
      // Pinia belum aktif (e.g. saat HMR) — abaikan.
    }
    return config
  },
  (err) => Promise.reject(err),
)

api.interceptors.response.use(
  (res: AxiosResponse) => res,
  (err) => {
    if (err?.response?.status === 401) {
      try {
        const auth = useAuthStore()
        if (auth) {
          auth.clearLocal()
          if (typeof window !== 'undefined' && !window.location.pathname.startsWith('/login')) {
            const next = encodeURIComponent(window.location.pathname + window.location.search)
            window.location.href = `/login?next=${next}`
          }
        }
      } catch {
        // silent
      }
    }
    return Promise.reject(err)
  },
)

/** Helper untuk GET request yang langsung mengembalikan data. */
export async function getJSON<T = unknown>(url: string, config?: AxiosRequestConfig): Promise<T> {
  const res = await api.get<T>(url, config)
  return res.data
}

/** Helper untuk POST request yang langsung mengembalikan data. */
export async function postJSON<T = unknown>(url: string, body?: unknown, config?: AxiosRequestConfig): Promise<T> {
  const res = await api.post<T>(url, body, config)
  return res.data
}

export default api
