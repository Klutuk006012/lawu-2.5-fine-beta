<script setup lang="ts">
import { ref } from 'vue'
import { RouterLink, useRouter } from 'vue-router'
import {
  BookOpenText,
  BookMarked,
  Cpu,
  Server,
  Settings2,
  MessageSquarePlus,
  Trash2,
} from 'lucide-vue-next'

const router = useRouter()

interface Conversation {
  id: string
  title: string
  updatedAt: string
  messageCount: number
}

const conversations = ref<Conversation[]>([])
const activeConversationId = ref<string | null>(null)

async function loadConversations() {
  try {
    const res = await fetch('/api/conversations')
    if (res.ok) {
      const data = await res.json()
      conversations.value = Array.isArray(data) ? data : []
    }
  } catch {
    conversations.value = []
  }
}

function newChat() {
  activeConversationId.value = null
  router.push('/chat')
}

async function deleteConversation(id: string) {
  try {
    await fetch(`/api/conversations/${id}`, { method: 'DELETE' })
    if (activeConversationId.value === id) activeConversationId.value = null
    loadConversations()
  } catch {
    // silent fail
  }
}

function selectConversation(id: string) {
  activeConversationId.value = id
  router.push('/chat')
}

loadConversations()

defineExpose({ loadConversations })
</script>

<template>
  <aside class="flex h-full flex-col border-r border-border bg-card">
    <!-- Logo -->
    <div class="px-4 pt-4">
      <div class="flex items-center gap-2.5">
        <div class="flex h-9 w-9 items-center justify-center rounded-xl bg-primary text-primary-foreground shadow-sm">
          <svg viewBox="0 0 32 32" fill="none" class="h-5 w-5" aria-hidden>
            <path d="M3 26 L11 12 L16 19 L21 8 L29 26 Z" fill="currentColor" opacity="0.95" />
          </svg>
        </div>
        <div class="leading-tight">
          <p class="text-sm font-semibold tracking-tight text-foreground">Lawu-2.5-Fine</p>
          <p class="text-[10.5px] text-muted-foreground">Mesin Pencari Ebook · Dashboard</p>
        </div>
      </div>
    </div>

    <!-- New Chat button -->
    <div class="px-3 pt-4">
      <button
        type="button"
        @click="newChat"
        class="flex w-full items-center justify-start gap-2 rounded-xl bg-primary px-3 py-2 text-sm font-medium text-primary-foreground shadow-sm transition-colors hover:bg-primary/90"
      >
        <MessageSquarePlus class="h-4 w-4" />
        Obrolan Baru
      </button>
    </div>

    <!-- Nav -->
    <nav aria-label="Navigasi utama" class="mt-4 px-3">
      <p class="px-2 pb-1.5 text-[10.5px] font-medium uppercase tracking-wider text-muted-foreground">
        Modul
      </p>
      <ul class="space-y-0.5">
        <li>
          <RouterLink
            to="/chat"
            class="flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-sm transition-colors hover:bg-secondary"
            active-class="bg-primary/10 font-medium text-primary"
          >
            <BookOpenText class="h-4 w-4 shrink-0" />
            Konsol Chat
          </RouterLink>
        </li>
        <li>
          <RouterLink
            to="/search"
            class="flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-sm transition-colors hover:bg-secondary"
            active-class="bg-primary/10 font-medium text-primary"
          >
            <BookMarked class="h-4 w-4 shrink-0" />
            Pencarian Ebook
          </RouterLink>
        </li>
        <li>
          <RouterLink
            to="/monitor"
            class="flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-sm transition-colors hover:bg-secondary"
            active-class="bg-primary/10 font-medium text-primary"
          >
            <Cpu class="h-4 w-4 shrink-0" />
            Monitoring &amp; Alert
          </RouterLink>
        </li>
        <li>
          <RouterLink
            to="/guide"
            class="flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-sm transition-colors hover:bg-secondary"
            active-class="bg-primary/10 font-medium text-primary"
          >
            <Server class="h-4 w-4 shrink-0" />
            Panduan Implementasi
          </RouterLink>
        </li>
        <li>
          <RouterLink
            to="/settings"
            class="flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-sm transition-colors hover:bg-secondary"
            active-class="bg-primary/10 font-medium text-primary"
          >
            <Settings2 class="h-4 w-4 shrink-0" />
            Pengaturan
          </RouterLink>
        </li>
      </ul>
    </nav>

    <!-- Conversation history -->
    <div class="mt-4 min-h-0 flex-1 px-3">
      <p class="px-2 pb-1.5 text-[10.5px] font-medium uppercase tracking-wider text-muted-foreground">
        Riwayat Obrolan
      </p>
      <div class="h-[calc(100%-2rem)] overflow-y-auto pr-2">
        <ul class="space-y-0.5 pb-3">
          <li v-if="conversations.length === 0" class="px-2 py-1.5 text-xs text-muted-foreground">
            Belum ada obrolan — mulai dari Konsol Chat.
          </li>
          <li
            v-for="c in conversations"
            :key="c.id"
            class="group relative"
          >
            <button
              type="button"
              @click="selectConversation(c.id)"
              class="flex w-full items-center rounded-lg px-2.5 py-2 pr-8 text-left text-[13px] transition-colors hover:bg-secondary"
              :class="{ 'bg-secondary font-medium': activeConversationId === c.id }"
            >
              <span class="truncate">{{ c.title }}</span>
            </button>
            <button
              type="button"
              :aria-label="`Hapus obrolan ${c.title}`"
              @click="deleteConversation(c.id)"
              class="absolute right-1.5 top-1/2 -translate-y-1/2 rounded-md p-1.5 text-muted-foreground opacity-0 transition-opacity hover:bg-destructive/10 hover:text-destructive focus-visible:opacity-100 group-hover:opacity-100"
            >
              <Trash2 class="h-3.5 w-3.5" />
            </button>
          </li>
        </ul>
      </div>
    </div>
  </aside>
</template>
