<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { Copy, Check, Link2 } from 'lucide-vue-next'
import { useThemeStore } from '@/stores/theme'
import { useAuthStore } from '@/stores/auth'
import { LAWU_BACKEND_URL, LAWU_LLM_URL } from '@/lib/lawu-backend'

const themeStore = useThemeStore()
const auth = useAuthStore()

const baseUrl = computed(() => {
  if (typeof window === 'undefined') return ''
  return window.location.origin
})

const llmUrlInput = ref(LAWU_LLM_URL || '')
const llmModelInput = ref('lawu-2.5-fine')
const llmKeyInput = ref('')

const copiedBase = ref(false)
const copiedLlm = ref(false)

async function copy(text: string, which: 'base' | 'llm') {
  if (!text) return
  try {
    await navigator.clipboard.writeText(text)
    if (which === 'base') {
      copiedBase.value = true
      setTimeout(() => { copiedBase.value = false }, 2000)
    } else {
      copiedLlm.value = true
      setTimeout(() => { copiedLlm.value = false }, 2000)
    }
  } catch {
    // ignore
  }
}

onMounted(() => {
  // Load saved LLM config dari localStorage (best-effort).
  try {
    const saved = localStorage.getItem('lawu-llm-config')
    if (saved) {
      const obj = JSON.parse(saved)
      if (obj.url) llmUrlInput.value = obj.url
      if (obj.model) llmModelInput.value = obj.model
      if (obj.key) llmKeyInput.value = obj.key
    }
  } catch {
    // ignore
  }
})

function saveLlmConfig() {
  try {
    localStorage.setItem('lawu-llm-config', JSON.stringify({
      url: llmUrlInput.value,
      model: llmModelInput.value,
      key: llmKeyInput.value,
    }))
  } catch {
    // ignore
  }
}

const backendUrl = LAWU_BACKEND_URL
</script>

