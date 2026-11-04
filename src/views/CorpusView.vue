<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import {
  Globe, Plus, Loader2, RefreshCw, Trash2, Database,
  UploadCloud, AlertTriangle, CheckCircle2, ExternalLink, Layers,
} from 'lucide-vue-next'

interface CorpusEntry {
  id: string
  url: string
  category: string
  source: 'seed' | 'custom'
  addedAt: string | null
  crawledAt: string | null
  pageTitle: string | null
  pagesCrawled: number | null
  errorCount: number | null
}

interface JobInfo {
  running: boolean
  pid?: number | null
  startedAt?: string | null
  lastResult?: Record<string, unknown>
  file?: string | null
}

const entries = ref<CorpusEntry[]>([])
const loading = ref(true)
const errorMsg = ref('')
const okMsg = ref('')
const urlInput = ref('')
const categoryInput = ref('custom')
const filterCategory = ref('semua')
const adding = ref(false)
const crawling = ref(false)
const ingesting = ref(false)
const status = ref<{ crawl: JobInfo; extract: JobInfo; ingest: JobInfo } | null>(null)
let pollTimer: number | undefined

const CATEGORIES = ['utama', 'berita', 'security', 'ai', 'custom', 'search-engine'] as const

const CAT_STYLE: Record<string, string> = {
  utama: 'bg-emerald-500/10 text-emerald-700 dark:text-emerald-400 border-emerald-500/20',
  berita: 'bg-amber-500/10 text-amber-700 dark:text-amber-400 border-amber-500/20',
  security: 'bg-red-600/10 text-red-700 dark:text-red-400 border-red-600/20',
  ai: 'bg-violet-500/10 text-violet-700 dark:text-violet-400 border-violet-500/20',
  custom: 'bg-teal-500/10 text-teal-700 dark:text-teal-400 border-teal-500/20',
  'search-engine': 'bg-slate-500/10 text-slate-700 dark:text-slate-400 border-slate-500/20',
}

const filtered = computed(() =>
  filterCategory.value === 'semua'
    ? entries.value
    : entries.value.filter((e) => e.category === filterCategory.value)
)

const crawledCount = computed(() => entries.value.filter((e) => e.crawledAt).length)
const seedCount = computed(() => entries.value.filter((e) => e.source === 'seed').length)
const customCount = computed(() => entries.value.filter((e) => e.source === 'custom').length)

async function load() {
  loading.value = true
  try {
    const res = await fetch('/api/corpus/web')
    if (res.ok) {
      const data = await res.json()
      entries.value = Array.isArray(data.entries) ? data.entries : []
    } else {
      errorMsg.value = `API korpus tidak tersedia (HTTP ${res.status}). Pastikan lawu-corpus.service aktif.`
    }
  } catch {
    errorMsg.value = 'Tidak bisa menghubungi /api/corpus/web (lawu-corpus :3002).'
  } finally {
    loading.value = false
  }
}

async function loadStatus() {
  try {
    const res = await fetch('/api/corpus/status')
    if (res.ok) status.value = await res.json()
  } catch { /* silent */ }
}

function flash(msg: string, ok = true) {
  if (ok) { okMsg.value = msg; errorMsg.value = '' } else { errorMsg.value = msg; okMsg.value = '' }
  window.setTimeout(() => { okMsg.value = ''; errorMsg.value = '' }, 6000)
}

async function addUrl() {
  const url = urlInput.value.trim()
  if (!url) return
  adding.value = true
  try {
    const res = await fetch('/api/corpus/web', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ url, category: categoryInput.value }),
    })
    const data = await res.json().catch(() => ({}))
    if (res.status === 201) {
      flash(`URL ditambahkan ke korpus ${categoryInput.value}. Klik "Mulai Crawl" untuk mengambil isinya.`)
      urlInput.value = ''
      await load()
    } else if (res.status === 409) {
      flash('URL sudah ada di daftar korpus.', false)
    } else {
      flash(data.error ?? `Gagal menambah URL (HTTP ${res.status}).`, false)
    }
  } catch {
    flash('Gagal menghubungi API korpus.', false)
  } finally {
    adding.value = false
  }
}

