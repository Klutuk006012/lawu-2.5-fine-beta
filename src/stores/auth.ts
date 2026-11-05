import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { api } from '@/lib/api'

export type UserRole = 'admin' | 'guest'

export interface AuthUser {
  id: number
  email: string
  role: UserRole
  createdAt: string
  lastLoginAt?: string | null
}

interface LoginResponse {
  token: string
  user: AuthUser
}

interface MeResponse {
  user: AuthUser
}

const TOKEN_KEY = 'lawu-token'

export const useAuthStore = defineStore('auth', () => {
  const user = ref<AuthUser | null>(null)
  const token = ref<string | null>(localStorage.getItem(TOKEN_KEY))
  const isAuthenticated = computed(() => !!token.value)
  const isAdmin = computed(() => user.value?.role === 'admin')
  const isGuest = computed(() => !user.value || user.value.role === 'guest')

  async function login(email: string, password: string): Promise<LoginResponse> {
    const res = await api.post<LoginResponse>('/api/auth/login', { email, password })
    token.value = res.data.token
    user.value = res.data.user
    localStorage.setItem(TOKEN_KEY, token.value!)
    return res.data
  }

  async function fetchMe(): Promise<void> {
    if (!token.value) return
    try {
      const res = await api.get<MeResponse>('/api/auth/me')
      user.value = res.data.user
    } catch {
      logout()
    }
  }

  async function logout(): Promise<void> {
    // Best-effort panggil endpoint logout di server.
    if (token.value) {
      try {
        await api.post('/api/auth/logout', {})
      } catch {
        // ignore — tetap clear lokal.
      }
    }
    user.value = null
    token.value = null
    localStorage.removeItem(TOKEN_KEY)
  }

  /** Logout lokal tanpa panggil server (dipakai oleh interceptor 401). */
  function clearLocal() {
    user.value = null
    token.value = null
    localStorage.removeItem(TOKEN_KEY)
  }

  return { user, token, isAuthenticated, isAdmin, isGuest, login, fetchMe, logout, clearLocal }
})
