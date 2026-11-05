<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { RouterLink, useRouter } from 'vue-router'
import {
  MessageCircle, Search, Globe, BookOpen, Activity, GraduationCap,
  Users, KeyRound, Settings, Compass,
  MessageSquarePlus, Trash2, Pencil, Check, X, ShieldAlert, LogIn, LogOut,
} from 'lucide-vue-next'
import { useChatStore } from '@/stores/chat'
import { useAuthStore } from '@/stores/auth'
import lawuLogo from '@/assets/lawu-logo-256.png'

const chat = useChatStore()
const auth = useAuthStore()
const router = useRouter()

const renamingId = ref<string | null>(null)
const renameDraft = ref('')
const deleteTarget = ref<{ id: string; title: string } | null>(null)

const sortedSessions = computed(() =>
  [...chat.sessions].sort((a, b) =>
    new Date(b.updatedAt).getTime() - new Date(a.updatedAt).getTime(),
  ),
)

async function newChat() {
  if (auth.isAuthenticated) {
    try {
      await chat.createSession()
    } catch {
      // gagal buat session, tetap ke /chat (empty state)
      chat.resetActive()
    }
  } else {
    chat.resetActive()
  }
  router.push('/chat')
}

async function selectSession(id: string) {
  await chat.selectSession(id)
  router.push({ path: '/chat', query: { session: id } })
}

function startRename(id: string, currentTitle: string) {
  renamingId.value = id
  renameDraft.value = currentTitle
}

async function commitRename(id: string) {
  const t = renameDraft.value.trim()
  if (!t) {
    renamingId.value = null
    return
  }
  try {
    await chat.renameSession(id, t)
  } catch {
    // ignore
  }
  renamingId.value = null
}

function cancelRename() {
  renamingId.value = null
}

async function confirmDelete() {
  if (!deleteTarget.value) return
  try {
    await chat.deleteSession(deleteTarget.value.id)
  } catch {
    // ignore
  } finally {
    deleteTarget.value = null
  }
}

function onLogout() {
  auth.logout()
  router.push('/login')
}

onMounted(() => {
  if (auth.isAuthenticated) void chat.loadSessions()
})

onUnmounted(() => {
  // nothing
})

defineExpose({ newChat, selectSession })
</script>

