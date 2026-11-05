<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import {
  BookOpen, LoaderCircle, RefreshCw, Database, FileText, FileType,
  FileCode, AlertTriangle, CheckCircle2,
  UploadCloud, HardDrive, Search, X,
} from 'lucide-vue-next'
import { api } from '@/lib/api'

interface EbookItem {
  name: string
  sizeBytes: number
  modifiedAt: string
  kind: string
  indexed: boolean
}

interface EbookSearchHit {
  docid: string
  title: string
  snippet: string
  kind: 'pdf' | 'epub' | 'docx' | 'txt' | string
  sourceFile: string
  sizeBytes: number
  modifiedAt: string
  score: number
}

interface SearchResponse {
  results: EbookSearchHit[]
  total: number
}

interface EbookDetail {
  ebook: {
    docid: string
    title: string
    text: string
    kind: string
    sourceFile: string
    sizeBytes: number
    modifiedAt: string
  }
}

const ebooks = ref<EbookItem[]>([])
const loading = ref(true)
const errorMsg = ref('')
const okMsg = ref('')
const uploading = ref(false)
const extracting = ref(false)
const ingesting = ref(false)
const dragOver = ref(false)
const fileInput = ref<HTMLInputElement | null>(null)

// Search state
const searchQuery = ref('')
const searchResults = ref<EbookSearchHit[]>([])
const searchTotal = ref(0)
const searching = ref(false)
const searchError = ref<string | null>(null)
const activeHit = ref<EbookSearchHit | null>(null)
const detailLoading = ref(false)
const detailText = ref<string | null>(null)
const detailModalOpen = ref(false)
let debounceTimer: ReturnType<typeof setTimeout> | null = null

const ACCEPT = '.pdf,.epub,.docx,.txt'

function humanSize(b: number): string {
  if (b < 1024) return `${b} B`
  if (b < 1024 * 1024) return `${(b / 1024).toFixed(1)} KB`
  if (b < 1024 * 1024 * 1024) return `${(b / 1024 / 1024).toFixed(1)} MB`
  return `${(b / 1024 / 1024 / 1024).toFixed(2)} GB`
}

const indexedCount = computed(() => ebooks.value.filter((e) => e.indexed).length)
const totalSize = computed(() => ebooks.value.reduce((a, e) => a + (e.sizeBytes || 0), 0))

async function load() {
  loading.value = true
  try {
    const res = await fetch('/api/ebooks')
    if (res.ok) {
      const data = await res.json()
      ebooks.value = Array.isArray(data.ebooks) ? data.ebooks : []
    } else {
      errorMsg.value = `API ebook tidak tersedia (HTTP ${res.status}). Pastikan lawu-corpus.service aktif.`
    }
  } catch {
    errorMsg.value = 'Tidak bisa menghubungi /api/ebooks (lawu-corpus :3002).'
  } finally {
    loading.value = false
  }
}

function flash(msg: string, ok = true) {
  if (ok) { okMsg.value = msg; errorMsg.value = '' } else { errorMsg.value = msg; okMsg.value = '' }
  window.setTimeout(() => { okMsg.value = ''; errorMsg.value = '' }, 6000)
}

async function uploadFiles(files: FileList | File[] | null) {
  if (!files || (files as FileList).length === 0) return
  const list = Array.from(files as FileList).filter((f) => /\.(pdf|epub|docx|txt)$/i.test(f.name))
  if (list.length === 0) {
    flash('Hanya file .pdf, .epub, .docx, atau .txt yang didukung.', false)
    return
  }
  uploading.value = true
  try {
    const fd = new FormData()
    for (const f of list) fd.append('file', f)
    const res = await fetch('/api/ebooks/upload', { method: 'POST', body: fd })
    const data = await res.json().catch(() => ({}))
    if (res.status === 201) {
      const n = Array.isArray(data.files) ? data.files.length : list.length
      flash(`${n} ebook terunggah ke server. Klik "Extract Semua" lalu "Ingest ke Index".`)
      await load()
    } else {
      flash(data.error ?? `Upload gagal (HTTP ${res.status}).`, false)
    }
  } catch {
    flash('Gagal mengunggah — koneksi ke API ebook terputus.', false)
  } finally {
    uploading.value = false
    if (fileInput.value) fileInput.value.value = ''
  }
}