async function removeUrl(e: CorpusEntry) {
  if (e.source !== 'custom') return
  try {
    const res = await fetch(`/api/corpus/web?url=${encodeURIComponent(e.url)}`, { method: 'DELETE' })
    if (res.ok) { flash('URL dihapus dari korpus.'); await load() }
  } catch { flash('Gagal menghapus URL.', false) }
}

async function startCrawl(force = false) {
  crawling.value = true
  try {
    const res = await fetch(`/api/corpus/web/crawl${force ? '?force=true' : ''}`, { method: 'POST' })
    const data = await res.json().catch(() => ({}))
    if (res.status === 202) {
      flash('Crawler dimulai di server. Pantau status di bawah — hasil masuk ke web_corpus.jsonl.')
      await loadStatus()
    } else {
      flash(data.error ?? 'Crawl tidak bisa dimulai.', false)
    }
  } catch { flash('Gagal menghubungi API korpus.', false) } finally { crawling.value = false }
}

async function startIngest(kind: 'web' | 'skill') {
  ingesting.value = true
  try {
    const res = await fetch('/api/corpus/ingest', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ kind }),
    })
    const data = await res.json().catch(() => ({}))
    if (res.status === 202) {
      flash(`Ingest ${kind} berjalan. Setelah selesai, restart lawu-server bila hasil belum muncul di Pencarian.`)
      await loadStatus()
    } else {
      flash(data.error ?? 'Ingest tidak bisa dimulai.', false)
    }
  } catch { flash('Gagal menghubungi API korpus.', false) } finally { ingesting.value = false }
}

function fmtTime(t: string | null): string {
  if (!t) return '—'
  try { return new Date(t).toLocaleString('id-ID', { dateStyle: 'medium', timeStyle: 'short' }) }
  catch { return t }
}

function fmtNum(n: number | null): string {
  if (!n) return '—'
  return n >= 1000 ? `${(n / 1000).toFixed(1)}k` : String(n)
}

onMounted(() => { load(); loadStatus() })
onUnmounted(() => { if (pollTimer) window.clearInterval(pollTimer) })
</script>

