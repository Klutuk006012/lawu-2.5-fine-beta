<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import {
  BookOpen, Loader2, RefreshCw, Database, FileText, FileType2,
  FileCode2, FileType, AlertTriangle, CheckCircle2, UploadCloud, HardDrive,
} from 'lucide-vue-next'

interface EbookItem {
  name: string
  sizeBytes: number
  modifiedAt: string
  kind: string
  indexed: boolean
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
  const list = Array.from(files as FileList).filter((f) =>
    /\.(pdf|epub|docx|txt)$/i.test(f.name)
  )
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
  if (kind === 'epub') return FileType2
  if (kind === 'docx') return FileCode2
  return FileType
}

function fmtTime(t: string): string {
  try { return new Date(t).toLocaleString('id-ID', { dateStyle: 'medium', timeStyle: 'short' }) }
  catch { return t }
}

onMounted(load)
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
        Ebook &amp; Dokumen Lawu Fine
      </h2>
      <p class="mt-1 text-sm text-muted-foreground">
        Tambahkan ebook (PDF / EPUB / DOCX / TXT) ke perpustakaan server, lalu ekstrak dan
        masukkan ke index pencarian agar Lawu Fine makin pintar.
      </p>
    </div>

    <!-- Notifikasi -->
    <p v-if="okMsg" class="mb-4 flex items-start gap-2 rounded-lg border border-emerald-500/30 bg-emerald-500/10 px-3 py-2 text-sm text-emerald-700 dark:text-emerald-400">
      <CheckCircle2 class="mt-0.5 h-4 w-4 shrink-0" /> {{ okMsg }}
    </p>
    <p v-if="errorMsg" class="mb-4 flex items-start gap-2 rounded-lg border border-red-600/30 bg-red-600/10 px-3 py-2 text-sm text-red-700 dark:text-red-400">
      <AlertTriangle class="mt-0.5 h-4 w-4 shrink-0" /> {{ errorMsg }}
    </p>

    <!-- Add Ebook (upload) -->
    <section aria-labelledby="add-ebook-title" class="rounded-xl border border-border bg-card p-4 shadow-sm sm:p-6">
      <h3 id="add-ebook-title" class="text-sm font-semibold text-foreground">Tambah Ebook</h3>
      <p class="mt-0.5 text-xs text-muted-foreground">Seret file ke area di bawah atau klik untuk memilih — bisa banyak file sekaligus.</p>

      <div
        class="mt-3 flex flex-col items-center justify-center gap-2 rounded-xl border-2 border-dashed px-4 py-8 text-center transition-colors"
        :class="dragOver ? 'border-red-600 bg-red-600/5' : 'border-border bg-background hover:bg-secondary/50'"
        role="button" tabindex="0"
        aria-label="Area unggah ebook. Klik atau seret file PDF, EPUB, DOCX, atau TXT."
        @click="fileInput?.click()"
        @keydown.enter.prevent="fileInput?.click()"
        @dragover.prevent="dragOver = true"
        @dragleave.prevent="dragOver = false"
        @drop.prevent="onDrop"
      >
        <Loader2 v-if="uploading" class="h-8 w-8 animate-spin text-red-600" />
        <UploadCloud v-else class="h-8 w-8 text-muted-foreground" />
        <p class="text-sm font-medium text-foreground">
          {{ uploading ? 'Mengunggah ke server…' : 'Seret &amp; letakkan file di sini, atau klik untuk memilih' }}
        </p>
        <p class="text-xs text-muted-foreground">PDF &middot; EPUB &middot; DOCX &middot; TXT — maksimal 500 MB per file</p>
        <input ref="fileInput" type="file" :accept="ACCEPT" multiple class="hidden" @change="onPick" />
      </div>

      <!-- Aksi lanjutan -->
      <div class="mt-4 flex flex-wrap items-center gap-2">
        <button
          type="button" :disabled="extracting || ebooks.length === 0"
          class="inline-flex h-9 items-center gap-1.5 rounded-lg bg-red-600 px-3.5 text-sm font-semibold text-white shadow-sm transition-colors hover:bg-red-700 disabled:cursor-not-allowed disabled:opacity-50"
          @click="startExtract"
        >
          <Loader2 v-if="extracting" class="h-4 w-4 animate-spin" />
          <BookOpen v-else class="h-4 w-4" /> Extract Semua Ebook
        </button>
        <button
          type="button" :disabled="ingesting"
          class="inline-flex h-9 items-center gap-1.5 rounded-lg bg-primary px-3.5 text-sm font-semibold text-primary-foreground shadow-sm transition-colors hover:bg-primary/90 disabled:opacity-50"
          @click="startIngest"
        >
          <Loader2 v-if="ingesting" class="h-4 w-4 animate-spin" />
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

    <!-- Ringkasan -->
    <section class="mt-4 grid gap-2 sm:grid-cols-3">
      <div class="rounded-xl border border-border bg-card p-4 shadow-sm">
        <p class="flex items-center gap-1.5 text-xs font-medium text-muted-foreground"><BookOpen class="h-3.5 w-3.5" /> Total Ebook</p>
        <p class="mt-1 text-2xl font-bold tracking-tight text-foreground">{{ ebooks.length }}</p>
      </div>
      <div class="rounded-xl border border-border bg-card p-4 shadow-sm">
        <p class="flex items-center gap-1.5 text-xs font-medium text-muted-foreground"><HardDrive class="h-3.5 w-3.5" /> Ukuran Perpustakaan</p>
        <p class="mt-1 text-2xl font-bold tracking-tight text-foreground">{{ humanSize(totalSize) }}</p>
      </div>
      <div class="rounded-xl border border-border bg-card p-4 shadow-sm">
        <p class="flex items-center gap-1.5 text-xs font-medium text-muted-foreground"><Database class="h-3.5 w-3.5" /> Sudah Terindeks</p>
        <p class="mt-1 text-2xl font-bold tracking-tight text-foreground">{{ indexedCount }}<span class="text-base font-medium text-muted-foreground"> / {{ ebooks.length }}</span></p>
      </div>
    </section>

    <!-- Daftar ebook -->
    <section aria-labelledby="daftar-ebook-title" class="mt-4 rounded-xl border border-border bg-card p-4 shadow-sm sm:p-6">
      <h3 id="daftar-ebook-title" class="text-sm font-semibold text-foreground">Perpustakaan di Server</h3>
      <p class="mt-0.5 text-xs text-muted-foreground">Lokasi: <code class="rounded bg-secondary px-1 py-0.5">/mnt/data/ebooks</code></p>

      <div class="mt-3 max-h-96 overflow-y-auto pr-1">
        <p v-if="loading" class="flex items-center gap-2 py-6 text-sm text-muted-foreground">
          <Loader2 class="h-4 w-4 animate-spin" /> Memuat perpustakaan&hellip;
        </p>
        <p v-else-if="ebooks.length === 0" class="py-6 text-sm text-muted-foreground">
          Belum ada ebook di server — unggah lewat area di atas, atau salin langsung via
          <code class="rounded bg-secondary px-1 py-0.5">scp</code> ke
          <code class="rounded bg-secondary px-1 py-0.5">/mnt/data/ebooks/</code>.
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
              class="shrink-0 rounded-md border px-1.5 py-0.5 text-[10px] font-semibold uppercase tracking-wide"
              :class="b.indexed
                ? 'border-emerald-500/20 bg-emerald-500/10 text-emerald-700 dark:text-emerald-400'
                : 'border-border bg-secondary text-muted-foreground'"
            >{{ b.indexed ? 'terindeks' : 'menunggu' }}</span>
          </li>
        </ul>
      </div>
    </section>

    <!-- Catatan alur -->
    <p class="mt-4 flex items-start gap-2 rounded-lg border border-border bg-card px-3 py-2.5 text-xs text-muted-foreground">
      <UploadCloud class="mt-0.5 h-4 w-4 shrink-0 text-red-600 dark:text-red-400" />
      Alur lengkap: <span class="font-medium text-foreground">Unggah &rarr; Extract &rarr; Ingest</span>.
      Extract mengubah ebook menjadi satu file JSONL (hemat inode); Ingest menambahkannya ke index
      Tantivy dengan dedup otomatis per dokumen.
    </p>
  </div>
</template>
