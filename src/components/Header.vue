<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { Moon, Sun, Menu, LogIn, LogOut, ChevronDown, User as UserIcon, Settings as SettingsIcon } from 'lucide-vue-next'
import { useThemeStore } from '@/stores/theme'
import { useAuthStore } from '@/stores/auth'
import lawuLogoMini from '@/assets/lawu-logo-64.png'

defineProps<{ backendOnline: boolean }>()
const emit = defineEmits<{ 'toggle-sidebar': [] }>()

const theme = useThemeStore()
const auth = useAuthStore()
const route = useRoute()
const router = useRouter()

const userMenuOpen = ref(false)
let menuTimer: ReturnType<typeof setTimeout> | null = null

const viewTitle = computed(() => {
  switch (route.name) {
    case 'chat': return 'Konsol Chat'
    case 'search': return 'Pencarian Korpus'
    case 'corpus': return 'Web Korpus Lawu Fine'
    case 'ebooks': return 'Perpustakaan Ebook'
    case 'monitor': return 'Monitoring & Alert'
    case 'training': return 'Manajemen Training Lawu Model Fine'
    case 'guide': return 'Panduan Implementasi'
    case 'settings': return 'Pengaturan'
    case 'users': return 'Manajemen User'
    case 'api-keys': return 'API Keys'
    case 'login': return 'Masuk'
    default: return 'Lawu Fine'
  }
})

function toggleTheme() {
  theme.toggle()
}

function onUserMenuEnter() {
  if (menuTimer) clearTimeout(menuTimer)
  userMenuOpen.value = true
}

function onUserMenuLeave() {
  if (menuTimer) clearTimeout(menuTimer)
  menuTimer = setTimeout(() => { userMenuOpen.value = false }, 200)
}

function goLogin() {
  router.push('/login')
}

function goSettings() {
  userMenuOpen.value = false
  router.push('/settings')
}

async function onLogout() {
  userMenuOpen.value = false
  await auth.logout()
  router.push('/login')
}

onMounted(() => {
  theme.init()
})

onUnmounted(() => {
  if (menuTimer) clearTimeout(menuTimer)
})
</script>

<template>
  <header
    class="sticky top-0 z-30 flex h-16 items-center justify-between gap-2 border-b border-border bg-background/85 px-4 backdrop-blur-md"
  >
    <!-- Left: hamburger (mobile only) + logo mini + title + badge -->
    <div class="flex min-w-0 flex-1 items-center gap-2">
      <button
        type="button"
        class="rounded-md p-2 text-foreground transition-colors hover:bg-secondary md:hidden"
        aria-label="Buka / tutup menu samping"
        @click="emit('toggle-sidebar')"
      >
        <Menu class="h-5 w-5" />
      </button>

      <!-- Logo mini + nama — di mobile hanya tampil logo -->
      <img
        :src="lawuLogoMini"
        alt="Logo Lawu Fine"
        class="h-8 w-8 rounded-lg ring-1 ring-border shadow-lawu-sm md:hidden"
      />

      <!-- Title + badge AI Lokal Indonesia — desktop & mobile (badge) -->
      <div class="flex min-w-0 flex-1 items-center gap-2">
        <h1 class="truncate text-base font-bold tracking-tight text-foreground sm:text-lg">
          {{ viewTitle }}
        </h1>
        <!-- Badge split merah-putih "AI LOKAL" — desktop only -->
        <span class="lawu-flag-badge hidden sm:inline-flex">
          <span class="lawu-flag-red">AI</span>
          <span class="lawu-flag-white">LOKAL</span>
          <span class="lawu-flag-red">ID</span>
        </span>
        <!-- Badge "Dibuat di Indonesia" — desktop only -->
        <span class="hidden items-center gap-1 rounded-md bg-secondary px-2 py-0.5 text-[10px] font-medium text-secondary-foreground lg:inline-flex">
          Dibuat di Indonesia
        </span>
      </div>
    </div>

    <!-- Right: theme toggle + user -->
    <div class="flex items-center gap-1.5">
      <button
        type="button"
        :aria-label="theme.isDark ? 'Ganti ke mode terang' : 'Ganti ke mode gelap'"
        @click="toggleTheme"
        class="rounded-md p-2 text-foreground transition-colors hover:bg-secondary"
      >
        <Sun v-if="theme.isDark" class="h-4.5 w-4.5" />
        <Moon v-else class="h-4.5 w-4.5" />
      </button>

      <template v-if="auth.isAuthenticated && auth.user">
        <div
          class="relative"
          @mouseenter="onUserMenuEnter"
          @mouseleave="onUserMenuLeave"
        >
          <button
            type="button"
            class="flex items-center gap-1.5 rounded-lg border border-border bg-card px-2 py-1.5 text-xs font-medium text-foreground hover:bg-secondary"
            :aria-expanded="userMenuOpen"
            @click="userMenuOpen = !userMenuOpen"
          >
            <span class="flex h-6 w-6 items-center justify-center rounded-full bg-primary text-primary-foreground">
              {{ auth.user.email.charAt(0).toUpperCase() }}
            </span>
            <span class="hidden sm:inline">{{ auth.user.email }}</span>
            <span class="hidden rounded bg-secondary px-1.5 py-0.5 text-[10px] uppercase tracking-wide text-muted-foreground sm:inline">
              {{ auth.user.role }}
            </span>
            <ChevronDown class="h-3.5 w-3.5" />
          </button>
          <Transition
            enter-active-class="transition duration-100"
            enter-from-class="opacity-0 -translate-y-1"
            enter-to-class="opacity-100 translate-y-0"
            leave-active-class="transition duration-100"
            leave-from-class="opacity-100 translate-y-0"
            leave-to-class="opacity-0 -translate-y-1"
          >
            <div
              v-if="userMenuOpen"
              class="absolute right-0 mt-1 w-56 rounded-lg border border-border bg-card p-1.5 shadow-lawu-md"
              role="menu"
            >
              <div class="border-b border-border px-2 py-1.5">
                <p class="truncate text-xs font-medium text-foreground">{{ auth.user.email }}</p>
                <p class="mt-0.5 inline-block rounded bg-accent/10 px-1.5 py-0.5 text-[10px] font-bold uppercase tracking-wide text-accent">
                  {{ auth.user.role }}
                </p>
              </div>
              <button
                type="button"
                class="mt-1 flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-xs text-foreground hover:bg-secondary"
                role="menuitem"
                @click="goSettings"
              >
                <SettingsIcon class="h-3.5 w-3.5" /> Pengaturan
              </button>
              <button
                type="button"
                class="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-xs text-destructive hover:bg-destructive/10"
                role="menuitem"
                @click="onLogout"
              >
                <LogOut class="h-3.5 w-3.5" /> Keluar
              </button>
            </div>
          </Transition>
        </div>
      </template>
      <button
        v-else
        type="button"
        @click="goLogin"
        class="inline-flex items-center gap-1.5 rounded-md bg-primary px-2.5 py-1.5 text-xs font-semibold text-primary-foreground hover:bg-primary/90"
      >
        <LogIn class="h-3.5 w-3.5" /> Masuk
      </button>
    </div>
  </header>
</template>