function onPick(ev: Event) {
  const el = ev.target as HTMLInputElement
  uploadFiles(el.files)
}

function onDrop(ev: DragEvent) {
  dragOver.value = false
  uploadFiles(ev.dataTransfer?.files ?? null)
}

async function startExtract() {
  extracting.value = true
  try {
    const res = await fetch('/api/ebooks/extract', { method: 'POST' })
    const data = await res.json().catch(() => ({}))
    if (res.status === 202) {
      flash('Ekstraksi berjalan di server. Ebook besar bisa memakan waktu beberapa menit.')
      await load()
    } else {
      flash(data.error ?? 'Extract tidak bisa dimulai.', false)
    }
  } catch { flash('Gagal menghubungi API ebook.', false) } finally { extracting.value = false }
}

async function startIngest() {
  ingesting.value = true
  try {
    const res = await fetch('/api/corpus/ingest', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ kind: 'ebooks' }),
    })
    const data = await res.json().catch(() => ({}))
    if (res.status === 202) {
      flash('Ingest ebook berjalan. Setelah selesai, restart lawu-server bila hasil belum muncul di Pencarian.')
    } else {
      flash(data.error ?? 'Ingest tidak bisa dimulai.', false)
    }
  } catch { flash('Gagal menghubungi API korpus.', false) } finally { ingesting.value = false }
}

function kindIcon(kind: string) {
  if (kind === 'pdf') return FileText
  if (kind === 'epub') return FileType
  if (kind === 'docx') return FileCode
  return FileType
}

function fmtTime(t: string): string {
  try { return new Date(t).toLocaleString('id-ID', { dateStyle: 'medium', timeStyle: 'short' }) }
  catch { return t }
}

function highlight(text: string, q: string): string {
  if (!q) return escapeHtml(text)
  const safe = escapeHtml(text)
  const terms = q.trim().split(/\s+/).filter(Boolean).map(escapeRegExp)
  if (terms.length === 0) return safe
  const re = new RegExp(`(${terms.join('|')})`, 'gi')
  return safe.replace(re, '<mark class="rounded bg-secondary px-0.5 text-foreground">$1</mark>')
}

function escapeHtml(s: string): string {
  return s.replace(/[&<>"']/g, (c) => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;',
  } as Record<string, string>)[c] ?? c)
}

function escapeRegExp(s: string): string {
  return s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
}

async function runSearch() {
  const q = searchQuery.value.trim()
  if (!q) {
    searchResults.value = []
    searchTotal.value = 0
    return
  }
  searching.value = true
  searchError.value = null
  try {
    const res = await api.get<SearchResponse>('/api/ebooks/search', {
      params: { q, limit: 20 },
    })
    searchResults.value = res.data.results ?? []
    searchTotal.value = res.data.total ?? 0
  } catch (err: unknown) {
    const e = err as { response?: { data?: { error?: string } } }
    searchError.value = e?.response?.data?.error ?? 'Gagal melakukan pencarian ebook.'
    searchResults.value = []
    searchTotal.value = 0
  } finally {
    searching.value = false
  }
}

function onSearchInput() {
  if (debounceTimer) clearTimeout(debounceTimer)
  debounceTimer = setTimeout(runSearch, 300)
}

async function openDetail(hit: EbookSearchHit) {
  activeHit.value = hit
  detailModalOpen.value = true
  detailLoading.value = true
  detailText.value = null
  try {
    const res = await api.get<EbookDetail>(`/api/ebooks/${encodeURIComponent(hit.docid)}`)
    detailText.value = res.data.ebook?.text ?? ''
  } catch (err: unknown) {
    const e = err as { response?: { data?: { error?: string } } }
    detailText.value = e?.response?.data?.error ?? 'Gagal memuat detail ebook.'
  } finally {
    detailLoading.value = false
  }
}