<template>
  <div class="mx-auto w-full max-w-3xl px-4 py-6 space-y-5">
    <!-- Base URL -->
    <section class="rounded-xl border border-border bg-card p-5 shadow-sm">
      <h2 class="flex items-center gap-2 text-base font-semibold text-foreground">
        <Link2 class="h-4 w-4 text-primary" /> Base URL
      </h2>
      <p class="mt-1.5 text-sm text-muted-foreground">
        Pakai URL ini di Opencode atau aplikasi lain sebagai Base URL API Lawu.
        Semua endpoint <code class="rounded bg-secondary px-1 py-0.5">/api/*</code> dilayani dari sini.
      </p>
      <div class="mt-3 flex items-stretch gap-2">
        <input
          :value="baseUrl"
          readonly
          type="text"
          class="h-10 flex-1 rounded-lg border border-border bg-background px-3 font-mono text-[12.5px] text-foreground"
          aria-label="Base URL saat ini"
        />
        <button
          type="button"
          @click="copy(baseUrl, 'base')"
          class="flex items-center gap-1.5 rounded-lg bg-primary px-3 text-sm font-medium text-primary-foreground hover:bg-primary/90"
        >
          <component :is="copiedBase ? Check : Copy" class="h-4 w-4" />
          <span>{{ copiedBase ? 'Tersalin' : 'Copy' }}</span>
        </button>
      </div>
      <p class="mt-2 text-[11px] text-muted-foreground">
        Catatan: backend default (proxy dev) di-render dari
        <code class="rounded bg-secondary px-1 py-0.5">VITE_LAWU_BACKEND_URL</code> =
        <code class="rounded bg-secondary px-1 py-0.5">{{ backendUrl || '(kosong — pakai nginx proxy)' }}</code>.
      </p>
    </section>

    <!-- Endpoint LLM -->
    <section class="rounded-xl border border-border bg-card p-5 shadow-sm">
      <h2 class="text-base font-semibold text-foreground">Endpoint LLM</h2>
      <p class="mt-1.5 text-sm text-muted-foreground">
        Isi alamat endpoint model Anda (format <code class="rounded bg-secondary px-1 py-0.5">/v1/chat/completions</code>)
        bila sudah tersedia. Kosongkan untuk tetap memakai mode simulasi.
      </p>
      <div class="mt-4 space-y-3">
        <div>
          <label class="text-xs text-muted-foreground" for="llm-url">Base URL LLM</label>
          <input
            id="llm-url"
            v-model="llmUrlInput"
            @change="saveLlmConfig"
            type="text"
            placeholder="http://<alamat-endpoint-model>:8080"
            class="mt-1 h-10 w-full rounded-lg border border-border bg-background px-3 text-sm focus:border-primary focus:outline-none"
          />
        </div>
        <div class="grid grid-cols-1 gap-3 sm:grid-cols-2">
          <div>
            <label class="text-xs text-muted-foreground" for="llm-model">Nama Model</label>
            <input
              id="llm-model"
              v-model="llmModelInput"
              @change="saveLlmConfig"
              type="text"
              class="mt-1 h-10 w-full rounded-lg border border-border bg-background px-3 text-sm focus:border-primary focus:outline-none"
            />
          </div>
          <div>
            <label class="text-xs text-muted-foreground" for="llm-key">API Key (opsional)</label>
            <input
              id="llm-key"
              v-model="llmKeyInput"
              @change="saveLlmConfig"
              type="password"
              placeholder="kosongkan bila tanpa key"
              class="mt-1 h-10 w-full rounded-lg border border-border bg-background px-3 text-sm focus:border-primary focus:outline-none"
            />
          </div>
        </div>
        <button
          type="button"
          @click="copy(llmUrlInput, 'llm')"
          :disabled="!llmUrlInput"
          class="inline-flex h-9 items-center gap-1.5 rounded-lg border border-border bg-secondary px-3 text-sm font-medium text-foreground hover:bg-secondary/70 disabled:opacity-50"
        >
          <component :is="copiedLlm ? Check : Copy" class="h-4 w-4" />
          {{ copiedLlm ? 'Tersalin' : 'Copy URL' }}
        </button>
      </div>
    </section>

    <!-- Tema -->
    <section class="rounded-xl border border-border bg-card p-5 shadow-sm">
      <h2 class="text-base font-semibold text-foreground">Tampilan</h2>
      <div class="mt-3 flex items-center justify-between rounded-lg border border-border px-4 py-3">
        <div>
          <p class="text-sm font-medium text-foreground">Mode Gelap</p>
          <p class="text-xs text-muted-foreground">Beralih antara tema terang (kertas bersih) dan gelap (nyaman di mata).</p>
        </div>
        <button
          type="button"
          @click="themeStore.toggle()"
          :aria-label="themeStore.isDark ? 'Aktifkan mode terang' : 'Aktifkan mode gelap'"
          class="relative h-6 w-11 rounded-full transition-colors"
          :class="themeStore.isDark ? 'bg-primary' : 'bg-secondary'"
        >
          <span
            class="absolute top-0.5 h-5 w-5 rounded-full bg-white shadow transition-transform"
            :class="themeStore.isDark ? 'translate-x-5' : 'translate-x-0.5'"
          />
        </button>
      </div>
    </section>

    <!-- Info akun -->
    <section v-if="auth.user" class="rounded-xl border border-border bg-card p-5 shadow-sm">
      <h2 class="text-base font-semibold text-foreground">Akun Saya</h2>
      <dl class="mt-3 grid grid-cols-1 gap-2 text-sm sm:grid-cols-2">
        <div>
          <dt class="text-xs text-muted-foreground">Email</dt>
          <dd class="font-medium text-foreground">{{ auth.user.email }}</dd>
        </div>
        <div>
          <dt class="text-xs text-muted-foreground">Role</dt>
          <dd>
            <span class="rounded-md bg-primary/10 px-2 py-0.5 text-xs font-medium text-primary">{{ auth.user.role }}</span>
          </dd>
        </div>
        <div>
          <dt class="text-xs text-muted-foreground">User ID</dt>
          <dd class="font-mono text-[12px] text-foreground">{{ auth.user.id }}</dd>
        </div>
        <div>
          <dt class="text-xs text-muted-foreground">Dibuat</dt>
          <dd class="text-foreground">{{ new Date(auth.user.createdAt).toLocaleString('id-ID') }}</dd>
        </div>
      </dl>
    </section>
  </div>
</template>
