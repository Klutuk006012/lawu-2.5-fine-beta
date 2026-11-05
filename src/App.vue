<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue'
import { RouterView, useRoute } from 'vue-router'
import Sidebar from './components/Sidebar.vue'
import Header from './components/Header.vue'
import { useThemeStore } from './stores/theme'
import { useAuthStore } from './stores/auth'
import { isBackendReachable } from './lib/lawu-backend'

const themeStore = useThemeStore()
const auth = useAuthStore()
const route = useRoute()

const sidebarOpen = ref(false)
const backendOnline = ref(false)
let backendTimer: ReturnType<typeof setInterval> | null = null

const isStandalone = computed(() => route.name === 'login')

async function checkBackend() {
  backendOnline.value = await isBackendReachable()
}

function toggleSidebar() {
  sidebarOpen.value = !sidebarOpen.value
}

function closeSidebar() {
  sidebarOpen.value = false
}

// Tutup drawer saat pindah route (mobile).
watch(
  () => route.fullPath,
  () => { sidebarOpen.value = false },
)

onMounted(async () => {
  themeStore.init()
  if (auth.token) {
    try {
      await auth.fetchMe()
    } catch {
      // ignore — fetchMe handles logout.
    }
  }
  checkBackend()
  backendTimer = setInterval(checkBackend, 30000)
})

onUnmounted(() => {
  if (backendTimer) clearInterval(backendTimer)
})
</script>

<template>
  <!-- Standalone (login) — tanpa sidebar/header -->
  <RouterView v-if="isStandalone" />

  <!-- Layout default -->
  <div v-else class="relative flex min-h-screen flex-col bg-background text-foreground">
    <!-- Background mountain decorative (subtle, pojok kanan bawah) -->
    <div
      aria-hidden="true"
      class="pointer-events-none fixed bottom-0 right-0 z-0 h-[420px] w-[520px] opacity-[0.035] dark:opacity-[0.05]"
    >
      <svg viewBox="0 0 520 420" xmlns="http://www.w3.org/2000/svg" preserveAspectRatio="xMaxYMax meet" class="h-full w-full">
        <defs>
          <linearGradient id="lawu-bg-mountain" x1="0" x2="0" y1="0" y2="1">
            <stop offset="0%" stop-color="currentColor" class="text-primary" />
            <stop offset="100%" stop-color="currentColor" class="text-primary" stop-opacity="0.4" />
          </linearGradient>
        </defs>
        <path d="M0 420 L80 250 L140 320 L210 130 L300 290 L380 180 L460 280 L520 220 L520 420 Z" fill="url(#lawu-bg-mountain)" />
        <path d="M0 420 L120 290 L200 330 L290 200 L380 320 L460 250 L520 310 L520 420 Z" fill="currentColor" class="text-primary" fill-opacity="0.5" />
      </svg>
    </div>

    <div class="relative z-10 flex min-h-0 flex-1">
      <!-- Sidebar desktop (w-72 untuk breathing room) -->
      <Sidebar class="sticky top-0 hidden h-screen w-72 shrink-0 md:block" />

      <!-- Sidebar mobile (drawer) dengan overlay backdrop -->
      <Transition
        enter-active-class="transition-opacity duration-200"
        enter-from-class="opacity-0"
        enter-to-class="opacity-100"
        leave-active-class="transition-opacity duration-200"
        leave-from-class="opacity-100"
        leave-to-class="opacity-0"
      >
        <div
          v-if="sidebarOpen"
          class="fixed inset-0 z-40 bg-black/40 backdrop-blur-sm md:hidden"
          aria-hidden="true"
          @click="closeSidebar"
        />
      </Transition>
      <Transition
        enter-active-class="transition-transform duration-200 ease-out"
        enter-from-class="-translate-x-full"
        enter-to-class="translate-x-0"
        leave-active-class="transition-transform duration-200 ease-in"
        leave-from-class="translate-x-0"
        leave-to-class="-translate-x-full"
      >
        <Sidebar
          v-if="sidebarOpen"
          class="fixed inset-y-0 left-0 z-50 h-screen w-72 shrink-0 shadow-2xl md:hidden"
        />
      </Transition>

      <!-- Area utama -->
      <div class="flex min-h-0 min-w-0 flex-1 flex-col">
        <Header :backend-online="backendOnline" @toggle-sidebar="toggleSidebar" />
        <main class="min-h-0 flex-1">
          <RouterView />
        </main>
        <!-- Sticky footer — status backend + tagline AI Lokal Indonesia -->
        <footer class="mt-auto flex flex-wrap items-center justify-between gap-2 border-t border-border bg-card/95 px-4 py-2.5 text-[11px] text-muted-foreground backdrop-blur">
          <p class="flex items-center gap-1.5">
            <span
              class="inline-block h-1.5 w-1.5 rounded-full"
              :class="backendOnline ? 'bg-primary' : 'bg-destructive'"
              aria-hidden
            />
            <span class="font-medium text-foreground">Lawu Fine</span>
            ·
            AI Lokal Indonesia
            ·
            Dashboard v3.7.1
          </p>
          <p class="flex items-center gap-1.5">
            <span
              class="inline-block h-1.5 w-1.5 rounded-full"
              :class="backendOnline ? 'bg-primary' : 'bg-muted-foreground'"
              aria-hidden
            />
            {{ backendOnline ? 'Backend terhubung' : 'Backend offline' }}
          </p>
        </footer>
      </div>
    </div>
  </div>
</template>
