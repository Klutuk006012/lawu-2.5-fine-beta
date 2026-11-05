<script setup lang="ts">
import { ref, computed, nextTick, onMounted, watch, onBeforeUnmount } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import {
  Sparkles, ArrowUp, BookOpenText, Trash2, Pencil, Check, X, LoaderCircle,
  MessageSquarePlus,
} from 'lucide-vue-next'
import { useChatStore } from '@/stores/chat'
import { useAuthStore } from '@/stores/auth'
import { renderMarkdown } from '@/composables/useMarkdown'

const chat = useChatStore()
const auth = useAuthStore()
const route = useRoute()
const router = useRouter()

const input = ref('')
const scrollRef = ref<HTMLDivElement | null>(null)
const textareaRef = ref<HTMLTextAreaElement | null>(null)

const editingTitle = ref(false)
const titleDraft = ref('')
const titleInputRef = ref<HTMLInputElement | null>(null)

const SUGGESTIONS = [
  'Jelaskan cara kerja pencarian ebook ini secara menyeluruh',
  'Apa saja fitur utama dashboard Lawu Fine?',
  'Bantu saya memahami modul monitoring & alert',
  'Ringkas langkah-langkah menyiapkan server produksi',
]

const sessionTitle = computed(() => {
  const s = chat.activeSession
  return s?.title || (chat.activeSessionId ? 'Obrolan tanpa judul' : 'Obrolan Baru')
})

const isEmpty = computed(
  () => chat.messages.length === 0 && !chat.sending && !chat.loading,
)

// Timer "sedang berpikir" — inference CPU lokal bisa 1-3 menit,
// user harus tahu sistem masih bekerja (bukan mati).
const thinkSecs = ref(0)
let thinkTimer: ReturnType<typeof setInterval> | null = null
watch(
  () => chat.sending,
  (on) => {
    if (on) {
      thinkSecs.value = 0
      thinkTimer = setInterval(() => {
        thinkSecs.value++
      }, 1000)
    } else if (thinkTimer) {
      clearInterval(thinkTimer)
      thinkTimer = null
    }
  },
)
onBeforeUnmount(() => {
  if (thinkTimer) clearInterval(thinkTimer)
})

function scrollToBottom() {
  const el = scrollRef.value
  if (!el) return
  el.scrollTo({ top: el.scrollHeight, behavior: 'smooth' })
}

async function send(text?: string) {
  const content = (text ?? input.value).trim()
  if (!content || chat.sending) return
  input.value = ''
  if (textareaRef.value) textareaRef.value.style.height = 'auto'
  try {
    await chat.sendMessage(content)
    await nextTick(scrollToBottom)
  } catch {
    // error sudah di-store, biarkan UI tampilkan.
  }
}

async function newChat() {
  if (auth.isAuthenticated) {
    try {
      await chat.createSession()
    } catch {
      chat.resetActive()
    }
  } else {
    chat.resetActive()
  }
  // Tetap di /chat
  router.push('/chat')
}

function autoGrow(e: Event) {
  const el = e.target as HTMLTextAreaElement
  el.style.height = 'auto'
  el.style.height = `${Math.min(el.scrollHeight, 160)}px`
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault()
    send()
  }
}

function startEditTitle() {
  if (!chat.activeSession) return
  titleDraft.value = chat.activeSession.title
  editingTitle.value = true
  nextTick(() => titleInputRef.value?.focus())
}

async function saveTitle() {
  if (!chat.activeSessionId) {
    editingTitle.value = false
    return
  }
  const t = titleDraft.value.trim()
  if (!t || t === chat.activeSession?.title) {
    editingTitle.value = false
    return
  }
  try {
    await chat.renameSession(chat.activeSessionId, t)
  } catch {
    // ignore
  }
  editingTitle.value = false
}

function cancelEditTitle() {
  editingTitle.value = false
}

