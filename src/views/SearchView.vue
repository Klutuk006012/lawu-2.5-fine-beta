<script setup lang="ts">
import { ref, onMounted } from 'vue'
import {
  Search,
  Globe,
  Wifi,
  WifiOff,
  Loader2,
  FileText,
  FileType2,
  FolderOpen,
} from 'lucide-vue-next'
import { searchWikipedia, fetchWikiStats, isBackendReachable } from '../lib/lawu-backend'
import type { WikiResult, WikiStatsResponse } from '../lib/lawu-backend'

interface Ebook {
  id: string
  title: string
  author: string
  category: string
  fileType: string
  path: string
  sizeMb: number
  pages?: number | null
  summary: string
  indexedAt: string
}

interface SearchResponse {
  results: Ebook[]
  total: number
  categories: string[]
}

interface StatsResponse {
  docs: {
    total: number
    sourceSizeMb: number
    indexSizeMb: number
    lastIndexedAt: string | null
  }
}

const query = ref('')
const fileType = ref('all')
const category = ref('all')
const loading = ref(true)
const ebookResults = ref<Ebook[]>([])
const ebookTotal = ref(0)
const categories = ref<string[]>([])
const wikiResults = ref<WikiResult[]>([])
const wikiLatencyMs = ref(0)
const backendOnline = ref(false)
const wikiStats = ref<WikiStatsResponse | null>(null)
const stats = ref<StatsResponse | null>(null)

function typeIcon(type: string) {
  if (type === 'PDF') return FileText
  if (type === 'EPUB') return FileType2
  return FolderOpen
}

async function runSearch() {
  loading.value = true
  try {
    const body = {
      query: query.value,
      fileType: fileType.value === 'all' ? undefined : fileType.value,
      category: category.value === 'all' ? undefined : category.value,
    }
    const res = await fetch('/api/search', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    })
    const data = (await res.json()) as SearchResponse & {
      wiki?: { results: WikiResult[]; latencyMs: number; backendOnline: boolean }
    }
    ebookResults.value = data.results ?? []
    ebookTotal.value = data.total ?? 0
    categories.value = data.categories ?? []
    if (data.wiki) {
      wikiResults.value = data.wiki.results
      wikiLatencyMs.value = data.wiki.latencyMs
      backendOnline.value = data.wiki.backendOnline
    } else {
      wikiResults.value = []
      backendOnline.value = await isBackendReachable()
    }
  } catch {
    ebookResults.value = []
    ebookTotal.value = 0
    wikiResults.value = []
  } finally {
    loading.value = false
  }
}

onMounted(async () => {
  await runSearch()
  try {
    const res = await fetch('/api/stats')
    stats.value = (await res.json()) as StatsResponse
  } catch {
    // ignore
  }
  wikiStats.value = await fetchWikiStats()
})

function formatDate(iso: string | null) {
  if (!iso) return '—'
  return new Date(iso).toLocaleString('id-ID', {
    day: '2-digit',
    month: 'short',
    hour: '2-digit',
    minute: '2-digit',
  })
}
</script>

