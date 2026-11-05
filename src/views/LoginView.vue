<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { LoaderCircle, LogIn, AlertCircle, MessageSquare } from 'lucide-vue-next'
import { useAuthStore } from '@/stores/auth'
import { useThemeStore } from '@/stores/theme'
import lawuLogo from '@/assets/lawu-logo-256.png'

const router = useRouter()
const route = useRoute()
const auth = useAuthStore()
const theme = useThemeStore()

const email = ref('dev@lawu.ai.id')
const password = ref('12345678')
const loading = ref(false)
const error = ref<string | null>(null)

onMounted(() => {
  theme.init()
  // Jika sudah login, langsung redirect ke ?next= atau /monitor.
  if (auth.isAuthenticated) {
    const next = (route.query.next as string) || '/monitor'
    router.replace(next)
  }
})

async function onSubmit() {
  if (!email.value.trim() || !password.value) {
    error.value = 'Email dan password wajib diisi.'
    return
  }
  loading.value = true
  error.value = null
  try {
    await auth.login(email.value.trim(), password.value)
    const next = (route.query.next as string) || '/monitor'
    router.replace(next)
  } catch (err: unknown) {
    const e = err as { response?: { data?: { error?: string } }; message?: string }
    error.value = e?.response?.data?.error ?? e?.message ?? 'Login gagal. Periksa kredensial Anda.'
  } finally {
    loading.value = false
  }
}

function goGuest() {
  router.push('/chat')
}
</script>