async function onDeleteSession() {
  if (!chat.activeSessionId) return
  if (!confirm('Hapus obrolan ini? Tidak bisa dibatalkan.')) return
  try {
    await chat.deleteSession(chat.activeSessionId)
  } catch {
    // ignore
  }
}

function fmtTime(iso: string): string {
  try {
    return new Date(iso).toLocaleTimeString('id-ID', { hour: '2-digit', minute: '2-digit' })
  } catch {
    return ''
  }
}

async function ensureSessionFromQuery() {
  const q = route.query.session
  if (typeof q === 'string' && q) {
    if (chat.activeSessionId !== q) {
      await chat.selectSession(q)
    }
  }
}

onMounted(async () => {
  // Load sessions untuk sidebar (di sini karena halaman chat adalah default landing).
  if (auth.isAuthenticated) {
    void chat.loadSessions()
  }
  await ensureSessionFromQuery()
  textareaRef.value?.focus()
})

watch(
  () => chat.messages.length,
  () => nextTick(scrollToBottom),
)

watch(
  () => route.query.session,
  (s) => {
    if (typeof s === 'string' && s && s !== chat.activeSessionId) {
      void chat.selectSession(s)
    }
  },
)

watch(
  () => chat.activeSessionId,
  (id) => {
    if (id && route.query.session !== id) {
      router.replace({ path: '/chat', query: { session: id } })
    }
  },
)
</script>

