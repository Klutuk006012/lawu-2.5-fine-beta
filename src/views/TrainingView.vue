<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import {
  GraduationCap, Database, BookOpen, Globe, FileText, Activity, Clock,
  UploadCloud, LoaderCircle, Play, Plus, Calendar, History,
  CheckCircle2, AlertTriangle, Send, Target, Mountain,
  Sparkles,
} from 'lucide-vue-next'
import { api } from '@/lib/api'

/**
 * TrainingView — "Manajemen Training Lawu Model Fine"
 * -----------------------------------------------------
 * Catatan Penting (Paten 3 — CPU-only):
 * Lawu Fine TIDAK melakukan fine-tune bobot model (butuh GPU).
 * Yang dimaksud "training" di sini:
 *   1. RAG augmentation (ingest dokumen baru ke Tantivy + Qdrant)
 *   2. Evaluation (Q&A test suite)
 *   3. Curriculum scheduling (jadwal belajar berkala)
 *
 * Backend API endpoint akan hadir di v3.7.0 — saat ini pakai MOCK data.
 * TODO: backend v3.7.0 — ganti mockFetch() ke api.get() / api.post().
 */

// ============================================================
// Types
// ============================================================

interface ModelStatus {
  model: string
  active: boolean
  totalDocs: number
  totalSessions: number
  lastTrainingAt: string | null
  evaluationScore: number
  coveragePct: number
}

interface MaterialRow {
  id: string
  source: string
  category: string
  docCount: number
  status: 'terindex' | 'pending' | 'gagal'
}

interface QueueItem {
  id: string
  filename: string
  category: string
  priority: 'tinggi' | 'sedang' | 'rendah'
  sizeBytes: number
  uploadedAt: string
}

interface ScheduleItem {
  id: string
  label: string
  cron: string
  action: string
  enabled: boolean
}

interface HistoryItem {
  id: string
  timestamp: string
  type: 'crawl' | 'extract' | 'ingest' | 'evaluasi'
  message: string
  success: boolean
  durationSec: number
  failedCount?: number
}

interface QAItem {
  id: string
  question: string
  expected: string
  actual: string | null
  score: number | null
  status: 'pending' | 'excellent' | 'good' | 'need_improvement'
}

// ============================================================
// State — TODO: backend v3.7.0 (saat ini mock)
// ============================================================

const modelStatus = ref<ModelStatus>({
  model: 'Qwen2.5-Coder-7B Instruct (Q4_K_M)',
  active: true,
  totalDocs: 1245903,
  totalSessions: 47,
  lastTrainingAt: new Date(Date.now() - 1000 * 60 * 60 * 6).toISOString(),
  evaluationScore: 85,
  coveragePct: 62,
})

const materials = ref<MaterialRow[]>([
  { id: 'mat-wikipedia', source: 'Wikipedia ID', category: 'Indonesia', docCount: 1384742, status: 'terindex' },
  { id: 'mat-ebooks', source: 'Ebook Perpustakaan', category: 'Teknologi', docCount: 30, status: 'terindex' },
  { id: 'mat-webcrawl', source: 'Web Crawl Korpus', category: 'Indonesia', docCount: 123, status: 'terindex' },
  { id: 'mat-skill', source: 'Skill Document', category: 'AI/ML', docCount: 1, status: 'terindex' },
])

const queue = ref<QueueItem[]>([
  { id: 'q1', filename: 'panduan-siber-indonesia.pdf', category: 'Siber', priority: 'tinggi', sizeBytes: 2_400_000, uploadedAt: new Date(Date.now() - 1000 * 60 * 30).toISOString() },
])

const schedules = ref<ScheduleItem[]>([
  { id: 'sch1', label: 'Setiap hari Senin 03:00 — Crawl web korpus + ingest', cron: '0 3 * * 1', action: 'crawl_web', enabled: true },
  { id: 'sch2', label: 'Setiap awal bulan — Re-extract semua ebook', cron: '0 3 1 * *', action: 'extract_ebooks', enabled: true },
  { id: 'sch3', label: 'Setiap 6 jam — Health check + auto-restart kalau perlu', cron: '0 */6 * * *', action: 'health_check', enabled: false },
])

const history = ref<HistoryItem[]>([
  { id: 'h1', timestamp: new Date(Date.now() - 1000 * 60 * 30).toISOString(), type: 'ingest', message: 'Ingest 123 dokumen web crawl', success: true, durationSec: 300 },
  { id: 'h2', timestamp: new Date(Date.now() - 1000 * 60 * 60 * 2).toISOString(), type: 'extract', message: 'Extract 30 ebook', success: true, durationSec: 480 },
  { id: 'h3', timestamp: new Date(Date.now() - 1000 * 60 * 60 * 5).toISOString(), type: 'crawl', message: 'Crawl 42 URL web korpus', success: true, durationSec: 240, failedCount: 54 },
  { id: 'h4', timestamp: new Date(Date.now() - 1000 * 60 * 60 * 6).toISOString(), type: 'ingest', message: 'Ingest 1 skill document', success: true, durationSec: 1 },
  { id: 'h5', timestamp: new Date(Date.now() - 1000 * 60 * 60 * 24).toISOString(), type: 'evaluasi', message: 'Evaluasi 5 Q&A pair — akurasi 85%', success: true, durationSec: 35 },
])

