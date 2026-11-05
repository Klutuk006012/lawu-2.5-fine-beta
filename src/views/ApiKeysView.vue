<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import {
  KeyRound, Plus, Trash2, X, Loader2, CheckCircle2, AlertCircle,
  Copy, ShieldCheck, ShieldAlert,
} from 'lucide-vue-next'
import { api } from '@/lib/api'

interface ApiKeyRow {
  id: string
  name: string
  prefix: string
  scopes: string[]
  createdAt: string
  lastUsedAt: string | null
  revokedAt: string | null
}

interface KeysResponse {
  keys: ApiKeyRow[]
}

interface CreateKeyResponse {
  id: string
  name: string
  key: string
  prefix: string
  scopes: string[]
  createdAt: string
}

const keys = ref<ApiKeyRow[]>([])
const loading = ref(false)
const toast = ref<{ msg: string; ok: boolean } | null>(null)

const SCOPE_OPTIONS = ['chat', 'search', 'ebooks', 'corpus', 'monitor', 'admin']

// Create modal
const modalOpen = ref(false)
const formName = ref('')
const formScopes = ref<string[]>(['chat'])
const formSubmitting = ref(false)
const formError = ref<string | null>(null)

// Plaintext reveal modal
const reveal = ref<{ key: string; name: string; prefix: string } | null>(null)
const copied = ref(false)

// Delete confirm
const deleteTarget = ref<ApiKeyRow | null>(null)

function showToast(msg: string, ok = true) {
  toast.value = { msg, ok }
  setTimeout(() => { toast.value = null }, 5000)
}

async function load() {
  loading.value = true
  try {
    const res = await api.get<KeysResponse>('/api/api-keys')
    keys.value = res.data.keys ?? []
  } catch (err: unknown) {
    const e = err as { response?: { data?: { error?: string } } }
    showToast(e?.response?.data?.error ?? 'Gagal memuat API keys.', false)
  } finally {
    loading.value = false
  }
}

function openCreate() {
  formName.value = ''
  formScopes.value = ['chat']
  formError.value = null
  modalOpen.value = true
}

function closeModal() {
  modalOpen.value = false
  formError.value = null
}

function toggleScope(scope: string) {
  const idx = formScopes.value.indexOf(scope)
  if (idx >= 0) formScopes.value.splice(idx, 1)
  else formScopes.value.push(scope)
}

async function submitForm() {
  if (!formName.value.trim()) {
    formError.value = 'Nama key wajib diisi.'
    return
  }
  formSubmitting.value = true
  formError.value = null
  try {
    const res = await api.post<CreateKeyResponse>('/api/api-keys', {
      name: formName.value.trim(),
      scopes: formScopes.value,
    })
    keys.value.unshift({
      id: res.data.id,
      name: res.data.name,
      prefix: res.data.prefix,
      scopes: res.data.scopes,
      createdAt: res.data.createdAt,
      lastUsedAt: null,
      revokedAt: null,
    })
    reveal.value = { key: res.data.key, name: res.data.name, prefix: res.data.prefix }
    modalOpen.value = false
  } catch (err: unknown) {
    const e = err as { response?: { data?: { error?: string } } }
    formError.value = e?.response?.data?.error ?? 'Gagal membuat API key.'
  } finally {
    formSubmitting.value = false
  }
}

async function copyReveal() {
  if (!reveal.value) return
  try {
    await navigator.clipboard.writeText(reveal.value.key)
    copied.value = true
    setTimeout(() => { copied.value = false }, 2000)
  } catch {
    showToast('Gagal menyalin ke clipboard.', false)
  }
}

function closeReveal() {
  reveal.value = null
}

async function confirmDelete() {
  if (!deleteTarget.value) return
  const target = deleteTarget.value
  try {
    await api.delete(`/api/api-keys/${target.id}`)
    keys.value = keys.value.filter((k) => k.id !== target.id)
    showToast(`Key ${target.name} di-revoke.`)
  } catch (err: unknown) {
    const e = err as { response?: { data?: { error?: string } } }
    showToast(e?.response?.data?.error ?? 'Gagal me-revoke key.', false)
  } finally {
    deleteTarget.value = null
  }
}

