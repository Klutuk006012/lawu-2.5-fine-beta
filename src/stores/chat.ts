import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { api } from '@/lib/api'

export interface ChatSession {
  id: string
  title: string
  createdAt: string
  updatedAt: string
  messageCount: number
}

export interface ChatMessage {
  id: string
  role: 'user' | 'assistant'
  content: string
  latencyMs?: number | null
  createdAt: string
}

interface SessionsResponse {
  sessions: ChatSession[]
}

interface SessionDetailResponse {
  session: ChatSession
  messages: ChatMessage[]
}

interface CreateSessionResponse {
  session: ChatSession
}

interface PatchSessionResponse {
  session: ChatSession
}

interface SendChatResponse {
  userMessage: ChatMessage
  assistantMessage: ChatMessage
}

export const useChatStore = defineStore('chat', () => {
  const sessions = ref<ChatSession[]>([])
  const activeSessionId = ref<string | null>(null)
  const messages = ref<ChatMessage[]>([])
  const loading = ref(false)
  const sending = ref(false)
  const error = ref<string | null>(null)

  const activeSession = computed(
    () => sessions.value.find((s) => s.id === activeSessionId.value) ?? null,
  )

  async function loadSessions(): Promise<void> {
    try {
      const res = await api.get<SessionsResponse>('/api/conversations?limit=50')
      sessions.value = res.data.sessions ?? []
    } catch (err: unknown) {
      sessions.value = []
      // Tidak kita simpan ke error — loadSessions background.
      void err
    }
  }

  async function selectSession(id: string): Promise<void> {
    activeSessionId.value = id
    loading.value = true
    error.value = null
    try {
      const res = await api.get<SessionDetailResponse>(`/api/conversations/${id}`)
      messages.value = res.data.messages ?? []
    } catch (err: unknown) {
      messages.value = []
      error.value = err instanceof Error ? err.message : 'Gagal memuat pesan.'
    } finally {
      loading.value = false
    }
  }

  async function createSession(title?: string): Promise<ChatSession> {
    const res = await api.post<CreateSessionResponse>('/api/conversations', title ? { title } : {})
    const session = res.data.session
    sessions.value = [session, ...sessions.value]
    activeSessionId.value = session.id
    messages.value = []
    return session
  }

  async function deleteSession(id: string): Promise<void> {
    await api.delete(`/api/conversations/${id}`)
    sessions.value = sessions.value.filter((s) => s.id !== id)
    if (activeSessionId.value === id) {
      activeSessionId.value = null
      messages.value = []
    }
  }

  async function renameSession(id: string, title: string): Promise<void> {
    const res = await api.patch<PatchSessionResponse>(`/api/conversations/${id}`, { title })
    const idx = sessions.value.findIndex((s) => s.id === id)
    if (idx >= 0) sessions.value[idx] = res.data.session
  }

  async function sendMessage(text: string): Promise<void> {
    const content = text.trim()
    if (!content) return
    if (!activeSessionId.value) {
      await createSession()
    }
    const sessionId = activeSessionId.value!
    sending.value = true
    error.value = null

    // Optimistic: tampilkan user message sementara.
    const tmpId = `tmp-${Date.now()}`
    messages.value.push({
      id: tmpId,
      role: 'user',
      content,
      createdAt: new Date().toISOString(),
    })

    try {
      // Timeout 5 menit: inference CPU-only (7B Q4_K_M, 6 thread) untuk jawaban
      // panjang + prompt RAG bisa 1-3 menit. Timeout global 30 dtk terlalu pendek.
      const res = await api.post<SendChatResponse>(
        '/api/chat',
        {
          conversationId: sessionId,
          message: content,
        },
        { timeout: 300000 },
      )
      messages.value = messages.value.filter((m) => m.id !== tmpId)
      messages.value.push(res.data.userMessage, res.data.assistantMessage)
      // Refresh sidebar (title mungkin auto-update).
      void loadSessions()
    } catch (err: unknown) {
      messages.value = messages.value.filter((m) => m.id !== tmpId)
      error.value = err instanceof Error ? err.message : 'Gagal mengirim pesan ke server.'
      throw err
    } finally {
      sending.value = false
    }
  }

  function resetActive(): void {
    activeSessionId.value = null
    messages.value = []
  }

  return {
    sessions,
    activeSessionId,
    activeSession,
    messages,
    loading,
    sending,
    error,
    loadSessions,
    selectSession,
    createSession,
    deleteSession,
    renameSession,
    sendMessage,
    resetActive,
  }
})