<template>
  <div class="mx-auto w-full max-w-4xl px-4 py-6">
    <!-- Ringkasan index -->
    <div class="grid grid-cols-2 gap-3 lg:grid-cols-4">
      <div class="rounded-xl border border-border bg-card px-4 py-3 shadow-sm">
        <p class="text-[11px] uppercase tracking-wide text-muted-foreground">Korpus Wikipedia-ID</p>
        <p class="mt-1 text-lg font-semibold tabular-nums" :class="{ 'text-primary': backendOnline }">
          {{ wikiStats ? wikiStats.total_docs.toLocaleString('id-ID') : '—' }}
        </p>
      </div>
      <div class="rounded-xl border border-border bg-card px-4 py-3 shadow-sm">
        <p class="text-[11px] uppercase tracking-wide text-muted-foreground">Ebook Terindeks</p>
        <p class="mt-1 text-lg font-semibold tabular-nums">{{ stats?.docs.total ?? '—' }}</p>
      </div>
      <div class="rounded-xl border border-border bg-card px-4 py-3 shadow-sm">
        <p class="text-[11px] uppercase tracking-wide text-muted-foreground">Ukuran Sumber</p>
        <p class="mt-1 text-lg font-semibold tabular-nums">{{ stats?.docs.sourceSizeMb ?? '—' }} MB</p>
      </div>
      <div class="rounded-xl border border-border bg-card px-4 py-3 shadow-sm">
        <p class="text-[11px] uppercase tracking-wide text-muted-foreground">Terakhir Diindeks</p>
        <p class="mt-1 text-lg font-semibold tabular-nums">{{ formatDate(stats?.docs.lastIndexedAt ?? null) }}</p>
      </div>
    </div>

    <!-- Status backend -->
    <div class="mt-3 flex items-center justify-between rounded-xl border border-border bg-card px-4 py-3 shadow-sm">
      <div class="flex items-center gap-2">
        <Wifi v-if="backendOnline" class="h-4 w-4 text-[#34a853]" />
        <WifiOff v-else class="h-4 w-4 text-muted-foreground" />
        <div>
          <p class="text-sm font-medium text-foreground">
            Backend Rust {{ backendOnline ? 'terhubung' : 'offline' }}
          </p>
          <p class="text-[11px] text-muted-foreground">
            <span v-if="backendOnline">Server Axum port 3001 aktif · {{ wikiStats?.segments ?? 0 }} segment Tantivy</span>
            <span v-else>Pencarian Wikipedia-ID tidak tersedia. Menampilkan ebook lokal saja.</span>
          </p>
        </div>
      </div>
      <span v-if="wikiLatencyMs > 0" class="rounded-md bg-secondary px-2 py-1 font-mono text-[10px]">
        {{ wikiLatencyMs }} ms
      </span>
    </div>

    <!-- Bar pencarian -->
    <form class="mt-5 flex flex-col gap-2 sm:flex-row" @submit.prevent="runSearch">
      <div class="relative flex-1">
        <Search class="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
        <input
          v-model="query"
          type="text"
          placeholder="Cari di Wikipedia Indonesia & koleksi ebook…"
          aria-label="Kata kunci pencarian"
          class="h-11 w-full rounded-lg border border-border bg-card pl-9 pr-3 text-sm shadow-sm focus:border-primary focus:outline-none"
        />
      </div>
      <button
        type="submit"
        :disabled="loading"
        class="flex h-11 items-center justify-center gap-2 rounded-lg bg-primary px-5 text-sm font-medium text-primary-foreground shadow-sm hover:bg-primary/90 disabled:opacity-50"
      >
        <Loader2 v-if="loading" class="h-4 w-4 animate-spin" />
        Cari
      </button>
    </form>

    <p class="mt-3 text-xs text-muted-foreground">
      <span v-if="!loading">{{ wikiResults.length + ebookTotal }} dokumen cocok · Wikipedia: {{ wikiResults.length }} · Ebook: {{ ebookTotal }}</span>
      <span v-else>Memuat…</span>
    </p>

    <!-- Hasil Wikipedia -->
    <section v-if="wikiResults.length > 0" class="mt-4">
      <div class="mb-2 flex items-center gap-2">
        <Globe class="h-4 w-4 text-primary" />
        <h2 class="text-sm font-semibold text-foreground">Wikipedia Indonesia</h2>
        <span class="rounded-md bg-secondary px-2 py-0.5 text-[10px]">
          {{ wikiResults.length }} hasil · {{ wikiLatencyMs }} ms
        </span>
      </div>
      <div class="flex flex-col gap-3">
        <article
          v-for="r in wikiResults"
          :key="r.docid"
          class="rounded-xl border border-border bg-card p-4 shadow-sm transition-all hover:-translate-y-0.5 hover:shadow-md"
        >
          <div class="flex flex-wrap items-center gap-2">
            <span class="inline-flex items-center gap-1 rounded-md bg-[#e8f0fe] px-2 py-0.5 text-xs font-medium text-[#1a73e8]">
              <Globe class="h-3 w-3" />
              Wikipedia ID
            </span>
            <span class="rounded-md bg-secondary px-2 py-0.5 font-mono text-[10px]">
              score {{ r.score.toFixed(2) }}
            </span>
            <span class="text-xs text-muted-foreground">docid: {{ r.docid }}</span>
          </div>
          <h3 class="mt-2 text-[15px] font-semibold leading-snug text-foreground">{{ r.title }}</h3>
          <p class="mt-2 text-sm leading-relaxed text-foreground/85">{{ r.preview }}</p>
        </article>
      </div>
    </section>

    <!-- Hasil Ebook -->
    <section v-if="ebookResults.length > 0" class="mt-6">
      <div class="mb-2 flex items-center gap-2">
        <FolderOpen class="h-4 w-4 text-primary" />
        <h2 class="text-sm font-semibold text-foreground">Koleksi Ebook</h2>
        <span class="rounded-md bg-secondary px-2 py-0.5 text-[10px]">{{ ebookTotal }} hasil</span>
      </div>
      <div class="flex flex-col gap-3">
        <article
          v-for="book in ebookResults"
          :key="book.id"
          class="rounded-xl border border-border bg-card p-4 shadow-sm transition-all hover:-translate-y-0.5 hover:shadow-md"
        >
          <div class="flex flex-wrap items-center gap-2">
            <span class="inline-flex items-center gap-1 rounded-md bg-secondary px-2 py-0.5 text-xs font-medium">
              <component :is="typeIcon(book.fileType)" class="h-3 w-3" />
              {{ book.fileType }}
            </span>
            <span class="rounded-md bg-secondary px-2 py-0.5 text-xs">{{ book.category }}</span>
            <span class="text-xs text-muted-foreground">{{ book.sizeMb }} MB<span v-if="book.pages"> · {{ book.pages }} hlm.</span></span>
          </div>
          <h3 class="mt-2 text-[15px] font-semibold leading-snug text-foreground">{{ book.title }}</h3>
          <p class="mt-0.5 text-xs text-muted-foreground">{{ book.author }}</p>
          <p class="mt-2 text-sm leading-relaxed text-foreground/85">{{ book.summary }}</p>
          <p class="mt-2 truncate font-mono text-[11px] text-muted-foreground" :title="book.path">{{ book.path }}</p>
        </article>
      </div>
    </section>

    <!-- Empty state -->
    <div
      v-if="!loading && wikiResults.length === 0 && ebookResults.length === 0"
      class="mt-4 rounded-xl border border-dashed border-border bg-card p-10 text-center"
    >
      <Search class="mx-auto h-8 w-8 text-muted-foreground" />
      <h3 class="mt-3 font-semibold text-foreground">Tidak ada hasil</h3>
      <p class="mt-1 text-sm text-muted-foreground">Coba kata kunci lain, atau longgarkan filter format/kategori.</p>
    </div>

    <!-- Loading skeleton -->
    <div v-if="loading && ebookResults.length === 0 && wikiResults.length === 0" class="mt-4 flex flex-col gap-3">
      <div v-for="i in 4" :key="i" class="rounded-xl border border-border bg-card p-4">
        <div class="h-5 w-3/4 animate-pulse rounded bg-muted"></div>
        <div class="mt-2 h-4 w-1/3 animate-pulse rounded bg-muted"></div>
        <div class="mt-3 h-4 w-full animate-pulse rounded bg-muted"></div>
      </div>
    </div>
  </div>
</template>