<template>
  <div class="mx-auto w-full max-w-4xl px-4 py-6">
    <!-- Kepala halaman -->
    <div class="mb-5">
      <div class="flex items-center gap-2 text-[11px] font-semibold uppercase tracking-widest text-red-600 dark:text-red-400">
        <span class="inline-block h-2 w-2 rounded-sm bg-red-600 dark:bg-red-400" aria-hidden />
        Korpus &amp; Pembelajaran Mandiri
      </div>
      <h2 class="mt-1.5 text-xl font-bold tracking-tight text-foreground sm:text-2xl">
        Web Korpus Lawu Fine
      </h2>
      <p class="mt-1 text-sm text-muted-foreground">
        Tambahkan sumber web yang wajib dikuasai Lawu Fine — teknologi, keamanan, berita,
        hingga teknik Advanced Google Search. Prinsip paten: <span class="font-medium text-foreground">gratis &amp; open source</span>.
      </p>
    </div>

    <!-- Notifikasi -->
    <p v-if="okMsg" class="mb-4 flex items-start gap-2 rounded-lg border border-emerald-500/30 bg-emerald-500/10 px-3 py-2 text-sm text-emerald-700 dark:text-emerald-400">
      <CheckCircle2 class="mt-0.5 h-4 w-4 shrink-0" /> {{ okMsg }}
    </p>
    <p v-if="errorMsg" class="mb-4 flex items-start gap-2 rounded-lg border border-red-600/30 bg-red-600/10 px-3 py-2 text-sm text-red-700 dark:text-red-400">
      <AlertTriangle class="mt-0.5 h-4 w-4 shrink-0" /> {{ errorMsg }}
    </p>

    <!-- Form Tambah Web Korpus -->
    <section aria-labelledby="add-corpus-title" class="rounded-xl border border-border bg-card p-4 shadow-sm sm:p-6">
      <h3 id="add-corpus-title" class="text-sm font-semibold text-foreground">Tambah Web Korpus</h3>
      <p class="mt-0.5 text-xs text-muted-foreground">Tempel URL sumber baru, pilih kategori, lalu simpan.</p>
      <form class="mt-3 flex flex-col gap-2.5 sm:flex-row" @submit.prevent="addUrl">
        <label class="sr-only" for="corpus-url">URL sumber</label>
        <input
          id="corpus-url" v-model="urlInput" type="url" required
          placeholder="https://contoh.id/dokumentasi"
          class="h-10 w-full flex-1 rounded-lg border border-border bg-background px-3 text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-red-600/40"
        />
        <label class="sr-only" for="corpus-category">Kategori</label>
        <select
          id="corpus-category" v-model="categoryInput"
          class="h-10 rounded-lg border border-border bg-background px-2.5 text-sm text-foreground focus:outline-none focus:ring-2 focus:ring-red-600/40"
        >
          <option v-for="c in CATEGORIES" :key="c" :value="c">{{ c }}</option>
        </select>
        <button
          type="submit" :disabled="adding || !urlInput.trim()"
          class="inline-flex h-10 items-center justify-center gap-1.5 rounded-lg bg-red-600 px-4 text-sm font-semibold text-white shadow-sm transition-colors hover:bg-red-700 disabled:cursor-not-allowed disabled:opacity-50"
        >
          <Loader2 v-if="adding" class="h-4 w-4 animate-spin" />
          <Plus v-else class="h-4 w-4" />
          Tambah
        </button>
      </form>
    </section>

    <!-- Aksi Korpus -->
    <section aria-labelledby="aksi-corpus-title" class="mt-4 rounded-xl border border-border bg-card p-4 shadow-sm sm:p-6">
      <div class="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h3 id="aksi-corpus-title" class="text-sm font-semibold text-foreground">Aksi Korpus</h3>
          <p class="mt-0.5 text-xs text-muted-foreground">
            Crawl mengambil isi web ke JSONL &middot; Ingest memasukkannya ke index pencarian Tantivy.
          </p>
        </div>
        <div class="flex flex-wrap items-center gap-2">
          <button
            type="button" :disabled="crawling"
            class="inline-flex h-9 items-center gap-1.5 rounded-lg border border-border bg-secondary px-3 text-sm font-medium text-foreground transition-colors hover:bg-secondary/70 disabled:opacity-50"
            @click="startCrawl(false)"
          >
            <Loader2 v-if="crawling" class="h-4 w-4 animate-spin" />
            <UploadCloud v-else class="h-4 w-4" />
            Mulai Crawl
          </button>
          <button
            type="button" :disabled="crawling"
            class="inline-flex h-9 items-center gap-1.5 rounded-lg border border-border bg-secondary px-3 text-sm font-medium text-foreground transition-colors hover:bg-secondary/70 disabled:opacity-50"
            @click="startCrawl(true)"
          >
            <RefreshCw class="h-4 w-4" /> Paksa Re-crawl
          </button>
          <button
            type="button" :disabled="ingesting"
            class="inline-flex h-9 items-center gap-1.5 rounded-lg bg-primary px-3 text-sm font-semibold text-primary-foreground shadow-sm transition-colors hover:bg-primary/90 disabled:opacity-50"
            @click="startIngest('web')"
          >
            <Loader2 v-if="ingesting" class="h-4 w-4 animate-spin" />
            <Database v-else class="h-4 w-4" /> Ingest Web ke Index
          </button>
          <button
            type="button" :disabled="ingesting" title="Masukkan materi Advanced Google Search ke index"
            class="inline-flex h-9 items-center gap-1.5 rounded-lg border border-border bg-secondary px-3 text-sm font-medium text-foreground transition-colors hover:bg-secondary/70 disabled:opacity-50"
            @click="startIngest('skill')"
          >
            <Layers class="h-4 w-4" /> Ingest Skill
          </button>
        </div>
      </div>

      <!-- Status job -->
      <div v-if="status" class="mt-4 grid gap-2 sm:grid-cols-2">
        <div class="rounded-lg border border-border bg-background px-3 py-2.5">
          <p class="flex items-center justify-between text-xs font-medium text-muted-foreground">
            <span>Crawler Web</span>
            <span class="inline-flex items-center gap-1.5 font-semibold"
                  :class="status.crawl?.running ? 'text-amber-600 dark:text-amber-400' : 'text-emerald-600 dark:text-emerald-400'">
              <span class="inline-block h-1.5 w-1.5 rounded-full"
                    :class="status.crawl?.running ? 'animate-pulse bg-amber-500' : 'bg-emerald-500'" aria-hidden />
              {{ status.crawl?.running ? `Berjalan (pid ${status.crawl?.pid})` : 'Siaga' }}
            </span>
          </p>
          <p class="mt-1 text-[11px] text-muted-foreground">
            Crawl terakhir: {{ fmtTime((status.crawl?.lastResult as any)?.finished_at as string ?? null) }}
            <template v-if="(status.crawl?.lastResult as any)?.pages != null">
              &middot; {{ (status.crawl!.lastResult as any).pages }} halaman
            </template>
          </p>
        </div>
        <div class="rounded-lg border border-border bg-background px-3 py-2.5">
          <p class="flex items-center justify-between text-xs font-medium text-muted-foreground">
            <span>Ingest ke Tantivy</span>
            <span class="inline-flex items-center gap-1.5 font-semibold"
                  :class="status.ingest?.running ? 'text-amber-600 dark:text-amber-400' : 'text-emerald-600 dark:text-emerald-400'">
              <span class="inline-block h-1.5 w-1.5 rounded-full"
                    :class="status.ingest?.running ? 'animate-pulse bg-amber-500' : 'bg-emerald-500'" aria-hidden />
              {{ status.ingest?.running ? `Berjalan (pid ${status.ingest?.pid})` : 'Siaga' }}
            </span>
          </p>
          <p class="mt-1 text-[11px] text-muted-foreground">
            {{ status.ingest?.file ? `Sumber: ${status.ingest.file.split('/').pop()}` : 'Belum ada ingest lewat UI' }}
          </p>
        </div>
      </div>
    </section>

    <!-- Daftar Korpus -->
    <section aria-labelledby="daftar-corpus-title" class="mt-4 rounded-xl border border-border bg-card shadow-sm sm:p-6 p-4">
      <div class="flex flex-wrap items-center justify-between gap-2">
        <h3 id="daftar-corpus-title" class="text-sm font-semibold text-foreground">
          Daftar Sumber
          <span class="ml-1.5 rounded-md bg-secondary px-1.5 py-0.5 text-xs font-medium text-muted-foreground">{{ entries.length }}</span>
        </h3>
        <div class="flex flex-wrap items-center gap-3 text-[11px] text-muted-foreground">
          <span class="inline-flex items-center gap-1"><span class="h-1.5 w-1.5 rounded-full bg-emerald-500" /> {{ crawledCount }} ter-crawl</span>
          <span class="inline-flex items-center gap-1"><span class="h-1.5 w-1.5 rounded-full bg-primary" /> {{ seedCount }} seed resmi</span>
          <span class="inline-flex items-center gap-1"><span class="h-1.5 w-1.5 rounded-full bg-red-500" /> {{ customCount }} custom</span>
          <button type="button" class="inline-flex items-center gap-1 rounded-md p-1 hover:bg-secondary" aria-label="Muat ulang daftar" @click="load">
            <RefreshCw class="h-3.5 w-3.5" :class="loading ? 'animate-spin' : ''" />
          </button>
        </div>
      </div>

      <!-- Filter kategori -->
      <div class="mt-3 flex flex-wrap gap-1.5">
        <button
          v-for="c in ['semua', ...CATEGORIES]" :key="c" type="button"
          class="rounded-full border px-2.5 py-1 text-xs font-medium transition-colors"
          :class="filterCategory === c
            ? 'border-red-600 bg-red-600 text-white'
            : 'border-border bg-background text-muted-foreground hover:bg-secondary'"
          @click="filterCategory = c"
        >{{ c }}</button>
      </div>

      <!-- List -->
      <div class="mt-3 max-h-96 overflow-y-auto pr-1">
        <p v-if="loading" class="flex items-center gap-2 py-6 text-sm text-muted-foreground">
          <Loader2 class="h-4 w-4 animate-spin" /> Memuat daftar korpus&hellip;
        </p>
        <p v-else-if="filtered.length === 0" class="py-6 text-sm text-muted-foreground">
          Tidak ada sumber pada kategori ini.
        </p>
        <ul v-else class="divide-y divide-border">
          <li v-for="e in filtered" :key="e.id" class="group flex items-start gap-3 py-2.5">
            <span class="mt-0.5 rounded-md border px-1.5 py-0.5 text-[10px] font-semibold uppercase tracking-wide"
                  :class="CAT_STYLE[e.category] ?? CAT_STYLE.custom">{{ e.category }}</span>
            <div class="min-w-0 flex-1">
              <p class="flex items-center gap-1.5 truncate text-sm font-medium text-foreground">
                {{ e.pageTitle ?? (e.url.split('://')[1] ?? e.url) }}
                <a :href="e.url" target="_blank" rel="noopener" class="text-muted-foreground hover:text-foreground" :aria-label="`Buka ${e.url}`">
                  <ExternalLink class="h-3 w-3" />
                </a>
              </p>
              <p class="truncate text-xs text-muted-foreground">{{ e.url }}</p>
              <p class="mt-0.5 text-[11px] text-muted-foreground">
                <span :class="e.crawledAt ? 'text-emerald-600 dark:text-emerald-400' : 'text-amber-600 dark:text-amber-400'">
                  {{ e.crawledAt ? `Tercrawl ${fmtTime(e.crawledAt)}` : 'Belum ter-crawl' }}
                </span>
                <template v-if="e.pagesCrawled"> &middot; {{ fmtNum(e.pagesCrawled) }} karakter</template>
                <template v-if="e.source === 'custom'"> &middot; via UI</template>
              </p>
            </div>
            <button
              v-if="e.source === 'custom'" type="button"
              class="rounded-md p-1.5 text-muted-foreground opacity-0 transition-opacity hover:bg-red-600/10 hover:text-red-600 focus-visible:opacity-100 group-hover:opacity-100"
              :aria-label="`Hapus ${e.url}`" @click="removeUrl(e)"
            >
              <Trash2 class="h-3.5 w-3.5" />
            </button>
          </li>
        </ul>
      </div>
    </section>

    <!-- Catatan prinsip -->
    <p class="mt-4 flex items-start gap-2 rounded-lg border border-border bg-card px-3 py-2.5 text-xs text-muted-foreground">
      <Globe class="mt-0.5 h-4 w-4 shrink-0 text-red-600 dark:text-red-400" />
      Crawler hanya mengambil halaman HTML (maks 60 halaman/domain, jeda 1 detik, hormat robots.txt)
      dan menyimpannya sebagai satu file JSONL di server — hemat inode, siap di-ingest Tantivy.
    </p>
  </div>
</template>