const qaPairs = ref<QAItem[]>([
  { id: 'qa1', question: 'Apa itu Lawu Fine?', expected: 'Lawu Fine adalah mesin pencari ebook lokal Indonesia', actual: 'Lawu Fine adalah mesin pencari ebook lokal Indonesia yang dibangun di atas Tantivy dan Qwen2.5', score: 0.92, status: 'excellent' },
  { id: 'qa2', question: 'Bagaimana cara kerja RAG?', expected: 'RAG menggabungkan retrieval dokumen dengan generasi LLM', actual: 'RAG adalah teknik yang menggabungkan pencarian dokumen dengan generasi teks LLM', score: 0.88, status: 'good' },
  { id: 'qa3', question: 'Apa keunggulan CPU-only?', expected: 'Hemat biaya, tidak butuh GPU, dapat berjalan di VPS murah', actual: null, score: null, status: 'pending' },
])

// ============================================================
// Form state
// ============================================================

const uploadCategory = ref<'Teknologi' | 'Keamanan' | 'AI/ML' | 'Siber' | 'Indonesia' | 'Custom'>('Teknologi')
const uploadPriority = ref<'Tinggi' | 'Sedang' | 'Rendah'>('Sedang')
const uploadDragging = ref(false)
const uploadFiles = ref<File[]>([])
const uploadInput = ref<HTMLInputElement | null>(null)
const uploading = ref(false)

const newScheduleFreq = ref<'harian' | 'mingguan' | 'bulanan' | 'custom'>('mingguan')
const newScheduleDay = ref('Senin')
const newScheduleTime = ref('03:00')
const newScheduleAction = ref<'crawl_web' | 'extract_ebooks' | 'ingest_skill' | 'custom'>('crawl_web')

const newQaQuestion = ref('')
const newQaExpected = ref('')

const historyFilter = ref<'semua' | 'crawl' | 'extract' | 'ingest' | 'evaluasi'>('semua')
const runningEval = ref(false)

// ============================================================
// Computed
// ============================================================

const filteredHistory = computed(() => {
  if (historyFilter.value === 'semua') return history.value
  return history.value.filter((h) => h.type === historyFilter.value)
})

const evalStats = computed(() => {
  const total = qaPairs.value.length
  const scored = qaPairs.value.filter((q) => q.score !== null)
  const avg = scored.length === 0 ? 0 : (scored.reduce((a, q) => a + (q.score ?? 0), 0) / scored.length) * 100
  const excellent = qaPairs.value.filter((q) => q.status === 'excellent').length
  const good = qaPairs.value.filter((q) => q.status === 'good').length
  const needImp = qaPairs.value.filter((q) => q.status === 'need_improvement').length
  return { total, avg, excellent, good, needImp }
})

const totalMaterialsDocs = computed(() => materials.value.reduce((a, m) => a + m.docCount, 0))

// ============================================================
// Helpers
// ============================================================

function fmtTime(iso: string | null): string {
  if (!iso) return '—'
  try {
    return new Date(iso).toLocaleString('id-ID', { dateStyle: 'medium', timeStyle: 'short' })
  } catch { return iso }
}

function fmtDuration(sec: number): string {
  if (sec < 60) return `${sec} dtk`
  if (sec < 3600) return `${(sec / 60).toFixed(0)} mnt`
  return `${(sec / 3600).toFixed(1)} jam`
}

function humanSize(b: number): string {
  if (b < 1024) return `${b} B`
  if (b < 1024 * 1024) return `${(b / 1024).toFixed(1)} KB`
  if (b < 1024 * 1024 * 1024) return `${(b / 1024 / 1024).toFixed(1)} MB`
  return `${(b / 1024 / 1024 / 1024).toFixed(2)} GB`
}

function historyTypeIcon(t: HistoryItem['type']) {
  if (t === 'crawl') return Globe
  if (t === 'extract') return BookOpen
  if (t === 'ingest') return Database
  return Target
}

function historyTypeLabel(t: HistoryItem['type']): string {
  const m: Record<HistoryItem['type'], string> = {
    crawl: 'Crawl',
    extract: 'Extract',
    ingest: 'Ingest',
    evaluasi: 'Evaluasi',
  }
  return m[t]
}

function qaStatusBadge(s: QAItem['status']): { label: string; class: string } {
  const map: Record<QAItem['status'], { label: string; class: string }> = {
    pending: { label: 'Pending', class: 'bg-muted text-muted-foreground' },
    excellent: { label: 'Excellent', class: 'bg-primary/10 text-primary' },
    good: { label: 'Good', class: 'bg-secondary text-secondary-foreground' },
    need_improvement: { label: 'Need Improvement', class: 'bg-accent/10 text-accent' },
  }
  return map[s]
}

// ============================================================
// Actions — TODO: backend v3.7.0 (saat ini mock)
// ============================================================

function onUploadPick(ev: Event) {
  const el = ev.target as HTMLInputElement
  if (!el.files) return
  uploadFiles.value = Array.from(el.files)
}

function onUploadDrop(ev: DragEvent) {
  uploadDragging.value = false
  if (ev.dataTransfer?.files) {
    uploadFiles.value = Array.from(ev.dataTransfer.files)
  }
}

async function submitUpload() {
  if (uploadFiles.value.length === 0) return
  uploading.value = true
  // TODO: backend v3.7.0 — POST /api/training/materials/upload (multipart)
  try {
    // mock delay
    await new Promise((r) => setTimeout(r, 600))
    for (const f of uploadFiles.value) {
      queue.value.unshift({
        id: `q-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`,
        filename: f.name,
        category: uploadCategory.value,
        priority: uploadPriority.value.toLowerCase() as QueueItem['priority'],
        sizeBytes: f.size,
        uploadedAt: new Date().toISOString(),
      })
    }
    uploadFiles.value = []
    if (uploadInput.value) uploadInput.value.value = ''
  } finally {
    uploading.value = false
  }
}