<template>
  <div class="flex h-full w-full flex-col">
    <!-- Header chat -->
    <header class="flex items-center gap-2 border-b border-border bg-background/85 px-4 py-2.5 backdrop-blur">
      <div v-if="editingTitle" class="flex flex-1 items-center gap-2">
        <input
          ref="titleInputRef"
          v-model="titleDraft"
          @keydown.enter.prevent="saveTitle"
          @keydown.esc="cancelEditTitle"
          class="h-9 flex-1 rounded-lg border border-border bg-card px-3 text-sm focus:border-primary focus:outline-none"
          aria-label="Edit judul obrolan"
        />
        <button
          type="button"
          @click="saveTitle"
          class="rounded-md bg-primary p-1.5 text-primary-foreground hover:bg-primary/90"
          aria-label="Simpan judul"
        >
          <Check class="h-4 w-4" />
        </button>
        <button
          type="button"
          @click="cancelEditTitle"
          class="rounded-md bg-secondary p-1.5 text-foreground hover:bg-secondary/70"
          aria-label="Batal edit judul"
        >
          <X class="h-4 w-4" />
        </button>
      </div>
      <div v-else class="flex min-w-0 flex-1 items-center gap-2">
        <h2 class="truncate text-sm font-semibold text-foreground" :title="sessionTitle">
          {{ sessionTitle }}
        </h2>
        <button
          v-if="chat.activeSession"
          type="button"
          @click="startEditTitle"
          class="rounded-md p-1 text-muted-foreground hover:bg-secondary hover:text-foreground"
          aria-label="Edit judul obrolan"
        >
          <Pencil class="h-3.5 w-3.5" />
        </button>
      </div>
      <button
        v-if="chat.activeSession"
        type="button"
        @click="onDeleteSession"
        class="rounded-md p-1.5 text-muted-foreground hover:bg-destructive/10 hover:text-destructive"
        aria-label="Hapus obrolan"
      >
        <Trash2 class="h-4 w-4" />
      </button>
      <button
        type="button"
        @click="newChat"
        class="inline-flex items-center gap-1.5 rounded-md bg-primary px-2.5 py-1.5 text-xs font-semibold text-primary-foreground hover:bg-primary/90"
      >
        <MessageSquarePlus class="h-3.5 w-3.5" />
        Obrolan Baru
      </button>
    </header>

    <!-- Area pesan -->
    <div
      ref="scrollRef"
      class="lawu-scroll min-h-0 flex-1 overflow-y-auto"
    >
      <!-- Empty state dengan gunung -->
      <div
        v-if="isEmpty"
        class="relative mx-auto flex max-w-2xl flex-col items-center px-4 pt-10 text-center sm:pt-16"
      >
        <!-- Mountain decorative -->
        <div aria-hidden="true" class="pointer-events-none absolute inset-x-0 -top-6 z-0 mx-auto h-40 w-full max-w-2xl opacity-[0.06] dark:opacity-[0.1]">
          <svg viewBox="0 0 600 200" xmlns="http://www.w3.org/2000/svg" preserveAspectRatio="xMidYMax meet" class="h-full w-full">
            <defs>
              <linearGradient id="lawu-empty-mountain" x1="0" x2="0" y1="0" y2="1">
                <stop offset="0%" stop-color="currentColor" class="text-primary" />
                <stop offset="100%" stop-color="currentColor" class="text-primary" stop-opacity="0.4" />
              </linearGradient>
            </defs>
            <!-- 3 puncak gunung untuk simetri visual (Hargo Dlemo + Cakrahayu + 1 simetri) -->
            <path d="M0 200 L100 80 L180 140 L260 30 L360 130 L460 70 L540 130 L600 100 L600 200 Z" fill="url(#lawu-empty-mountain)" />
            <path d="M0 200 L80 130 L160 170 L240 100 L340 165 L420 120 L520 160 L600 130 L600 200 Z" fill="currentColor" class="text-primary" fill-opacity="0.55" />
          </svg>
        </div>

        <div class="relative z-10 mb-4 flex h-14 w-14 items-center justify-center rounded-2xl bg-primary text-primary-foreground shadow-lawu-sm">
          <Sparkles class="h-7 w-7" />
        </div>
        <h2 class="relative z-10 text-2xl font-extrabold tracking-tight text-foreground">
          Mulai obrolan baru dengan Lawu Fine
        </h2>
        <p class="relative z-10 mt-2 max-w-md text-sm text-muted-foreground">
          Asisten cerdas untuk mesin pencari ebook Lawu Fine. Tanya apa saja seputar koleksi,
          fitur, atau panduan implementasi. Dibangun di Indonesia, untuk Indonesia.
        </p>
        <div class="relative z-10 mt-6 grid w-full grid-cols-1 gap-2 sm:grid-cols-2">
          <button
            v-for="s in SUGGESTIONS"
            :key="s"
            type="button"
            @click="send(s)"
            class="rounded-xl border border-border bg-card px-4 py-3 text-left text-[13px] leading-snug text-foreground/90 shadow-lawu-sm transition-all hover:-translate-y-0.5 hover:border-primary/40 hover:shadow-lawu-md"
          >
            {{ s }}
          </button>
        </div>
      </div>

      <!-- Loading state -->
      <div v-else-if="chat.loading" class="mx-auto flex max-w-3xl items-center gap-3 px-4 py-10">
        <LoaderCircle class="h-5 w-5 animate-spin text-primary" />
        <p class="text-sm text-muted-foreground">Memuat riwayat pesan…</p>
      </div>

      <!-- Messages list -->
      <div v-else class="mx-auto flex max-w-3xl flex-col gap-5 px-4 py-6">
        <p v-if="chat.messages.length === 0" class="text-center text-sm text-muted-foreground py-10">
          Belum ada obrolan. Kirim pesan pertama!
        </p>
        <template v-for="m in chat.messages" :key="m.id">
          <!-- user -->
          <div v-if="m.role === 'user'" class="flex justify-end">
            <div class="flex max-w-[85%] flex-col items-end gap-1">
              <div class="rounded-2xl rounded-br-md bg-primary px-4 py-2.5 text-sm leading-relaxed text-primary-foreground shadow-lawu-sm">
                <p class="whitespace-pre-wrap break-words">{{ m.content }}</p>
              </div>
              <p v-if="m.createdAt" class="text-[10.5px] text-muted-foreground">{{ fmtTime(m.createdAt) }}</p>
            </div>
          </div>
          <!-- assistant -->
          <div v-else class="flex gap-3">
            <!-- Avatar gunung mini -->
            <div class="mt-0.5 flex h-8 w-8 shrink-0 items-center justify-center rounded-full bg-secondary text-primary ring-1 ring-border">
              <BookOpenText class="h-4 w-4" />
            </div>
            <div class="min-w-0 flex-1">
              <div
                class="prose-lawu rounded-2xl rounded-bl-md border border-border bg-card px-4 py-3 text-sm leading-relaxed text-foreground shadow-lawu-sm"
                v-html="renderMarkdown(m.content)"
              />
              <p class="mt-1.5 px-1 text-[10.5px] text-muted-foreground">
                Lawu Fine
                <span v-if="m.latencyMs"> · {{ m.latencyMs }} ms</span>
                <span v-if="m.createdAt"> · {{ fmtTime(m.createdAt) }}</span>
              </p>
            </div>
          </div>
        </template>

        <!-- Typing dots -->
        <div v-if="chat.sending" class="flex gap-3">
          <div class="mt-0.5 flex h-8 w-8 shrink-0 items-center justify-center rounded-full bg-secondary text-primary ring-1 ring-border">
            <BookOpenText class="h-4 w-4" />
          </div>
          <div class="flex flex-col gap-0.5 py-2">
            <div class="flex items-center gap-1">
              <span class="h-2 w-2 animate-bounce rounded-full bg-primary/70" style="animation-delay: 0ms; animation-duration: 900ms" />
              <span class="h-2 w-2 animate-bounce rounded-full bg-primary/70" style="animation-delay: 150ms; animation-duration: 900ms" />
              <span class="h-2 w-2 animate-bounce rounded-full bg-primary/70" style="animation-delay: 300ms; animation-duration: 900ms" />
            </div>
            <span class="text-xs text-muted-foreground">
              Lawu Fine sedang berpikir… <span class="tabular-nums">{{ thinkSecs }} dtk</span>
              <span v-if="thinkSecs >= 30" class="text-[10.5px]"> · CPU lokal, jawaban panjang bisa 1–3 menit</span>
            </span>
          </div>
        </div>
      </div>
    </div>

    <!-- Input -->
    <div class="border-t border-border bg-background/85 px-4 py-3 backdrop-blur">
      <div class="mx-auto w-full max-w-3xl">
        <div
          v-if="chat.error"
          class="mb-2 rounded-lg border border-destructive/40 bg-destructive/10 px-3 py-2 text-xs text-destructive"
        >
          {{ chat.error }}
        </div>
        <div class="flex items-end gap-2 rounded-2xl border border-border bg-card p-2 shadow-lawu-sm focus-within:border-primary/50">
          <textarea
            ref="textareaRef"
            v-model="input"
            @input="autoGrow"
            @keydown="onKeydown"
            placeholder="Tanya apa saja ke Lawu Fine…"
            aria-label="Pesan untuk Lawu Fine"
            class="min-h-[44px] max-h-40 flex-1 resize-none border-0 bg-transparent px-2 py-2.5 text-sm shadow-none focus:outline-none"
          />
          <button
            type="button"
            @click="send()"
            :disabled="chat.sending || !input.trim()"
            aria-label="Kirim pesan"
            class="mb-0.5 flex h-10 w-10 shrink-0 items-center justify-center rounded-xl bg-primary text-primary-foreground shadow-lawu-sm hover:bg-primary/90 disabled:opacity-50"
          >
            <LoaderCircle v-if="chat.sending" class="h-5 w-5 animate-spin" />
            <ArrowUp v-else class="h-5 w-5" />
          </button>
        </div>
        <p class="mt-2 text-center text-[10.5px] text-muted-foreground">
          Enter untuk kirim · Shift+Enter baris baru · AI Lokal Indonesia
        </p>
      </div>
    </div>
  </div>
</template>