function fmtDate(s: string | null): string {
  if (!s) return '—'
  try {
    return new Date(s).toLocaleString('id-ID', { dateStyle: 'medium', timeStyle: 'short' })
  } catch {
    return s
  }
}

const activeCount = computed(() => keys.value.filter((k) => !k.revokedAt).length)
const revokedCount = computed(() => keys.value.filter((k) => k.revokedAt).length)

onMounted(load)
</script>

<template>
  <div class="mx-auto w-full max-w-5xl px-4 py-6">
    <!-- Header -->
    <div class="flex flex-wrap items-end justify-between gap-2">
      <div>
        <div class="flex items-center gap-2 text-[11px] font-semibold uppercase tracking-widest text-red-600 dark:text-red-400">
          <span class="inline-block h-2 w-2 rounded-sm bg-red-600 dark:bg-red-400" aria-hidden />
          Akses API
        </div>
        <h1 class="mt-1 text-lg font-semibold tracking-tight text-foreground">API Keys</h1>
        <p class="mt-0.5 text-sm text-muted-foreground">
          Generate API key untuk dipakai sebagai <code class="rounded bg-secondary px-1 py-0.5">Authorization: Bearer lawu_...</code>
          di Opencode atau aplikasi lain untuk akses API Lawu.
        </p>
      </div>
      <button
        type="button"
        @click="openCreate"
        class="inline-flex h-10 items-center gap-2 rounded-lg bg-primary px-4 text-sm font-semibold text-primary-foreground shadow-sm hover:bg-primary/90"
      >
        <Plus class="h-4 w-4" /> Generate Key Baru
      </button>
    </div>

    <!-- Stat cards -->
    <div class="mt-4 grid grid-cols-2 gap-3 sm:grid-cols-3">
      <div class="rounded-xl border border-border bg-card p-4 shadow-sm">
        <p class="flex items-center gap-1.5 text-xs font-medium text-muted-foreground"><KeyRound class="h-3.5 w-3.5" /> Total Key</p>
        <p class="mt-1 text-2xl font-bold tracking-tight text-foreground">{{ keys.length }}</p>
      </div>
      <div class="rounded-xl border border-border bg-card p-4 shadow-sm">
        <p class="flex items-center gap-1.5 text-xs font-medium text-muted-foreground"><ShieldCheck class="h-3.5 w-3.5" /> Active</p>
        <p class="mt-1 text-2xl font-bold tracking-tight text-foreground">{{ activeCount }}</p>
      </div>
      <div class="rounded-xl border border-border bg-card p-4 shadow-sm">
        <p class="flex items-center gap-1.5 text-xs font-medium text-muted-foreground"><ShieldAlert class="h-3.5 w-3.5" /> Revoked</p>
        <p class="mt-1 text-2xl font-bold tracking-tight text-foreground">{{ revokedCount }}</p>
      </div>
    </div>

    <!-- Toast -->
    <div
      v-if="toast"
      class="mt-4 flex items-start gap-2 rounded-lg border px-3 py-2 text-sm"
      :class="toast.ok
        ? 'border-emerald-500/30 bg-emerald-500/10 text-emerald-700 dark:text-emerald-400'
        : 'border-destructive/40 bg-destructive/10 text-destructive'"
    >
      <component :is="toast.ok ? CheckCircle2 : AlertCircle" class="mt-0.5 h-4 w-4 shrink-0" />
      <span>{{ toast.msg }}</span>
    </div>

    <!-- Table -->
    <div class="mt-4 overflow-hidden rounded-xl border border-border bg-card shadow-sm">
      <div v-if="loading" class="flex items-center gap-2 px-4 py-10 text-sm text-muted-foreground">
        <Loader2 class="h-4 w-4 animate-spin" /> Memuat API keys…
      </div>
      <div v-else-if="keys.length === 0" class="px-4 py-10 text-center text-sm text-muted-foreground">
        Belum ada API key. Klik "Generate Key Baru" untuk membuat yang pertama.
      </div>
      <div v-else class="overflow-x-auto">
        <table class="min-w-full divide-y divide-border text-left text-sm">
          <thead class="bg-secondary/60 text-[11px] uppercase tracking-wider text-muted-foreground">
            <tr>
              <th class="px-4 py-2.5 font-medium">Name</th>
              <th class="px-4 py-2.5 font-medium">Prefix</th>
              <th class="px-4 py-2.5 font-medium">Scopes</th>
              <th class="px-4 py-2.5 font-medium">Status</th>
              <th class="px-4 py-2.5 font-medium">Dibuat</th>
              <th class="px-4 py-2.5 font-medium">Dipakai Terakhir</th>
              <th class="px-4 py-2.5 font-medium text-right">Aksi</th>
            </tr>
          </thead>
          <tbody class="divide-y divide-border">
            <tr v-for="k in keys" :key="k.id" class="hover:bg-secondary/40">
              <td class="px-4 py-2.5 font-medium text-foreground">{{ k.name }}</td>
              <td class="px-4 py-2.5 font-mono text-[12px] text-muted-foreground">{{ k.prefix }}…</td>
              <td class="px-4 py-2.5">
                <div class="flex flex-wrap gap-1">
                  <span
                    v-for="sc in k.scopes"
                    :key="sc"
                    class="rounded bg-secondary px-1.5 py-0.5 text-[10px] font-medium text-muted-foreground"
                  >{{ sc }}</span>
                </div>
              </td>
              <td class="px-4 py-2.5">
                <span
                  class="inline-flex items-center gap-1 rounded-md px-2 py-0.5 text-[10.5px] font-medium"
                  :class="k.revokedAt
                    ? 'bg-destructive/10 text-destructive'
                    : 'bg-emerald-500/10 text-emerald-700 dark:text-emerald-400'"
                >
                  {{ k.revokedAt ? 'Revoked' : 'Active' }}
                </span>
              </td>
              <td class="px-4 py-2.5 text-muted-foreground">{{ fmtDate(k.createdAt) }}</td>
              <td class="px-4 py-2.5 text-muted-foreground">{{ fmtDate(k.lastUsedAt) }}</td>
              <td class="px-4 py-2.5 text-right">
                <button
                  v-if="!k.revokedAt"
                  type="button"
                  @click="deleteTarget = k"
                  class="rounded-md p-1.5 text-muted-foreground hover:bg-destructive/10 hover:text-destructive"
                  :aria-label="`Revoke key ${k.name}`"
                >
                  <Trash2 class="h-4 w-4" />
                </button>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

    <!-- Create modal -->
    <div
      v-if="modalOpen"
      class="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-4"
      role="dialog"
      aria-modal="true"
      @click.self="closeModal"
    >
      <div class="w-full max-w-md rounded-2xl border border-border bg-card p-5 shadow-xl">
        <div class="flex items-center justify-between">
          <h3 class="text-sm font-semibold text-foreground">Generate API Key Baru</h3>
          <button type="button" @click="closeModal" class="rounded-md p-1.5 text-muted-foreground hover:bg-secondary" aria-label="Tutup">
            <X class="h-4 w-4" />
          </button>
        </div>
        <form class="mt-4 space-y-3" @submit.prevent="submitForm">
          <div>
            <label class="text-xs text-muted-foreground" for="k-name">Nama Key</label>
            <input
              id="k-name"
              v-model="formName"
              type="text"
              required
              placeholder="mis. Opencode CLI"
              class="mt-1 h-10 w-full rounded-lg border border-border bg-background px-3 text-sm focus:border-primary focus:outline-none"
            />
          </div>
          <div>
            <p class="text-xs text-muted-foreground">Scopes</p>
            <div class="mt-1.5 flex flex-wrap gap-1.5">
              <button
                v-for="sc in SCOPE_OPTIONS"
                :key="sc"
                type="button"
                @click="toggleScope(sc)"
                class="rounded-md border px-2.5 py-1 text-xs font-medium transition-colors"
                :class="formScopes.includes(sc)
                  ? 'border-primary bg-primary/10 text-primary'
                  : 'border-border bg-background text-muted-foreground hover:bg-secondary'"
              >
                {{ sc }}
              </button>
            </div>
          </div>
          <div
            v-if="formError"
            class="rounded-lg border border-destructive/40 bg-destructive/10 px-3 py-2 text-xs text-destructive"
          >
            {{ formError }}
          </div>
          <div class="flex justify-end gap-2 pt-1">
            <button
              type="button"
              @click="closeModal"
              class="rounded-lg border border-border bg-background px-3 py-1.5 text-sm text-foreground hover:bg-secondary"
            >
              Batal
            </button>
            <button
              type="submit"
              :disabled="formSubmitting"
              class="flex items-center gap-1.5 rounded-lg bg-primary px-3 py-1.5 text-sm font-medium text-primary-foreground hover:bg-primary/90 disabled:opacity-50"
            >
              <Loader2 v-if="formSubmitting" class="h-4 w-4 animate-spin" />
              Generate
            </button>
          </div>
        </form>
      </div>
    </div>

    <!-- Reveal plaintext modal -->
    <div
      v-if="reveal"
      class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4"
      role="dialog"
      aria-modal="true"
    >
      <div class="w-full max-w-lg rounded-2xl border border-border bg-card p-5 shadow-xl">
        <div class="flex items-start gap-3">
          <div class="flex h-10 w-10 shrink-0 items-center justify-center rounded-full bg-amber-500/15 text-amber-600 dark:text-amber-400">
            <ShieldAlert class="h-5 w-5" />
          </div>
          <div class="min-w-0 flex-1">
            <h3 class="text-sm font-semibold text-foreground">Key berhasil dibuat: {{ reveal.name }}</h3>
            <p class="mt-1 text-xs text-muted-foreground">
              Salin sekarang — key ini <span class="font-semibold text-foreground">tidak akan ditampilkan lagi</span>.
            </p>
          </div>
        </div>
        <div class="mt-4 flex items-stretch gap-2">
          <input
            :value="reveal.key"
            readonly
            type="text"
            class="h-10 flex-1 rounded-lg border border-border bg-background px-3 font-mono text-[12px] text-foreground"
            @focus="(e) => (e.target as HTMLInputElement | null)?.select()"
          />
          <button
            type="button"
            @click="copyReveal"
            class="flex items-center gap-1.5 rounded-lg bg-primary px-3 text-sm font-medium text-primary-foreground hover:bg-primary/90"
          >
            <Copy class="h-4 w-4" />
            <span v-if="copied">Tersalin!</span>
            <span v-else>Copy</span>
          </button>
        </div>
        <div class="mt-3 rounded-lg bg-secondary/60 px-3 py-2 text-[11px] text-muted-foreground">
          <p class="font-medium text-foreground">Contoh pemakaian:</p>
          <pre class="mt-1 overflow-x-auto font-mono text-[11px]">Authorization: Bearer {{ reveal.key }}</pre>
        </div>
        <div class="mt-4 flex justify-end">
          <button
            type="button"
            @click="closeReveal"
            class="rounded-lg bg-primary px-3 py-1.5 text-sm font-medium text-primary-foreground hover:bg-primary/90"
          >
            Saya sudah menyimpan
          </button>
        </div>
      </div>
    </div>

    <!-- Delete confirm modal -->
    <div
      v-if="deleteTarget"
      class="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-4"
      role="dialog"
      aria-modal="true"
      @click.self="deleteTarget = null"
    >
      <div class="w-full max-w-sm rounded-2xl border border-border bg-card p-5 shadow-xl">
        <h3 class="text-sm font-semibold text-foreground">Revoke API key?</h3>
        <p class="mt-1 text-xs text-muted-foreground">
          Key <span class="font-medium text-foreground">{{ deleteTarget.name }}</span> ({{
            deleteTarget.prefix
          }}…) akan di-revoke permanen. Aplikasi yang memakai key ini akan kehilangan akses.
        </p>
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
            Revoke
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