async function ingestQueueItem(item: QueueItem) {
  // TODO: backend v3.7.0 — POST /api/training/ingest/:materialId
  await new Promise((r) => setTimeout(r, 400))
  // hapus dari antrian, tambah ke history
  queue.value = queue.value.filter((q) => q.id !== item.id)
  history.value.unshift({
    id: `h-${Date.now()}`,
    timestamp: new Date().toISOString(),
    type: 'ingest',
    message: `Ingest 1 dokumen ${item.filename}`,
    success: true,
    durationSec: 2,
  })
}

function toggleSchedule(id: string) {
  // TODO: backend v3.7.0 — PATCH /api/training/schedule/:id
  const s = schedules.value.find((x) => x.id === id)
  if (s) s.enabled = !s.enabled
}

function addSchedule() {
  // TODO: backend v3.7.0 — POST /api/training/schedule
  const label = `${newScheduleFreq.value} · ${newScheduleDay.value} ${newScheduleTime.value} — ${newScheduleAction.value}`
  schedules.value.push({
    id: `sch-${Date.now()}`,
    label,
    cron: newScheduleFreq.value === 'custom' ? '* * * * *' : '0 3 * * 1',
    action: newScheduleAction.value,
    enabled: true,
  })
}

function addQaPair() {
  // TODO: backend v3.7.0 — POST /api/training/evaluations
  if (!newQaQuestion.value.trim() || !newQaExpected.value.trim()) return
  qaPairs.value.push({
    id: `qa-${Date.now()}`,
    question: newQaQuestion.value.trim(),
    expected: newQaExpected.value.trim(),
    actual: null,
    score: null,
    status: 'pending',
  })
  newQaQuestion.value = ''
  newQaExpected.value = ''
}

async function runEvaluation() {
  // TODO: backend v3.7.0 — POST /api/training/evaluations/run
  runningEval.value = true
  try {
    await new Promise((r) => setTimeout(r, 1200))
    for (const q of qaPairs.value) {
      if (q.status !== 'pending') continue
      // Mock: kirim ke /api/chat dan compare via keyword match
      try {
        const res = await api.post<{ assistantMessage?: { content?: string } }>('/api/chat', {
          conversationId: null,
          message: q.question,
        }).catch(() => null)
        const actual = res?.data?.assistantMessage?.content ?? '[no response — backend offline]'
        q.actual = actual
        // simple keyword similarity
        const words = q.expected.toLowerCase().split(/\s+/).filter(Boolean)
        const matched = words.filter((w) => actual.toLowerCase().includes(w)).length
        q.score = Math.min(1, matched / words.length)
        q.status = q.score > 0.85 ? 'excellent' : q.score > 0.6 ? 'good' : 'need_improvement'
      } catch {
        q.actual = '[error]'
        q.score = 0
        q.status = 'need_improvement'
      }
    }
    // update model status
    modelStatus.value.evaluationScore = Math.round(evalStats.value.avg)
    modelStatus.value.lastTrainingAt = new Date().toISOString()
    history.value.unshift({
      id: `h-${Date.now()}`,
      timestamp: new Date().toISOString(),
      type: 'evaluasi',
      message: `Evaluasi ${qaPairs.value.length} Q&A pair — akurasi ${Math.round(evalStats.value.avg)}%`,
      success: true,
      durationSec: 12,
    })
  } finally {
    runningEval.value = false
  }
}

onMounted(() => {
  // TODO: backend v3.7.0 — fetch /api/training/status, /materials, /schedule, /history, /evaluations
})
</script>