<template>
  <aside class="relative flex h-full flex-col border-r border-border bg-card">
    <!-- Background mountain pattern (subtle, opacity 0.05) -->
    <div
      aria-hidden="true"
      class="pointer-events-none absolute inset-x-0 top-0 z-0 h-44 bg-mountain-pattern bg-repeat opacity-[0.05] dark:opacity-[0.08]"
    />

    <!-- Logo + brand -->
    <div class="relative z-10 px-4 pt-5">
      <RouterLink to="/" class="flex items-center gap-3">
        <img
          :src="lawuLogo"
          alt="Logo Lawu Fine — gunung Lawu, elang garuda, bendera Merah Putih, dan gelombang air"
          class="h-14 w-14 rounded-2xl shadow-lawu-sm ring-1 ring-border"
        />
        <div class="leading-tight">
          <p class="text-base font-extrabold tracking-tight text-foreground">Lawu Fine</p>
          <p class="mt-0.5 text-[11px] font-medium text-muted-foreground">
            AI Lokal Indonesia · v3.7.1
          </p>
          <!-- Badge merah-putih -->
          <span class="lawu-flag-badge mt-1.5">
            <span class="lawu-flag-red">AI</span>
            <span class="lawu-flag-white">LOKAL</span>
            <span class="lawu-flag-red">ID</span>
          </span>
        </div>
      </RouterLink>
    </div>

    <!-- "+ Obrolan Baru" full-width primary -->
    <div class="relative z-10 px-4 pt-4">
      <button
        type="button"
        @click="newChat"
        class="flex w-full items-center justify-center gap-2 rounded-xl bg-primary px-3 py-2.5 text-sm font-semibold text-primary-foreground shadow-lawu-sm transition-all hover:bg-primary/90 hover:shadow-lawu-md"
      >
        <MessageSquarePlus class="h-4 w-4" />
        Obrolan Baru
      </button>
    </div>

    <!-- Nav menu -->
    <nav aria-label="Navigasi utama" class="relative z-10 mt-5 flex-1 overflow-y-auto px-3 lawu-scroll">
      <p class="px-2 pb-1.5 text-[10.5px] font-semibold uppercase tracking-wider text-muted-foreground">
        Modul
      </p>
      <ul class="space-y-0.5">
        <li>
          <RouterLink
            to="/chat"
            class="flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-sm text-foreground/80 transition-colors hover:bg-secondary/60 hover:text-foreground"
            active-class="!border-l-4 !border-primary !bg-primary/5 !pl-[6px] font-semibold !text-primary"
          >
            <MessageCircle class="h-4 w-4 shrink-0" />
            Chat
          </RouterLink>
        </li>
        <li v-if="auth.isAuthenticated">
          <RouterLink
            to="/search"
            class="flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-sm text-foreground/80 transition-colors hover:bg-secondary/60 hover:text-foreground"
            active-class="!border-l-4 !border-primary !bg-primary/5 !pl-[6px] font-semibold !text-primary"
          >
            <Search class="h-4 w-4 shrink-0" />
            Pencarian
          </RouterLink>
        </li>
        <li v-if="auth.isAdmin">
          <RouterLink
            to="/corpus"
            class="flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-sm text-foreground/80 transition-colors hover:bg-secondary/60 hover:text-foreground"
            active-class="!border-l-4 !border-primary !bg-primary/5 !pl-[6px] font-semibold !text-primary"
          >
            <Globe class="h-4 w-4 shrink-0" />
            Korpus Web
          </RouterLink>
        </li>
        <li v-if="auth.isAdmin">
          <RouterLink
            to="/ebooks"
            class="flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-sm text-foreground/80 transition-colors hover:bg-secondary/60 hover:text-foreground"
            active-class="!border-l-4 !border-primary !bg-primary/5 !pl-[6px] font-semibold !text-primary"
          >
            <BookOpen class="h-4 w-4 shrink-0" />
            Ebook &amp; Dokumen
          </RouterLink>
        </li>
        <li v-if="auth.isAuthenticated">
          <RouterLink
            to="/monitor"
            class="flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-sm text-foreground/80 transition-colors hover:bg-secondary/60 hover:text-foreground"
            active-class="!border-l-4 !border-primary !bg-primary/5 !pl-[6px] font-semibold !text-primary"
          >
            <Activity class="h-4 w-4 shrink-0" />
            Monitoring
          </RouterLink>
        </li>
        <li v-if="auth.isAuthenticated">
          <RouterLink
            to="/training"
            class="flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-sm text-foreground/80 transition-colors hover:bg-secondary/60 hover:text-foreground"
            active-class="!border-l-4 !border-primary !bg-primary/5 !pl-[6px] font-semibold !text-primary"
          >
            <GraduationCap class="h-4 w-4 shrink-0" />
            Training Model
            <span class="ml-auto rounded bg-accent/10 px-1.5 py-0.5 text-[9px] font-bold uppercase tracking-wide text-accent">Baru</span>
          </RouterLink>
        </li>
        <li v-if="auth.isAdmin">
          <RouterLink
            to="/users"
            class="flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-sm text-foreground/80 transition-colors hover:bg-secondary/60 hover:text-foreground"
            active-class="!border-l-4 !border-primary !bg-primary/5 !pl-[6px] font-semibold !text-primary"
          >
            <Users class="h-4 w-4 shrink-0" />
            Manajemen User
          </RouterLink>
        </li>
        <li v-if="auth.isAuthenticated">
          <RouterLink
            to="/api-keys"
            class="flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-sm text-foreground/80 transition-colors hover:bg-secondary/60 hover:text-foreground"
            active-class="!border-l-4 !border-primary !bg-primary/5 !pl-[6px] font-semibold !text-primary"
          >
            <KeyRound class="h-4 w-4 shrink-0" />
            API Keys
          </RouterLink>
        </li>
        <li v-if="auth.isAuthenticated">
          <RouterLink
            to="/settings"
            class="flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-sm text-foreground/80 transition-colors hover:bg-secondary/60 hover:text-foreground"
            active-class="!border-l-4 !border-primary !bg-primary/5 !pl-[6px] font-semibold !text-primary"
          >
            <Settings class="h-4 w-4 shrink-0" />
            Pengaturan
          </RouterLink>
        </li>
        <li>
          <RouterLink
            to="/guide"
            class="flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-sm text-foreground/80 transition-colors hover:bg-secondary/60 hover:text-foreground"
            active-class="!border-l-4 !border-primary !bg-primary/5 !pl-[6px] font-semibold !text-primary"
          >
            <Compass class="h-4 w-4 shrink-0" />
            Panduan
          </RouterLink>
        </li>
      </ul>
    </nav>

    <!-- Conversation history (hanya untuk yang login) -->
    <div v-if="auth.isAuthenticated" class="relative z-10 mt-4 min-h-0 flex-1 border-t border-border px-3 pt-3">
      <p class="px-2 pb-1.5 text-[10.5px] font-semibold uppercase tracking-wider text-muted-foreground">
        Riwayat Obrolan
      </p>
      <div class="lawu-scroll max-h-56 overflow-y-auto pr-1">
        <ul class="space-y-0.5 pb-3">
          <li v-if="sortedSessions.length === 0" class="px-2 py-1.5 text-xs text-muted-foreground">
            Belum ada obrolan — mulai dari Obrolan Baru.
          </li>
          <li
            v-for="c in sortedSessions"
            :key="c.id"
            class="group relative"
          >
            <!-- rename mode -->
            <div v-if="renamingId === c.id" class="flex items-center gap-1 px-1 py-1">
              <input
                v-model="renameDraft"
                @keydown.enter.prevent="commitRename(c.id)"
                @keydown.esc="cancelRename"
                @blur="commitRename(c.id)"
                class="h-8 flex-1 rounded-md border border-primary/50 bg-background px-2 text-[12.5px] focus:outline-none"
                :aria-label="`Ganti judul ${c.title}`"
              />
              <button
                type="button"
                @click="commitRename(c.id)"
                class="rounded-md bg-primary p-1 text-primary-foreground"
                aria-label="Simpan"
              >
                <Check class="h-3.5 w-3.5" />
              </button>
              <button
                type="button"
                @click="cancelRename"
                class="rounded-md bg-secondary p-1 text-foreground"
                aria-label="Batal"
              >
                <X class="h-3.5 w-3.5" />
              </button>
            </div>
            <!-- view mode -->
            <button
              v-else
              type="button"
              @click="selectSession(c.id)"
              class="flex w-full items-center rounded-lg px-2.5 py-2 pr-16 text-left text-[13px] transition-colors hover:bg-secondary/60"
              :class="{ 'bg-secondary font-medium': chat.activeSessionId === c.id }"
              :aria-current="chat.activeSessionId === c.id ? 'true' : undefined"
            >
              <span class="truncate">{{ c.title || 'Tanpa Judul' }}</span>
            </button>
            <div
              v-if="renamingId !== c.id"
              class="absolute right-1.5 top-1/2 flex -translate-y-1/2 items-center gap-0.5 opacity-0 transition-opacity focus-within:opacity-100 group-hover:opacity-100"
            >
              <button
                type="button"
                :aria-label="`Ganti nama obrolan ${c.title}`"
                @click.stop="startRename(c.id, c.title)"
                class="rounded-md p-1.5 text-muted-foreground hover:bg-secondary hover:text-foreground"
              >
                <Pencil class="h-3.5 w-3.5" />
              </button>
              <button
                type="button"
                :aria-label="`Hapus obrolan ${c.title}`"
                @click.stop="deleteTarget = { id: c.id, title: c.title }"
                class="rounded-md p-1.5 text-muted-foreground hover:bg-destructive/10 hover:text-destructive"
              >
                <Trash2 class="h-3.5 w-3.5" />
              </button>
            </div>
          </li>
        </ul>
      </div>
    </div>

    <!-- Spacer jika belum login -->
    <div v-else class="relative z-10 flex-1" />

    <!-- Footer: user info + logout -->
    <div class="relative z-10 border-t border-border p-3">
      <div v-if="auth.isAuthenticated && auth.user" class="mb-2 flex items-center gap-2 rounded-xl bg-secondary/60 px-3 py-2">
        <div class="flex h-9 w-9 shrink-0 items-center justify-center rounded-full bg-primary text-primary-foreground shadow-lawu-sm">
          <span class="text-xs font-bold">{{ auth.user.email.charAt(0).toUpperCase() }}</span>
        </div>
        <div class="min-w-0 flex-1">
          <p class="truncate text-xs font-medium text-foreground">{{ auth.user.email }}</p>
          <p class="mt-0.5 inline-block rounded bg-accent/10 px-1.5 py-0.5 text-[9px] font-bold uppercase tracking-wide text-accent">
            {{ auth.user.role }}
          </p>
        </div>
      </div>
      <button
        v-if="auth.isAuthenticated"
        type="button"
        @click="onLogout"
        class="flex w-full items-center justify-center gap-2 rounded-lg border border-border bg-background px-3 py-2 text-sm font-medium text-foreground transition-colors hover:bg-secondary"
      >
        <LogOut class="h-4 w-4" />
        Keluar
      </button>
      <RouterLink
        v-else
        to="/login"
        class="flex w-full items-center justify-center gap-2 rounded-lg bg-primary px-3 py-2 text-sm font-semibold text-primary-foreground transition-colors hover:bg-primary/90"
      >
        <LogIn class="h-4 w-4" />
        Masuk
      </RouterLink>
    </div>

    <!-- Modal konfirmasi delete -->
    <div
      v-if="deleteTarget"
      class="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-4"
      role="dialog"
      aria-modal="true"
      @click.self="deleteTarget = null"
    >
      <div class="w-full max-w-sm rounded-2xl border border-border bg-card p-5 shadow-2xl">
        <div class="flex items-start gap-3">
          <div class="flex h-10 w-10 shrink-0 items-center justify-center rounded-full bg-destructive/10 text-destructive">
            <ShieldAlert class="h-5 w-5" />
          </div>
          <div class="min-w-0 flex-1">
            <h3 class="text-sm font-semibold text-foreground">Hapus obrolan?</h3>
            <p class="mt-1 text-xs text-muted-foreground">
              Obrolan <span class="font-medium text-foreground">"{{ deleteTarget.title }}"</span> akan dihapus permanen beserta seluruh pesannya.
            </p>
          </div>
        </div>
        <div class="mt-4 flex justify-end gap-2">
          <button
            type="button"
            @click="deleteTarget = null"
            class="rounded-lg border border-border bg-background px-3 py-1.5 text-sm text-foreground hover:bg-secondary"
          >
            Batal
          </button>
          <button
            type="button"
            @click="confirmDelete"
            class="rounded-lg bg-destructive px-3 py-1.5 text-sm font-medium text-destructive-foreground hover:bg-destructive/90"
          >
            Hapus
          </button>
        </div>
      </div>
    </div>
  </aside>
</template>