function closeDetail() {
  detailModalOpen.value = false
  activeHit.value = null
  detailText.value = null
}

onMounted(() => {
  load()
})

onUnmounted(() => {
  if (debounceTimer) clearTimeout(debounceTimer)
})
</script>

<template>
  <div class="mx-auto w-full max-w-5xl px-4 py-6">
    <!-- 1. Header halaman -->
    <div class="mb-5">
      <div class="flex items-center gap-2 text-[11px] font-semibold uppercase tracking-widest text-accent">
        <span class="inline-block h-2 w-2 rounded-sm bg-accent" aria-hidden />
        Perpustakaan Digital
      </div>
      <h2 class="mt-1.5 text-2xl font-extrabold tracking-tight text-foreground sm:text-3xl">
        Perpustakaan Ebook Lawu Fine
      </h2>
      <p class="mt-1 text-sm text-muted-foreground">
        {{ ebooks.length }} ebook terindex di korpus pencarian — PDF, EPUB, DOCX, TXT.
        Cari, unggah, ekstrak, dan ingest ke index Lawu Fine.
      </p>
    </div>

    <!-- Notifikasi -->
    <p v-if="okMsg" class="mb-4 flex items-start gap-2 rounded-lg border border-primary/30 bg-primary/10 px-3 py-2 text-sm text-primary">
      <CheckCircle2 class="mt-0.5 h-4 w-4 shrink-0" /> {{ okMsg }}
    </p>
    <p v-if="errorMsg" class="mb-4 flex items-start gap-2 rounded-lg border border-accent/30 bg-accent/10 px-3 py-2 text-sm text-accent">
      <AlertTriangle class="mt-0.5 h-4 w-4 shrink-0" /> {{ errorMsg }}
    </p>

    <!-- 2. Search box besar -->
    <section aria-labelledby="search-ebook-title" class="mt-2 rounded-2xl border border-border bg-card p-5 shadow-lawu-sm sm:p-6">
      <h3 id="search-ebook-title" class="flex items-center gap-2 text-base font-bold text-foreground">
        <Search class="h-4 w-4 text-primary" /> Cari Ebook
      </h3>
      <p class="mt-0.5 text-xs text-muted-foreground">
        Pencarian full-text atas index Tantivy. Ketik untuk auto-search (debounce 300ms).
      </p>
      <div class="mt-4 flex flex-col gap-2 sm:flex-row">
        <div class="relative flex-1">
          <Search class="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
          <input
            v-model="searchQuery"
            @input="onSearchInput"
            @keydown.enter="runSearch"
            type="text"
            placeholder="Cari judul, konten, atau penulis ebook…"
            aria-label="Kata kunci pencarian ebook"
            class="h-12 w-full rounded-xl border border-border bg-background pl-10 pr-3 text-sm shadow-sm focus:border-primary focus:outline-none focus:ring-2 focus:ring-primary/20"
          />
        </div>
        <button
          type="button"
          @click="runSearch"
          :disabled="searching"
          class="flex h-12 items-center justify-center gap-2 rounded-xl bg-primary px-6 text-sm font-semibold text-primary-foreground shadow-lawu-sm hover:bg-primary/90 disabled:opacity-50"
        >
          <LoaderCircle v-if="searching" class="h-4 w-4 animate-spin" />
          Cari
        </button>
      </div>

      <p v-if="searchError" class="mt-3 rounded-lg border border-destructive/40 bg-destructive/10 px-3 py-2 text-xs text-destructive">
        {{ searchError }}
      </p>

      <!-- 3. Hasil Search -->
      <div v-if="searchQuery.trim()" class="mt-4">
        <p class="text-xs text-muted-foreground">
          <span v-if="!searching" class="font-medium text-foreground">{{ searchTotal.toLocaleString('id-ID') }}</span>
          dokumen cocok
          <span v-if="searching">· Mencari…</span>
        </p>
        <ul class="mt-2 divide-y divide-border overflow-hidden rounded-xl border border-border bg-background">
          <li v-if="!searching && searchResults.length === 0" class="px-4 py-8 text-center text-xs text-muted-foreground">
            Tidak ada hasil. Coba kata kunci lain atau pastikan sudah menjalankan Extract + Ingest.
          </li>
          <li
            v-for="hit in searchResults"
            :key="hit.docid"
            class="cursor-pointer px-4 py-3 transition-colors hover:bg-secondary/50"
            @click="openDetail(hit)"
          >
            <div class="flex flex-wrap items-center gap-2">
              <span class="inline-flex items-center gap-1 rounded-md bg-secondary px-2 py-0.5 text-[10.5px] font-bold uppercase text-secondary-foreground">
                <component :is="kindIcon(hit.kind)" class="h-3 w-3" /> {{ hit.kind }}
              </span>
              <span class="rounded-md bg-muted px-2 py-0.5 font-mono text-[10px] text-muted-foreground">score {{ hit.score.toFixed(2) }}</span>
              <span class="text-[11px] text-muted-foreground">{{ humanSize(hit.sizeBytes) }} · {{ fmtTime(hit.modifiedAt) }}</span>
              <!-- Score relevance bar -->
              <div class="ml-auto flex items-center gap-1.5">
                <div class="h-1.5 w-16 overflow-hidden rounded-full bg-muted">
                  <div class="h-full bg-primary" :style="{ width: `${Math.min(100, hit.score * 100)}%` }" />
                </div>
              </div>
            </div>
            <p class="mt-1.5 text-sm font-semibold text-foreground" v-html="highlight(hit.title, searchQuery)" />
            <p class="mt-1 text-xs leading-relaxed text-muted-foreground line-clamp-2" v-html="highlight(hit.snippet, searchQuery)" />
            <p class="mt-1 truncate font-mono text-[10.5px] text-muted-foreground" :title="hit.sourceFile">{{ hit.sourceFile }}</p>
          </li>
        </ul>
      </div>
    </section>

    <!-- 4. Add Ebook (upload) -->
    <section aria-labelledby="add-ebook-title" class="mt-5 rounded-2xl border border-border bg-card p-5 shadow-lawu-sm sm:p-6">
      <h3 id="add-ebook-title" class="text-base font-bold text-foreground">Tambah Ebook Baru</h3>
      <p class="mt-0.5 text-xs text-muted-foreground">Seret file ke area di bawah atau klik untuk memilih — bisa banyak file sekaligus.</p>

      <div
        class="mt-3 flex flex-col items-center justify-center gap-2 rounded-xl border-2 border-dashed px-4 py-8 text-center transition-colors"
        :class="dragOver ? 'border-accent bg-accent/5' : 'border-border bg-background hover:bg-secondary/50'"
        role="button" tabindex="0"
        aria-label="Area unggah ebook. Klik atau seret file PDF, EPUB, DOCX, atau TXT."
        @click="fileInput?.click()"
        @keydown.enter.prevent="fileInput?.click()"
        @dragover.prevent="dragOver = true"
        @dragleave.prevent="dragOver = false"
        @drop.prevent="onDrop"
      >
        <LoaderCircle v-if="uploading" class="h-8 w-8 animate-spin text-accent" />
        <UploadCloud v-else class="h-8 w-8 text-muted-foreground" />
        <p class="text-sm font-medium text-foreground">
          {{ uploading ? 'Mengunggah ke server…' : 'Seret &amp; letakkan file di sini, atau klik untuk memilih' }}
        </p>
        <p class="text-xs text-muted-foreground">PDF &middot; EPUB &middot; DOCX &middot; TXT — maksimal 500 MB per file</p>
        <input ref="fileInput" type="file" :accept="ACCEPT" multiple class="hidden" @change="onPick" />
      </div>

      <div class="mt-4 flex flex-wrap items-center gap-2">
        <button
          type="button" :disabled="extracting || ebooks.length === 0"
          class="inline-flex h-9 items-center gap-1.5 rounded-lg bg-accent px-3.5 text-sm font-semibold text-accent-foreground shadow-lawu-sm transition-colors hover:bg-accent/90 disabled:cursor-not-allowed disabled:opacity-50"
          @click="startExtract"
        >
          <LoaderCircle v-if="extracting" class="h-4 w-4 animate-spin" />
          <BookOpen v-else class="h-4 w-4" /> Extract Semua Ebook
        </button>
        <button
          type="button" :disabled="ingesting"
          class="inline-flex h-9 items-center gap-1.5 rounded-lg bg-primary px-3.5 text-sm font-semibold text-primary-foreground shadow-lawu-sm transition-colors hover:bg-primary/90 disabled:opacity-50"
          @click="startIngest"
        >
          <LoaderCircle v-if="ingesting" class="h-4 w-4 animate-spin" />
          <Database v-else class="h-4 w-4" /> Ingest ke Index
        </button>
        <button
          type="button" class="inline-flex h-9 items-center gap-1.5 rounded-lg border border-border bg-secondary px-3 text-sm font-medium text-foreground transition-colors hover:bg-secondary/70"
          aria-label="Muat ulang daftar ebook" @click="load"
        >
          <RefreshCw class="h-4 w-4" :class="loading ? 'animate-spin' : ''" /> Muat Ulang
        </button>
      </div>
    </section>

    <!-- 5. Stat cards + Library list (default) -->
    <section class="mt-5 grid gap-3 sm:grid-cols-3">
      <div class="rounded-xl border border-border bg-card p-4 shadow-lawu-sm">
        <p class="flex items-center gap-1.5 text-xs font-medium text-muted-foreground"><BookOpen class="h-3.5 w-3.5" /> Total Ebook</p>
        <p class="mt-1 text-2xl font-extrabold tracking-tight text-foreground">{{ ebooks.length }}</p>
      </div>
      <div class="rounded-xl border border-border bg-card p-4 shadow-lawu-sm">
        <p class="flex items-center gap-1.5 text-xs font-medium text-muted-foreground"><HardDrive class="h-3.5 w-3.5" /> Ukuran Perpustakaan</p>
        <p class="mt-1 text-2xl font-extrabold tracking-tight text-foreground">{{ humanSize(totalSize) }}</p>
      </div>
      <div class="rounded-xl border border-border bg-card p-4 shadow-lawu-sm">
        <p class="flex items-center gap-1.5 text-xs font-medium text-muted-foreground"><Database class="h-3.5 w-3.5" /> Sudah Terindex</p>
        <p class="mt-1 text-2xl font-extrabold tracking-tight text-foreground">{{ indexedCount }}<span class="text-base font-medium text-muted-foreground"> / {{ ebooks.length }}</span></p>
      </div>
    </section>

    <!-- Library list -->
    <section aria-labelledby="daftar-ebook-title" class="mt-4 rounded-2xl border border-border bg-card p-5 shadow-lawu-sm sm:p-6">
      <h3 id="daftar-ebook-title" class="text-base font-bold text-foreground">Perpustakaan di Server</h3>
      <p class="mt-0.5 text-xs text-muted-foreground">Lokasi: <code class="rounded bg-muted px-1 py-0.5 font-mono text-[10.5px]">/mnt/data/ebooks</code></p>

      <div class="lawu-scroll mt-3 max-h-96 overflow-y-auto pr-1">
        <p v-if="loading" class="flex items-center gap-2 py-6 text-sm text-muted-foreground">
          <LoaderCircle class="h-4 w-4 animate-spin" /> Memuat perpustakaan&hellip;
        </p>
        <p v-else-if="ebooks.length === 0" class="py-6 text-sm text-muted-foreground">
          Belum ada ebook di server — unggah lewat area di atas, atau salin langsung via
          <code class="rounded bg-muted px-1 py-0.5 font-mono text-[10.5px]">scp</code> ke
          <code class="rounded bg-muted px-1 py-0.5 font-mono text-[10.5px]">/mnt/data/ebooks/</code>.
        </p>
        <ul v-else class="divide-y divide-border">
          <li v-for="b in ebooks" :key="b.name" class="flex items-center gap-3 py-2.5">
            <span class="flex h-9 w-9 shrink-0 items-center justify-center rounded-lg bg-secondary text-foreground">
              <component :is="kindIcon(b.kind)" class="h-4 w-4" />
            </span>
            <div class="min-w-0 flex-1">
              <p class="truncate text-sm font-medium text-foreground" :title="b.name">{{ b.name }}</p>
              <p class="text-[11px] text-muted-foreground">
                {{ humanSize(b.sizeBytes) }} &middot; {{ b.kind.toUpperCase() }} &middot; {{ fmtTime(b.modifiedAt) }}
              </p>
            </div>
            <span
              class="shrink-0 rounded-md border px-1.5 py-0.5 text-[10px] font-bold uppercase tracking-wide"
              :class="b.indexed
                ? 'border-primary/20 bg-primary/10 text-primary'
                : 'border-border bg-secondary text-muted-foreground'"
            >{{ b.indexed ? 'Indexed' : 'Pending' }}</span>
          </li>
        </ul>
      </div>
    </section>

    <!-- Catatan alur -->
    <p class="mt-4 flex items-start gap-2 rounded-lg border border-border bg-card px-3 py-2.5 text-xs text-muted-foreground">
      <UploadCloud class="mt-0.5 h-4 w-4 shrink-0 text-accent" />
      Alur lengkap: <span class="font-medium text-foreground">Unggah &rarr; Extract &rarr; Ingest</span>.
      Extract mengubah ebook menjadi satu file JSONL (hemat inode); Ingest menambahkannya ke index
      Tantavy dengan dedup otomatis per dokumen.
    </p>

    <!-- Detail modal -->
    <div
      v-if="detailModalOpen"
      class="fixed inset-0 z-50 flex items-end justify-center bg-black/50 p-0 backdrop-blur-sm sm:items-center sm:p-4"
      role="dialog"
      aria-modal="true"
      @click.self="closeDetail"
    >
      <div class="flex max-h-[90vh] w-full max-w-3xl flex-col rounded-t-2xl border border-border bg-card shadow-2xl sm:rounded-2xl">
        <header class="flex items-start justify-between gap-3 border-b border-border p-4">
          <div class="min-w-0">
            <p v-if="activeHit" class="text-[10.5px] uppercase tracking-wide text-muted-foreground">{{ activeHit.kind }} · {{ humanSize(activeHit.sizeBytes) }}</p>
            <h3 class="mt-0.5 truncate text-sm font-bold text-foreground" :title="activeHit?.title">{{ activeHit?.title }}</h3>
            <p v-if="activeHit" class="mt-0.5 truncate font-mono text-[10.5px] text-muted-foreground" :title="activeHit.sourceFile">{{ activeHit.sourceFile }}</p>
          </div>
          <button
            type="button"
            @click="closeDetail"
            class="rounded-md p-1.5 text-muted-foreground hover:bg-secondary hover:text-foreground"
            aria-label="Tutup detail"
          >
            <X class="h-4 w-4" />
          </button>
        </header>
        <div class="lawu-scroll min-h-0 flex-1 overflow-y-auto p-4">
          <div v-if="detailLoading" class="flex items-center gap-2 py-6 text-sm text-muted-foreground">
            <LoaderCircle class="h-4 w-4 animate-spin" /> Memuat teks lengkap…
          </div>
          <pre v-else class="whitespace-pre-wrap break-words font-sans text-[12.5px] leading-relaxed text-foreground">{{ detailText ?? '' }}</pre>
        </div>
      </div>
    </div>
  </div>
</template>