<template>
  <div class="mx-auto w-full max-w-6xl px-4 py-6">
    <!-- Page header -->
    <div class="flex items-center justify-between">
      <div>
        <div class="flex items-center gap-2 text-[11px] font-semibold uppercase tracking-widest text-accent">
          <span class="inline-block h-2 w-2 rounded-sm bg-accent" aria-hidden />
          Training &amp; Evaluasi
        </div>
        <h1 class="mt-1.5 text-2xl font-extrabold tracking-tight text-foreground">
          Manajemen Training Lawu Model Fine
        </h1>
        <p class="mt-1 text-sm text-muted-foreground">
          Atur materi, jadwal, dan evaluasi kecerdasan Lawu Fine · AI Lokal Indonesia.
        </p>
      </div>
      <span class="hidden items-center gap-1.5 rounded-md bg-secondary px-2.5 py-1 text-xs font-medium text-secondary-foreground sm:inline-flex">
        <Mountain class="h-3.5 w-3.5 text-primary" />
        RAG + Evaluasi · CPU-only
      </span>
    </div>

    <!-- 1. Hero: Status Model Lawu Fine -->
    <section aria-label="Status model" class="mt-5">
      <div class="overflow-hidden rounded-2xl border border-border bg-card p-5 shadow-lawu-sm sm:p-6">
        <div class="flex flex-wrap items-start justify-between gap-3">
          <div class="flex items-center gap-3">
            <div class="flex h-12 w-12 items-center justify-center rounded-xl bg-primary/10 text-primary ring-1 ring-primary/20">
              <GraduationCap class="h-6 w-6" />
            </div>
            <div>
              <h2 class="text-lg font-bold tracking-tight text-foreground">Status Model Lawu Fine</h2>
              <p class="mt-0.5 text-xs text-muted-foreground">
                {{ modelStatus.model }}
              </p>
            </div>
          </div>
          <span
            class="inline-flex items-center gap-1.5 rounded-full px-3 py-1 text-xs font-bold"
            :class="modelStatus.active ? 'bg-primary/10 text-primary' : 'bg-accent/10 text-accent'"
          >
            <span class="inline-block h-2 w-2 rounded-full animate-lawu-pulse" :class="modelStatus.active ? 'bg-primary' : 'bg-accent'" />
            {{ modelStatus.active ? 'Active' : 'Inactive' }}
          </span>
        </div>

        <!-- Stats grid 4 kolom -->
        <div class="mt-5 grid grid-cols-2 gap-3 sm:grid-cols-4">
          <div class="rounded-xl border border-border bg-background px-3 py-2.5">
            <p class="flex items-center gap-1.5 text-[10.5px] uppercase tracking-wide text-muted-foreground">
              <Database class="h-3 w-3" /> Dokumen Dipelajari
            </p>
            <p class="mt-1 text-xl font-extrabold tabular-nums text-foreground">
              {{ modelStatus.totalDocs.toLocaleString('id-ID') }}
            </p>
          </div>
          <div class="rounded-xl border border-border bg-background px-3 py-2.5">
            <p class="flex items-center gap-1.5 text-[10.5px] uppercase tracking-wide text-muted-foreground">
              <Activity class="h-3 w-3" /> Sesi Training
            </p>
            <p class="mt-1 text-xl font-extrabold tabular-nums text-foreground">{{ modelStatus.totalSessions }}</p>
          </div>
          <div class="rounded-xl border border-border bg-background px-3 py-2.5">
            <p class="flex items-center gap-1.5 text-[10.5px] uppercase tracking-wide text-muted-foreground">
              <Clock class="h-3 w-3" /> Belajar Terakhir
            </p>
            <p class="mt-1 text-xs font-bold text-foreground">{{ fmtTime(modelStatus.lastTrainingAt) }}</p>
          </div>
          <div class="rounded-xl border border-border bg-background px-3 py-2.5">
            <p class="flex items-center gap-1.5 text-[10.5px] uppercase tracking-wide text-muted-foreground">
              <Target class="h-3 w-3" /> Akurasi Evaluasi
            </p>
            <p class="mt-1 text-xl font-extrabold tabular-nums text-primary">{{ modelStatus.evaluationScore }}%</p>
          </div>
        </div>

        <!-- Mountain progress bar visual -->
        <div class="mt-5">
          <div class="flex items-center justify-between text-xs">
            <span class="font-medium text-foreground">Knowledge Coverage</span>
            <span class="tabular-nums text-muted-foreground">{{ modelStatus.coveragePct }}% dari target kurikulum</span>
          </div>
          <div class="relative mt-2 h-6 w-full overflow-hidden rounded-lg border border-border bg-background">
            <!-- Background mountain silhouette -->
            <svg viewBox="0 0 600 30" preserveAspectRatio="none" class="absolute inset-0 h-full w-full opacity-30" aria-hidden="true">
              <path d="M0 30 L60 8 L120 22 L180 4 L240 18 L300 6 L360 20 L420 8 L480 18 L540 6 L600 20 L600 30 Z" fill="currentColor" class="text-primary" />
            </svg>
            <!-- Progress fill -->
            <div
              class="absolute inset-y-0 left-0 bg-primary/30 transition-all"
              :style="{ width: `${modelStatus.coveragePct}%` }"
            />
            <div
              class="absolute inset-y-0 left-0 border-r-2 border-primary transition-all"
              :style="{ width: `${modelStatus.coveragePct}%` }"
            />
            <div class="absolute inset-0 flex items-center justify-center text-[10.5px] font-bold tabular-nums text-foreground">
              {{ modelStatus.coveragePct }}% covered
            </div>
          </div>
          <p class="mt-1 text-[10.5px] text-muted-foreground">
            Estimasi berdasarkan target kurikulum: 2,000,000 dokumen.
          </p>
        </div>
      </div>
    </section>

    <!-- 2. Materi Training (table + upload) -->
    <section aria-label="Materi training" class="mt-8">
      <div class="flex items-center justify-between">
        <div>
          <h2 class="text-sm font-bold text-foreground">Materi Training</h2>
          <p class="mt-0.5 text-xs text-muted-foreground">Kelola dokumen yang dipelajari Lawu Fine</p>
        </div>
        <span class="text-[10.5px] text-muted-foreground">
          Total: <span class="font-bold tabular-nums text-foreground">{{ totalMaterialsDocs.toLocaleString('id-ID') }}</span> dokumen
        </span>
      </div>

      <!-- Tabel materi yang sudah dipelajari -->
      <div class="mt-3 overflow-hidden rounded-xl border border-border bg-card shadow-lawu-sm">
        <div class="overflow-x-auto">
          <table class="min-w-full divide-y divide-border text-left text-sm">
            <thead class="bg-secondary/60 text-[11px] uppercase tracking-wider text-muted-foreground">
              <tr>
                <th class="px-4 py-2.5 font-medium">Sumber</th>
                <th class="px-4 py-2.5 font-medium">Kategori</th>
                <th class="px-4 py-2.5 font-medium">Jumlah Dok</th>
                <th class="px-4 py-2.5 font-medium">Status</th>
                <th class="px-4 py-2.5 font-medium text-right">Aksi</th>
              </tr>
            </thead>
            <tbody class="divide-y divide-border">
              <tr v-for="m in materials" :key="m.id" class="hover:bg-secondary/40">
                <td class="px-4 py-2.5">
                  <div class="flex items-center gap-2">
                    <Database class="h-3.5 w-3.5 text-primary" />
                    <span class="font-medium text-foreground">{{ m.source }}</span>
                  </div>
                </td>
                <td class="px-4 py-2.5">
                  <span class="rounded-md bg-muted px-2 py-0.5 text-[10.5px] font-medium text-muted-foreground">{{ m.category }}</span>
                </td>
                <td class="px-4 py-2.5 tabular-nums text-foreground">{{ m.docCount.toLocaleString('id-ID') }}</td>
                <td class="px-4 py-2.5">
                  <span
                    class="inline-flex items-center gap-1 rounded-md px-2 py-0.5 text-[10.5px] font-bold"
                    :class="m.status === 'terindex'
                      ? 'bg-primary/10 text-primary'
                      : m.status === 'pending'
                        ? 'bg-secondary text-secondary-foreground'
                        : 'bg-accent/10 text-accent'"
                  >
                    <span class="inline-block h-1.5 w-1.5 rounded-full"
                      :class="m.status === 'terindex' ? 'bg-primary' : m.status === 'pending' ? 'bg-muted-foreground' : 'bg-accent'" />
                    {{ m.status === 'terindex' ? 'Terindex' : m.status === 'pending' ? 'Pending' : 'Gagal' }}
                  </span>
                </td>
                <td class="px-4 py-2.5 text-right">
                  <button type="button" class="text-[10.5px] font-medium text-muted-foreground hover:text-foreground">
                    Lihat
                  </button>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>

      <!-- Form upload materi baru -->
      <div class="mt-4 rounded-xl border border-border bg-card p-5 shadow-lawu-sm">
        <h3 class="text-sm font-bold text-foreground">Upload Materi Baru</h3>
        <p class="mt-0.5 text-xs text-muted-foreground">
          PDF, EPUB, DOCX, TXT, atau JSONL — akan masuk ke antrian ingest.
        </p>

        <div
          class="mt-3 flex flex-col items-center justify-center gap-2 rounded-xl border-2 border-dashed px-4 py-8 text-center transition-colors"
          :class="uploadDragging ? 'border-accent bg-accent/5' : 'border-border bg-background hover:bg-secondary/50'"
          role="button" tabindex="0"
          aria-label="Area upload materi training. Klik atau seret file."
          @click="uploadInput?.click()"
          @keydown.enter.prevent="uploadInput?.click()"
          @dragover.prevent="uploadDragging = true"
          @dragleave.prevent="uploadDragging = false"
          @drop.prevent="onUploadDrop"
        >
          <LoaderCircle v-if="uploading" class="h-8 w-8 animate-spin text-accent" />
          <UploadCloud v-else class="h-8 w-8 text-muted-foreground" />
          <p class="text-sm font-medium text-foreground">
            {{ uploading ? 'Mengunggah…' : 'Seret &amp; letakkan file di sini, atau klik untuk memilih' }}
          </p>
          <p v-if="uploadFiles.length > 0" class="text-xs text-foreground">
            {{ uploadFiles.length }} file terpilih
          </p>
          <input ref="uploadInput" type="file" accept=".pdf,.epub,.docx,.txt,.jsonl" multiple class="hidden" @change="onUploadPick" />
        </div>

        <div class="mt-3 grid gap-3 sm:grid-cols-2">
          <div>
            <label class="text-xs text-muted-foreground" for="upload-cat">Kategori</label>
            <select id="upload-cat" v-model="uploadCategory" class="mt-1 h-10 w-full rounded-lg border border-border bg-background px-3 text-sm">
              <option>Teknologi</option>
              <option>Keamanan</option>
              <option>AI/ML</option>
              <option>Siber</option>
              <option>Indonesia</option>
              <option>Custom</option>
            </select>
          </div>
          <div>
            <label class="text-xs text-muted-foreground" for="upload-prio">Prioritas</label>
            <select id="upload-prio" v-model="uploadPriority" class="mt-1 h-10 w-full rounded-lg border border-border bg-background px-3 text-sm">
              <option>Tinggi</option>
              <option>Sedang</option>
              <option>Rendah</option>
            </select>
          </div>
        </div>

        <button
          type="button"
          @click="submitUpload"
          :disabled="uploadFiles.length === 0 || uploading"
          class="mt-3 flex w-full items-center justify-center gap-2 rounded-lg bg-primary px-4 py-2.5 text-sm font-semibold text-primary-foreground shadow-lawu-sm hover:bg-primary/90 disabled:opacity-50"
        >
          <LoaderCircle v-if="uploading" class="h-4 w-4 animate-spin" />
          <UploadCloud v-else class="h-4 w-4" /> Upload &amp; Queue
        </button>
      </div>

      <!-- Antrian Materi -->
      <div v-if="queue.length > 0" class="mt-4 rounded-xl border border-border bg-card p-5 shadow-lawu-sm">
        <h3 class="text-sm font-bold text-foreground">Antrian Materi (pending ingest)</h3>
        <ul class="mt-3 divide-y divide-border">
          <li v-for="q in queue" :key="q.id" class="flex flex-wrap items-center gap-3 py-2.5">
            <FileText class="h-4 w-4 shrink-0 text-muted-foreground" />
            <div class="min-w-0 flex-1">
              <p class="truncate text-sm font-medium text-foreground" :title="q.filename">{{ q.filename }}</p>
              <p class="text-[10.5px] text-muted-foreground">
                {{ q.category }} · {{ q.priority }} · {{ humanSize(q.sizeBytes) }} · {{ fmtTime(q.uploadedAt) }}
              </p>
            </div>
            <button
              type="button"
              @click="ingestQueueItem(q)"
              class="inline-flex items-center gap-1.5 rounded-md bg-primary px-3 py-1.5 text-[11px] font-semibold text-primary-foreground hover:bg-primary/90"
            >
              <Play class="h-3 w-3" /> Belajar Sekarang
            </button>
          </li>
        </ul>
      </div>
    </section>

    <!-- 3. Jadwal Training (Curriculum Scheduler) -->
    <section aria-label="Jadwal training otomatis" class="mt-8">
      <div class="rounded-2xl border border-border bg-card p-5 shadow-lawu-sm sm:p-6">
        <div class="flex items-center gap-2">
          <Calendar class="h-4 w-4 text-primary" />
          <h2 class="text-sm font-bold text-foreground">Jadwal Training Otomatis</h2>
        </div>
        <p class="mt-0.5 text-xs text-muted-foreground">Atur kapan Lawu Fine harus belajar secara berkala</p>

        <!-- List schedule existing -->
        <ul class="mt-4 space-y-2">
          <li
            v-for="s in schedules"
            :key="s.id"
            class="flex items-center gap-3 rounded-lg border border-border bg-background px-3 py-2.5"
          >
            <button
              type="button"
              class="relative inline-flex h-5 w-9 shrink-0 items-center rounded-full transition-colors"
              :class="s.enabled ? 'bg-primary' : 'bg-muted-foreground/30'"
              :aria-pressed="s.enabled"
              :aria-label="`Toggle jadwal ${s.label}`"
              @click="toggleSchedule(s.id)"
            >
              <span
                class="inline-block h-4 w-4 transform rounded-full bg-white shadow transition-transform"
                :class="s.enabled ? 'translate-x-4' : 'translate-x-0.5'"
              />
            </button>
            <div class="min-w-0 flex-1">
              <p class="text-sm font-medium text-foreground">{{ s.label }}</p>
              <p class="text-[10.5px] font-mono text-muted-foreground">cron: {{ s.cron }} · action: {{ s.action }}</p>
            </div>
            <span
              class="rounded-md px-2 py-0.5 text-[10px] font-bold uppercase tracking-wide"
              :class="s.enabled ? 'bg-primary/10 text-primary' : 'bg-muted text-muted-foreground'"
            >
              {{ s.enabled ? 'Enable' : 'Disable' }}
            </span>
          </li>
        </ul>

        <!-- Form tambah jadwal baru -->
        <div class="mt-5 rounded-lg border border-dashed border-border bg-background p-4">
          <p class="text-xs font-semibold text-foreground">Tambah Jadwal Baru</p>
          <div class="mt-3 grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
            <div>
              <label class="text-[10.5px] text-muted-foreground" for="sch-freq">Frekuensi</label>
              <select id="sch-freq" v-model="newScheduleFreq" class="mt-1 h-9 w-full rounded-md border border-border bg-card px-2 text-sm">
                <option value="harian">Harian</option>
                <option value="mingguan">Mingguan</option>
                <option value="bulanan">Bulanan</option>
                <option value="custom">Custom cron</option>
              </select>
            </div>
            <div>
              <label class="text-[10.5px] text-muted-foreground" for="sch-day">Hari</label>
              <select id="sch-day" v-model="newScheduleDay" class="mt-1 h-9 w-full rounded-md border border-border bg-card px-2 text-sm">
                <option>Senin</option>
                <option>Selasa</option>
                <option>Rabu</option>
                <option>Kamis</option>
                <option>Jumat</option>
                <option>Sabtu</option>
                <option>Minggu</option>
              </select>
            </div>
            <div>
              <label class="text-[10.5px] text-muted-foreground" for="sch-time">Jam</label>
              <input id="sch-time" v-model="newScheduleTime" type="time" class="mt-1 h-9 w-full rounded-md border border-border bg-card px-2 text-sm" />
            </div>
            <div>
              <label class="text-[10.5px] text-muted-foreground" for="sch-act">Aksi</label>
              <select id="sch-act" v-model="newScheduleAction" class="mt-1 h-9 w-full rounded-md border border-border bg-card px-2 text-sm">
                <option value="crawl_web">Crawl Web</option>
                <option value="extract_ebooks">Extract Ebook</option>
                <option value="ingest_skill">Ingest Skill</option>
                <option value="custom">Custom</option>
              </select>
            </div>
          </div>
          <button
            type="button"
            @click="addSchedule"
            class="mt-3 inline-flex items-center gap-1.5 rounded-md bg-primary px-3 py-1.5 text-xs font-semibold text-primary-foreground hover:bg-primary/90"
          >
            <Plus class="h-3.5 w-3.5" /> Tambah Jadwal
          </button>
        </div>
      </div>
    </section>

    <!-- 4. Riwayat Training (timeline) -->
    <section aria-label="Riwayat training" class="mt-8">
      <div class="flex items-center justify-between">
        <div>
          <h2 class="flex items-center gap-2 text-sm font-bold text-foreground">
            <History class="h-4 w-4 text-primary" /> Riwayat Sesi Training
          </h2>
          <p class="mt-0.5 text-xs text-muted-foreground">Log semua aktivitas belajar Lawu Fine</p>
        </div>
        <select v-model="historyFilter" class="h-9 rounded-md border border-border bg-card px-2 text-xs">
          <option value="semua">Semua</option>
          <option value="crawl">Crawl</option>
          <option value="extract">Extract</option>
          <option value="ingest">Ingest</option>
          <option value="evaluasi">Evaluasi</option>
        </select>
      </div>

      <ol class="mt-4 space-y-3">
        <li v-for="h in filteredHistory" :key="h.id" class="relative flex gap-3 rounded-xl border border-border bg-card p-3.5 shadow-lawu-sm">
          <!-- Timeline dot + icon -->
          <div class="flex shrink-0 flex-col items-center">
            <div
              class="flex h-9 w-9 items-center justify-center rounded-full"
              :class="h.success ? 'bg-primary/10 text-primary' : 'bg-accent/10 text-accent'"
            >
              <component :is="historyTypeIcon(h.type)" class="h-4 w-4" />
            </div>
          </div>
          <div class="min-w-0 flex-1">
            <div class="flex flex-wrap items-center gap-2">
              <span class="rounded-md bg-secondary px-1.5 py-0.5 text-[10px] font-bold uppercase tracking-wide text-secondary-foreground">
                {{ historyTypeLabel(h.type) }}
              </span>
              <p class="text-sm font-semibold text-foreground">{{ h.message }}</p>
              <span
                v-if="h.success"
                class="inline-flex items-center gap-1 rounded-md bg-primary/10 px-1.5 py-0.5 text-[10px] font-medium text-primary"
              >
                <CheckCircle2 class="h-3 w-3" /> Sukses
              </span>
              <span
                v-else
                class="inline-flex items-center gap-1 rounded-md bg-accent/10 px-1.5 py-0.5 text-[10px] font-medium text-accent"
              >
                <AlertTriangle class="h-3 w-3" /> Gagal
              </span>
              <span v-if="h.failedCount" class="text-[10.5px] text-accent">
                {{ h.failedCount }} gagal
              </span>
            </div>
            <p class="mt-1 text-[10.5px] text-muted-foreground">
              {{ fmtTime(h.timestamp) }} · durasi {{ fmtDuration(h.durationSec) }}
            </p>
          </div>
        </li>
      </ol>

      <div class="mt-4 text-center">
        <button type="button" class="inline-flex items-center gap-1.5 rounded-md border border-border bg-card px-3 py-1.5 text-xs font-medium text-foreground hover:bg-secondary">
          <LoaderCircle class="h-3.5 w-3.5" /> Muat Lebih Lama
        </button>
      </div>
    </section>

    <!-- 5. Evaluasi Model -->
    <section aria-label="Evaluasi model" class="mt-8">
      <div class="rounded-2xl border border-border bg-card p-5 shadow-lawu-sm sm:p-6">
        <div class="flex items-center gap-2">
          <Target class="h-4 w-4 text-primary" />
          <h2 class="text-sm font-bold text-foreground">Evaluasi Kecerdasan Lawu Fine</h2>
        </div>
        <p class="mt-0.5 text-xs text-muted-foreground">Test seberapa pintar Lawu Fine dengan Q&amp;A pairs</p>

        <!-- Form input Q&A -->
        <div class="mt-4 grid gap-3 sm:grid-cols-2">
          <div>
            <label class="text-[10.5px] text-muted-foreground" for="qa-q">Pertanyaan</label>
            <textarea
              id="qa-q"
              v-model="newQaQuestion"
              rows="3"
              placeholder="Mis: Apa itu Lawu Fine?"
              class="mt-1 w-full rounded-lg border border-border bg-background px-3 py-2 text-sm focus:border-primary focus:outline-none"
            />
          </div>
          <div>
            <label class="text-[10.5px] text-muted-foreground" for="qa-a">Jawaban Expected</label>
            <textarea
              id="qa-a"
              v-model="newQaExpected"
              rows="3"
              placeholder="Mis: Lawu Fine adalah mesin pencari ebook lokal Indonesia."
              class="mt-1 w-full rounded-lg border border-border bg-background px-3 py-2 text-sm focus:border-primary focus:outline-none"
            />
          </div>
        </div>
        <button
          type="button"
          @click="addQaPair"
          :disabled="!newQaQuestion.trim() || !newQaExpected.trim()"
          class="mt-3 inline-flex items-center gap-1.5 rounded-md bg-secondary px-3 py-1.5 text-xs font-semibold text-secondary-foreground hover:bg-secondary/70 disabled:opacity-50"
        >
          <Plus class="h-3.5 w-3.5" /> Tambah ke Test Suite
        </button>

        <!-- List Q&A pairs -->
        <div class="mt-5 overflow-hidden rounded-xl border border-border bg-background">
          <table class="min-w-full divide-y divide-border text-left text-sm">
            <thead class="bg-secondary/60 text-[11px] uppercase tracking-wider text-muted-foreground">
              <tr>
                <th class="px-4 py-2.5 font-medium">Pertanyaan</th>
                <th class="px-4 py-2.5 font-medium">Jawaban Expected</th>
                <th class="px-4 py-2.5 font-medium">Jawaban Lawu</th>
                <th class="px-4 py-2.5 font-medium">Skor</th>
                <th class="px-4 py-2.5 font-medium">Status</th>
              </tr>
            </thead>
            <tbody class="divide-y divide-border">
              <tr v-for="q in qaPairs" :key="q.id" class="align-top hover:bg-secondary/40">
                <td class="px-4 py-2.5 text-xs text-foreground">{{ q.question }}</td>
                <td class="px-4 py-2.5 text-xs text-muted-foreground">{{ q.expected }}</td>
                <td class="px-4 py-2.5 text-xs text-muted-foreground">
                  <span v-if="q.actual" class="line-clamp-2">{{ q.actual }}</span>
                  <span v-else class="italic text-muted-foreground">— belum dijalankan —</span>
                </td>
                <td class="px-4 py-2.5">
                  <span v-if="q.score !== null" class="tabular-nums font-bold text-foreground">
                    {{ Math.round(q.score * 100) }}%
                  </span>
                  <span v-else class="text-muted-foreground">—</span>
                </td>
                <td class="px-4 py-2.5">
                  <span
                    class="inline-flex items-center gap-1 rounded-md px-2 py-0.5 text-[10px] font-bold"
                    :class="qaStatusBadge(q.status).class"
                  >
                    {{ qaStatusBadge(q.status).label }}
                  </span>
                </td>
              </tr>
            </tbody>
          </table>
        </div>

        <!-- Hasil terakhir -->
        <div class="mt-5 grid grid-cols-2 gap-3 sm:grid-cols-4">
          <div class="rounded-lg border border-border bg-background px-3 py-2.5">
            <p class="text-[10.5px] uppercase tracking-wide text-muted-foreground">Total Soal</p>
            <p class="mt-1 text-lg font-extrabold tabular-nums text-foreground">{{ evalStats.total }}</p>
          </div>
          <div class="rounded-lg border border-border bg-background px-3 py-2.5">
            <p class="text-[10.5px] uppercase tracking-wide text-muted-foreground">Rata-rata Skor</p>
            <p class="mt-1 text-lg font-extrabold tabular-nums text-primary">{{ Math.round(evalStats.avg) }}%</p>
          </div>
          <div class="rounded-lg border border-border bg-background px-3 py-2.5">
            <p class="text-[10.5px] uppercase tracking-wide text-muted-foreground">Excellent</p>
            <p class="mt-1 text-lg font-extrabold tabular-nums text-foreground">{{ evalStats.excellent }}</p>
          </div>
          <div class="rounded-lg border border-border bg-background px-3 py-2.5">
            <p class="text-[10.5px] uppercase tracking-wide text-muted-foreground">Need Improvement</p>
            <p class="mt-1 text-lg font-extrabold tabular-nums text-accent">{{ evalStats.needImp }}</p>
          </div>
        </div>

        <!-- Tombol Jalankan Evaluasi (full width) -->
        <button
          type="button"
          @click="runEvaluation"
          :disabled="runningEval"
          class="mt-5 flex w-full items-center justify-center gap-2 rounded-lg bg-primary px-4 py-3 text-sm font-bold text-primary-foreground shadow-lawu-sm hover:bg-primary/90 disabled:opacity-50"
        >
          <LoaderCircle v-if="runningEval" class="h-4 w-4 animate-spin" />
          <Sparkles v-else class="h-4 w-4" />
          {{ runningEval ? 'Menjalankan evaluasi…' : 'Jalankan Evaluasi' }}
        </button>
        <p class="mt-2 text-center text-[10.5px] text-muted-foreground">
          Kirim semua pertanyaan ke /api/chat, compare jawaban dengan expected via keyword similarity.
        </p>
      </div>
    </section>

    <!-- Catatan konsep training CPU-only -->
    <p class="mt-8 flex items-start gap-2 rounded-lg border border-border bg-card px-3 py-2.5 text-xs text-muted-foreground">
      <GraduationCap class="mt-0.5 h-4 w-4 shrink-0 text-primary" />
      <span>
        <span class="font-medium text-foreground">Konsep "Training" di Lawu Fine (CPU-only):</span>
        Lawu Fine tidak melakukan fine-tune bobot model (butuh GPU).
        Yang dilakukan di sini adalah <span class="font-medium text-foreground">RAG augmentation</span>
        (ingest dokumen baru ke Tantivy + Qdrant),
        <span class="font-medium text-foreground">evaluasi</span> (Q&amp;A test suite), dan
        <span class="font-medium text-foreground">curriculum scheduling</span> (jadwal belajar berkala).
      </span>
    </p>

    <!-- Backend mock note -->
    <p class="mt-3 flex items-start gap-2 rounded-lg border border-dashed border-border bg-muted/50 px-3 py-2.5 text-[11px] text-muted-foreground">
      <AlertTriangle class="mt-0.5 h-3.5 w-3.5 shrink-0 text-accent" />
      <span>
        <span class="font-medium text-foreground">TODO: backend v3.7.0.</span>
        Endpoint training (<code class="rounded bg-card px-1 py-0.5 font-mono text-[10px]">/api/training/*</code>)
        belum tersedia. Halaman ini menggunakan mock data — interface sudah siap, tinggal ganti
        <code class="rounded bg-card px-1 py-0.5 font-mono text-[10px]">mockFetch()</code> ke
        <code class="rounded bg-card px-1 py-0.5 font-mono text-[10px]">api.get()</code> saat backend siap.
      </span>
    </p>
  </div>
</template>