<template>
  <div class="relative flex min-h-screen items-center justify-center overflow-hidden bg-background px-4 py-10">
    <!-- Mountain decorative — sisi kanan & kiri (opacity 0.1, pointer-events none) -->
    <div aria-hidden="true" class="pointer-events-none absolute inset-0 z-0">
      <!-- Sisi kiri -->
      <svg class="absolute bottom-0 left-0 h-[60%] w-1/2 opacity-[0.1] dark:opacity-[0.15]" viewBox="0 0 600 400" preserveAspectRatio="xMaxYMax meet" xmlns="http://www.w3.org/2000/svg">
        <defs>
          <linearGradient id="lawu-login-left" x1="0" x2="0" y1="0" y2="1">
            <stop offset="0%" stop-color="currentColor" class="text-primary" />
            <stop offset="100%" stop-color="currentColor" class="text-primary" stop-opacity="0.3" />
          </linearGradient>
        </defs>
        <path d="M0 400 L120 200 L200 280 L300 80 L380 240 L460 120 L560 220 L600 180 L600 400 Z" fill="url(#lawu-login-left)" />
        <path d="M0 400 L100 280 L180 320 L280 180 L360 290 L440 220 L520 280 L600 250 L600 400 Z" fill="currentColor" class="text-primary" fill-opacity="0.5" />
      </svg>
      <!-- Sisi kanan -->
      <svg class="absolute bottom-0 right-0 h-[60%] w-1/2 opacity-[0.1] dark:opacity-[0.15]" viewBox="0 0 600 400" preserveAspectRatio="xMinYMax meet" xmlns="http://www.w3.org/2000/svg" style="transform: scaleX(-1)">
        <defs>
          <linearGradient id="lawu-login-right" x1="0" x2="0" y1="0" y2="1">
            <stop offset="0%" stop-color="currentColor" class="text-primary" />
            <stop offset="100%" stop-color="currentColor" class="text-primary" stop-opacity="0.3" />
          </linearGradient>
        </defs>
        <path d="M0 400 L120 200 L200 280 L300 80 L380 240 L460 120 L560 220 L600 180 L600 400 Z" fill="url(#lawu-login-right)" />
        <path d="M0 400 L100 280 L180 320 L280 180 L360 290 L440 220 L520 280 L600 250 L600 400 Z" fill="currentColor" class="text-primary" fill-opacity="0.5" />
      </svg>
    </div>

    <div class="relative z-10 w-full max-w-md">
      <!-- Header logo -->
      <div class="flex flex-col items-center text-center">
        <img
          :src="lawuLogo"
          alt="Logo Lawu Fine"
          class="h-20 w-20 rounded-2xl shadow-lawu-md ring-1 ring-border"
        />
        <h1 class="mt-4 text-2xl font-extrabold tracking-tight text-foreground">
          Dashboard Lawu Fine
        </h1>
        <p class="mt-1 text-sm text-muted-foreground">
          Masuk untuk mengelola dashboard
        </p>
        <!-- Badge "AI Lokal Indonesia" -->
        <div class="mt-3 flex items-center gap-2">
          <span class="lawu-flag-badge">
            <span class="lawu-flag-red">AI</span>
            <span class="lawu-flag-white">LOKAL</span>
            <span class="lawu-flag-red">ID</span>
          </span>
          <span class="rounded-md bg-secondary px-2 py-1 text-[10px] font-medium text-secondary-foreground">
            Dibuat di Indonesia
          </span>
        </div>
      </div>

      <!-- Card form -->
      <form
        @submit.prevent="onSubmit"
        class="mt-6 rounded-2xl border border-border bg-card p-6 shadow-lawu-md"
      >
        <div class="space-y-4">
          <div>
            <label for="email" class="block text-xs font-medium text-muted-foreground">Email</label>
            <input
              id="email"
              v-model="email"
              type="email"
              autocomplete="email"
              required
              placeholder="anda@lawu.ai.id"
              class="mt-1 h-11 w-full rounded-lg border border-border bg-background px-3 text-sm shadow-sm focus:border-primary focus:outline-none focus:ring-2 focus:ring-primary/20"
            />
          </div>
          <div>
            <label for="password" class="block text-xs font-medium text-muted-foreground">Password</label>
            <input
              id="password"
              v-model="password"
              type="password"
              autocomplete="current-password"
              required
              placeholder="••••••••"
              class="mt-1 h-11 w-full rounded-lg border border-border bg-background px-3 text-sm shadow-sm focus:border-primary focus:outline-none focus:ring-2 focus:ring-primary/20"
            />
          </div>
        </div>

        <div
          v-if="error"
          class="mt-4 flex items-start gap-2 rounded-lg border border-destructive/40 bg-destructive/10 px-3 py-2 text-xs text-destructive"
        >
          <AlertCircle class="mt-0.5 h-4 w-4 shrink-0" />
          <span>{{ error }}</span>
        </div>

        <button
          type="submit"
          :disabled="loading"
          class="mt-5 flex h-11 w-full items-center justify-center gap-2 rounded-lg bg-primary px-4 text-sm font-semibold text-primary-foreground shadow-lawu-sm transition-colors hover:bg-primary/90 disabled:cursor-not-allowed disabled:opacity-60"
        >
          <LoaderCircle v-if="loading" class="h-4 w-4 animate-spin" />
          <LogIn v-else class="h-4 w-4" />
          Masuk
        </button>

        <p class="mt-3 text-center text-[11px] text-muted-foreground">
          Hint default: <code class="rounded bg-muted px-1 py-0.5 font-mono">dev@lawu.ai.id</code> /
          <code class="rounded bg-muted px-1 py-0.5 font-mono">12345678</code>
        </p>
      </form>

      <!-- Guest link -->
      <div class="mt-4 text-center">
        <button
          type="button"
          @click="goGuest"
          class="inline-flex items-center gap-1.5 text-xs font-medium text-primary hover:underline"
        >
          <MessageSquare class="h-3.5 w-3.5" />
          Hanya mau chat? → Klik di sini
        </button>
      </div>

      <p class="mt-6 text-center text-[10.5px] text-muted-foreground">
        © Lawu Fine · AI Lokal Indonesia · v3.6.0
      </p>
    </div>
  </div>
</template>
