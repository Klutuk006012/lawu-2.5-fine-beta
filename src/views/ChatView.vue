<script setup lang="ts">
import { ref, nextTick, onMounted } from 'vue'
import { Sparkles, ArrowUp, BookOpenText } from 'lucide-vue-next'

interface ChatMessage {
  id: string
  role: 'user' | 'assistant'
  content: string
  latencyMs?: number | null
  createdAt: string
}

const messages = ref<ChatMessage[]>([])
const input = ref('')
const loading = ref(false)
const error = ref<string | null>(null)
const scrollRef = ref<HTMLDivElement | null>(null)
const textareaRef = ref<HTMLTextAreaElement | null>(null)

const SUGGESTIONS = [
  'Jelaskan cara kerja pencarian ebook ini secara menyeluruh',
  'Apa saja fitur utama dashboard Lawu-2.5-Fine?',
  'Bantu saya memahami modul monitoring & alert',
  'Ringkas langkah-langkah menyiapkan server produksi',
]

function scrollToBottom() {
  const el = scrollRef.value
  if (el && (messages.value.length > 0 || loading.value)) {
    el.scrollTo({ top: el.scrollHeight, behavior: 'smooth' })
  }
}

async function send(text?: string) {
  const content = (text ?? input.value).trim()
  if (!content || loading.value) return
  error.value = null
  input.value = ''
  if (textareaRef.value) textareaRef.value.style.height = 'auto'
  loading.value = true

  const optimistic: ChatMessage = {
    id: `tmp-${Date.now()}`,
    role: 'user',
    content,
    createdAt: new Date().toISOString(),
  }
  messages.value.push(optimistic)

  await nextTick(scrollToBottom)

  try {
    const res = await fetch('/api/chat', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ message: content }),
    })
    const data = await res.json()
    if (!res.ok) throw new Error(data.error ?? 'Gagal mengirim pesan')

    messages.value = messages.value.filter((m) => m.id !== optimistic.id)
    messages.value.push(data.userMessage, data.assistantMessage)
  } catch (err) {
    messages.value = messages.value.filter((m) => m.id !== optimistic.id)
    input.value = content
    error.value = err instanceof Error ? err.message : 'Terjadi kesalahan.'
  } finally {
    loading.value = false
    nextTick(scrollToBottom)
    textareaRef.value?.focus()
  }
}

function autoGrow(e: Event) {
  const el = e.target as HTMLTextAreaElement
  el.style.height = 'auto'
  el.style.height = `${Math.min(el.scrollHeight, 160)}px`
}

const empty = () => messages.value.length === 0 && !loading.value

onMounted(() => {
  textareaRef.value?.focus()
})
</script>

<template>
  <div class="flex w-full flex-col">
    <!-- Area pesan -->
    <div ref="scrollRef" class="h-[calc(100dvh-330px)] min-h-[300px] overflow-y-auto pb-2 sm:h-[calc(100dvh-350px)]">
      <div v-if="empty()" class="mx-auto flex max-w-2xl flex-col items-center px-4 pt-10 text-center sm:pt-16">
        <div class="mb-4 flex h-14 w-14 items-center justify-center rounded-2xl bg-primary text-primary-foreground shadow-sm">
          <Sparkles class="h-7 w-7" />
        </div>
        <h2 class="text-2xl font-semibold tracking-tight text-foreground">Lawu-2.5-Fine</h2>
        <p class="mt-2 max-w-md text-sm text-muted-foreground">
          Asisten cerdas untuk mesin pencari ebook Lawu. Tanya apa saja seputar koleksi, fitur, atau panduan implementasi.
        </p>
        <div class="mt-6 grid w-full grid-cols-1 gap-2 sm:grid-cols-2">
          <button
            v-for="s in SUGGESTIONS"
            :key="s"
            type="button"
            @click="send(s)"
            class="rounded-xl border border-border bg-card px-4 py-3 text-left text-[13px] leading-snug text-foreground/90 shadow-sm transition-all hover:-translate-y-0.5 hover:border-primary/40 hover:shadow-md"
          >
            {{ s }}
          </button>
        </div>
      </div>

      <div v-else class="mx-auto flex max-w-3xl flex-col gap-5 px-4">
        <template v-for="m in messages" :key="m.id">
          <div v-if="m.role === 'user'" class="flex justify-end">
            <div class="max-w-[85%] rounded-2xl rounded-br-md bg-primary px-4 py-2.5 text-sm leading-relaxed text-primary-foreground shadow-sm">
              <p class="whitespace-pre-wrap break-words">{{ m.content }}</p>
            </div>
          </div>
          <div v-else class="flex gap-3">
            <div class="mt-0.5 flex h-8 w-8 shrink-0 items-center justify-center rounded-full bg-secondary text-primary">
              <BookOpenText class="h-4 w-4" />
            </div>
            <div class="min-w-0 flex-1">
              <div class="prose-lawu space-y-2 text-sm leading-relaxed text-foreground">
                {{ m.content }}
              </div>
              <p v-if="m.latencyMs" class="mt-1.5 text-[11px] text-muted-foreground">
                Lawu-2.5-Fine · {{ m.latencyMs }} ms
              </p>
            </div>
          </div>
        </template>
        <div v-if="loading" class="flex gap-3">
          <div class="mt-0.5 flex h-8 w-8 shrink-0 items-center justify-center rounded-full bg-secondary text-primary">
            <BookOpenText class="h-4 w-4" />
          </div>
          <div class="flex items-center gap-1 py-2">
            <span class="h-2 w-2 animate-bounce rounded-full bg-primary/70" style="animation-delay: 0ms; animation-duration: 900ms" />
            <span class="h-2 w-2 animate-bounce rounded-full bg-primary/70" style="animation-delay: 150ms; animation-duration: 900ms" />
            <span class="h-2 w-2 animate-bounce rounded-full bg-primary/70" style="animation-delay: 300ms; animation-duration: 900ms" />
          </div>
        </div>
      </div>
    </div>

    <!-- Input -->
    <div class="mx-auto w-full max-w-3xl px-4 pb-4">
      <div v-if="error" class="mb-2 rounded-lg border border-destructive/40 bg-destructive/10 px-3 py-2 text-xs text-destructive">
        {{ error }}
      </div>
      <div class="flex items-end gap-2 rounded-2xl border border-border bg-card p-2 shadow-sm focus-within:border-primary/50">
        <textarea
          ref="textareaRef"
          v-model="input"
          @input="autoGrow"
          @keydown.enter.exact.prevent="send()"
          placeholder="Tanya apa saja ke Lawu-2.5-Fine…"
          aria-label="Pesan untuk Lawu-2.5-Fine"
          class="min-h-[44px] max-h-40 flex-1 resize-none border-0 bg-transparent px-2 py-2.5 text-sm shadow-none focus:outline-none"
        />
        <button
          type="button"
          @click="send()"
          :disabled="loading || !input.trim()"
          aria-label="Kirim pesan"
          class="mb-0.5 flex h-10 w-10 shrink-0 items-center justify-center rounded-xl bg-primary text-primary-foreground shadow-sm hover:bg-primary/90 disabled:opacity-50"
        >
          <ArrowUp class="h-5 w-5" />
        </button>
      </div>
      <p class="mt-2 text-center text-[11px] text-muted-foreground">
        Enter untuk kirim · Shift+Enter baris baru
      </p>
    </div>
  </div>
</template>
