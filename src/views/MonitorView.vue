<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import {
  Radio, RefreshCw, MessageCircleWarning, Send, ShieldCheck,
  Cpu, MemoryStick, HardDrive, Gauge, Zap, Server,
  Activity, Database, BookOpen, Globe, LoaderCircle, RotateCw,
} from 'lucide-vue-next'
import { api } from '@/lib/api'

interface Metrics {
  cpuPct: number
  ramUsedGb: number
  ramTotalGb: number
  ssdUsedGb: number
  ssdTotalGb: number
  hddUsedGb: number
  hddTotalGb: number
  loadAvg: number
  llamaTokPerSec: number
}

interface ServiceRow {
  name: string
  active: boolean
  sub?: string | null
  pid?: number | null
  uptimeSec?: number | null
  memoryMb?: number | null
}

interface HealthResponse {
  backend: boolean
  corpus: boolean
  auth: boolean
  chat: boolean
  monitor: boolean
  llama: boolean
}

interface AlertItem {
  id: string
  level: 'info' | 'success' | 'warning' | 'error' | string
  title: string
  message: string
  source: string
  delivered: boolean
  createdAt: string
}

interface StatsResponse {
  totalDocs?: number
  totalEbooks?: number
  totalChats?: number
  totalAlerts?: number
  totalWebCrawl?: number
  [k: string]: unknown
}

const metrics = ref<Metrics | null>(null)
const services = ref<ServiceRow[]>([])
const health = ref<HealthResponse | null>(null)
const alerts = ref<AlertItem[]>([])
const stats = ref<StatsResponse | null>(null)

const tokenInput = ref('')
const chatIdInput = ref('')
const lastPreview = ref<string | null>(null)
const errorMsg = ref<string | null>(null)
const lastChecked = ref<string>('')
let timer: ReturnType<typeof setInterval> | null = null

async function loadMetrics() {
  try {
    const res = await api.get<Metrics>('/api/monitor/metrics')
    metrics.value = res.data
  } catch (err: unknown) {
    const e = err as { response?: { status?: number } }
    if (e?.response?.status === 401) return // interceptor akan redirect
    // silent untuk error lain — tetap tampilkan data lama.
  }
}

async function loadServices() {
  try {
    const res = await api.get<{ services: ServiceRow[] }>('/api/monitor/services')
    services.value = res.data.services ?? []
  } catch {
    // silent
  }
}

async function loadHealth() {
  try {
    const res = await api.get<HealthResponse>('/api/monitor/health')
    health.value = res.data
    lastChecked.value = new Date().toLocaleTimeString('id-ID', { hour: '2-digit', minute: '2-digit', second: '2-digit' })
  } catch {
    // silent
  }
}

async function loadAlerts() {
  try {
    const res = await fetch('/api/alerts')
    if (res.ok) {
      const data = await res.json()
      alerts.value = Array.isArray(data) ? data : (data?.alerts ?? [])
    }
  } catch {
    // silent
  }
}

async function loadStats() {
  try {
    const res = await fetch('/api/stats')
    if (res.ok) {
      stats.value = await res.json()
    }
  } catch {
    // silent
  }
}

async function loadAll() {
  await Promise.allSettled([loadMetrics(), loadServices(), loadHealth(), loadAlerts(), loadStats()])
}

async function runTest() {
  try {
    const res = await fetch('/api/telegram/test', { method: 'POST' })
    const data = await res.json()
    if (data?.payloadPreview?.text) lastPreview.value = data.payloadPreview.text
    await loadAlerts()
  } catch {
    // silent
  }
}

async function restartService(name: string) {
  // TODO: backend v3.7.0 — POST /api/monitor/service/:name/restart belum ada.
  // Sementara: kasih pesan placeholder supaya UI tetap berfungsi.
  if (!confirm(`Restart service "${name}"? Backend v3.7.0 belum tersedia, aksi ini placeholder.`)) return
  errorMsg.value = `Endpoint restart service belum tersedia di backend (akan hadir di v3.7.0). Service: ${name}`
  setTimeout(() => { errorMsg.value = null }, 6000)
}

function pct(used: number, total: number): number {
  if (!total) return 0
  return Math.min(100, Math.max(0, (used / total) * 100))
}

function fmtUptime(sec?: number | null): string {
  if (!sec || sec < 0) return '—'
  if (sec < 60) return `${sec.toFixed(0)} dtk`
  if (sec < 3600) return `${(sec / 60).toFixed(0)} mnt`
  if (sec < 86400) return `${(sec / 3600).toFixed(1)} jam`
  return `${(sec / 86400).toFixed(1)} hari`
}

function fmtMem(mb?: number | null): string {
  if (!mb || mb < 0) return '—'
  if (mb < 1024) return `${mb.toFixed(0)} MB`
  return `${(mb / 1024).toFixed(2)} GB`
}

function formatLevel(level: string) {
  const styles: Record<string, { label: string; class: string }> = {
    info: { label: 'Info', class: 'bg-lawuSky/10 text-lawuSky' },
    success: { label: 'Sukses', class: 'bg-primary/10 text-primary' },
    warning: { label: 'Peringatan', class: 'bg-secondary text-secondary-foreground' },
    error: { label: 'Kritis', class: 'bg-accent/10 text-accent' },
  }
  return styles[level] ?? styles.info
}

function loadLevelLabel(load: number): { label: string; class: string } {
  if (load < 1) return { label: 'Rendah', class: 'text-primary' }
  if (load < 4) return { label: 'Sedang', class: 'text-secondary-foreground' }
  return { label: 'Tinggi', class: 'text-accent' }
}

const SERVICE_DEFAULTS: ServiceRow[] = [
  { name: 'lawu-server', active: false, pid: null, uptimeSec: null, memoryMb: null },
  { name: 'lawu-corpus', active: false, pid: null, uptimeSec: null, memoryMb: null },
  { name: 'lawu-auth', active: false, pid: null, uptimeSec: null, memoryMb: null },
  { name: 'lawu-chat', active: false, pid: null, uptimeSec: null, memoryMb: null },
  { name: 'lawu-monitor', active: false, pid: null, uptimeSec: null, memoryMb: null },
  { name: 'lawu-llama', active: false, pid: null, uptimeSec: null, memoryMb: null },
  { name: 'nginx', active: false, pid: null, uptimeSec: null, memoryMb: null },
]

const servicesView = computed<ServiceRow[]>(() => {
  if (services.value.length > 0) return services.value
  return SERVICE_DEFAULTS
})

const totalDocs = computed(() => stats.value?.totalDocs ?? 1245903)
const totalEbooks = computed(() => stats.value?.totalEbooks ?? 31)
const totalWebCrawl = computed(() => stats.value?.totalWebCrawl ?? 123)
const totalChats = computed(() => stats.value?.totalChats ?? 0)
const totalAlertsCount = computed(() => stats.value?.totalAlerts ?? alerts.value.length)

const healthDots = computed(() => {
  const h = health.value
  return [
    { name: 'Backend', ok: !!h?.backend },
    { name: 'Corpus', ok: !!h?.corpus },
    { name: 'Auth', ok: !!h?.auth },
    { name: 'Chat', ok: !!h?.chat },
    { name: 'Monitor', ok: !!h?.monitor },
    { name: 'Llama', ok: !!h?.llama },
  ]
})

const overallHealth = computed(() => {
  if (!health.value) return null
  const all = Object.values(health.value)
  const ok = all.filter(Boolean).length
  return { ok, total: all.length, allUp: ok === all.length }
})

onMounted(() => {
  loadAll()
  timer = setInterval(loadAll, 8000)
})

onUnmounted(() => {
  if (timer) clearInterval(timer)
})
</script>

<template>
  <div class="mx-auto w-full max-w-6xl px-4 py-6">
    <!-- Page header -->
    <div class="flex items-center justify-between">
      <div>
        <div class="flex items-center gap-2 text-[11px] font-semibold uppercase tracking-widest text-accent">
          <span class="inline-block h-2 w-2 rounded-sm bg-accent" aria-hidden />
          Monitoring Sistem
        </div>
        <h1 class="mt-1.5 text-2xl font-extrabold tracking-tight text-foreground">
          Status Sistem Lawu Fine
        </h1>
        <p class="mt-1 text-sm text-muted-foreground">
          Metrik real-time &amp; status service Lawu Fine · AI Lokal Indonesia.
        </p>
      </div>
      <span class="inline-flex items-center gap-1.5 rounded-md bg-secondary px-2.5 py-1 text-xs font-medium text-secondary-foreground">
        <Radio class="h-3 w-3 animate-lawu-pulse text-primary" />
        Live · polling 8 dtk
      </span>
    </div>

    <!-- Error -->
    <div
      v-if="errorMsg"
      class="mt-3 rounded-lg border border-destructive/40 bg-destructive/10 px-3 py-2 text-xs text-destructive"
    >
      {{ errorMsg }}
    </div>

    <!-- 1. Hero Status Card (full width) -->
    <section aria-label="Status kesehatan sistem" class="mt-5">
      <div class="overflow-hidden rounded-2xl border border-border bg-card p-5 shadow-lawu-sm sm:p-6">
        <div class="flex flex-wrap items-start justify-between gap-3">
          <div>
            <h2 class="text-lg font-bold tracking-tight text-foreground">
              Status Sistem Lawu Fine
            </h2>
            <p class="mt-1 text-xs text-muted-foreground">
              Last checked:
              <span class="font-mono text-foreground">{{ lastChecked || '—' }}</span>
            </p>
          </div>
          <div class="flex items-center gap-2">
            <span
              v-if="overallHealth"
              class="inline-flex items-center gap-1.5 rounded-full px-3 py-1 text-xs font-semibold"
              :class="overallHealth.allUp
                ? 'bg-primary/10 text-primary'
                : 'bg-accent/10 text-accent'"
            >
              <span
                class="inline-block h-2 w-2 rounded-full animate-lawu-pulse"
                :class="overallHealth.allUp ? 'bg-primary' : 'bg-accent'"
              />
              {{ overallHealth.allUp ? 'All Systems Operational' : 'Partial Outage' }}
            </span>
            <span
              v-else
              class="inline-flex items-center gap-1.5 rounded-full bg-muted px-3 py-1 text-xs text-muted-foreground"
            >
              <span class="inline-block h-2 w-2 rounded-full bg-muted-foreground" />
              Memuat…
            </span>
          </div>
        </div>

        <!-- 5+1 health dots -->
        <div class="mt-5 grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-6">
          <div
            v-for="dot in healthDots"
            :key="dot.name"
            class="rounded-xl border border-border bg-background px-3 py-2.5"
          >
            <p class="text-[10.5px] uppercase tracking-wide text-muted-foreground">{{ dot.name }}</p>
            <p class="mt-1 flex items-center gap-1.5 text-sm font-bold"
               :class="dot.ok ? 'text-primary' : 'text-accent'">
              <span
                class="inline-block h-2 w-2 rounded-full"
                :class="dot.ok ? 'bg-primary' : 'bg-accent'"
              />
              {{ dot.ok ? 'Active' : 'Down' }}
            </p>
          </div>
        </div>
      </div>
    </section>

    <!-- 2. Metrik Real-time (grid 3x2 / 2x3) -->
    <section aria-label="Metrik sistem real-time" class="mt-6">
      <div class="flex items-center justify-between">
        <h2 class="text-sm font-bold text-foreground">Metrik Real-time</h2>
        <button @click="loadAll" class="inline-flex items-center gap-1.5 rounded-md px-2 py-1 text-xs text-muted-foreground hover:bg-secondary">
          <RefreshCw class="h-3.5 w-3.5" /> Muat ulang
        </button>
      </div>

      <div class="mt-3 grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3">
        <!-- CPU gauge -->
        <div class="rounded-xl border border-border bg-card p-4 shadow-lawu-sm">
          <div class="flex items-center gap-2 text-xs text-muted-foreground">
            <Cpu class="h-4 w-4 text-primary" /> Beban CPU
          </div>
          <div class="mt-2 flex items-center gap-3">
            <div class="relative h-14 w-14 shrink-0">
              <svg viewBox="0 0 36 36" class="h-full w-full -rotate-90">
                <circle cx="18" cy="18" r="15.5" fill="none" stroke="currentColor" class="text-muted" stroke-width="3" />
                <circle
                  cx="18" cy="18" r="15.5" fill="none" stroke="currentColor" class="text-primary" stroke-width="3"
                  stroke-dasharray="97.4"
                  :stroke-dashoffset="97.4 - (97.4 * (metrics?.cpuPct ?? 0)) / 100"
                  stroke-linecap="round"
                />
              </svg>
              <div class="absolute inset-0 flex items-center justify-center text-[11px] font-bold tabular-nums text-foreground">
                {{ metrics ? metrics.cpuPct.toFixed(0) : '—' }}%
              </div>
            </div>
            <div class="min-w-0">
              <p v-if="metrics" class="text-xl font-extrabold tabular-nums text-foreground">{{ metrics.cpuPct.toFixed(1) }}%</p>
              <p v-else class="text-xl font-extrabold text-muted-foreground">—</p>
              <p class="text-[10.5px] text-muted-foreground">Beban prosesor</p>
            </div>
          </div>
        </div>

        <!-- RAM -->
        <div class="rounded-xl border border-border bg-card p-4 shadow-lawu-sm">
          <div class="flex items-center gap-2 text-xs text-muted-foreground">
            <MemoryStick class="h-4 w-4 text-primary" /> Memori Aktif
          </div>
          <p v-if="metrics" class="mt-1 text-xl font-extrabold tabular-nums text-foreground">
            {{ metrics.ramUsedGb.toFixed(1) }}<span class="text-sm font-medium text-muted-foreground"> / {{ metrics.ramTotalGb }} GB</span>
          </p>
          <p v-else class="mt-1 text-xl font-extrabold text-muted-foreground">—</p>
          <div class="mt-2 h-2 w-full overflow-hidden rounded-full bg-secondary">
            <div class="h-full bg-primary transition-all" :style="{ width: `${pct(metrics?.ramUsedGb ?? 0, metrics?.ramTotalGb ?? 1)}%` }" />
          </div>
          <p class="mt-1 text-[10.5px] tabular-nums text-muted-foreground">
            {{ pct(metrics?.ramUsedGb ?? 0, metrics?.ramTotalGb ?? 1).toFixed(0) }}% terpakai
          </p>
        </div>

        <!-- SSD (OS disk) -->
        <div class="rounded-xl border border-border bg-card p-4 shadow-lawu-sm">
          <div class="flex items-center gap-2 text-xs text-muted-foreground">
            <HardDrive class="h-4 w-4 text-primary" /> SSD Sistem
          </div>
          <p v-if="metrics" class="mt-1 text-xl font-extrabold tabular-nums text-foreground">
            {{ metrics.ssdUsedGb.toFixed(1) }}<span class="text-sm font-medium text-muted-foreground"> / {{ metrics.ssdTotalGb }} GB</span>
          </p>
          <p v-else class="mt-1 text-xl font-extrabold text-muted-foreground">—</p>
          <div class="mt-2 h-2 w-full overflow-hidden rounded-full bg-secondary">
            <div class="h-full bg-primary transition-all" :style="{ width: `${pct(metrics?.ssdUsedGb ?? 0, metrics?.ssdTotalGb ?? 1)}%` }" />
          </div>
          <p class="mt-1 text-[10.5px] tabular-nums text-muted-foreground">
            {{ pct(metrics?.ssdUsedGb ?? 0, metrics?.ssdTotalGb ?? 1).toFixed(0) }}% terpakai · OS disk
          </p>
        </div>

        <!-- HDD (data disk) -->
        <div class="rounded-xl border border-border bg-card p-4 shadow-lawu-sm">
          <div class="flex items-center gap-2 text-xs text-muted-foreground">
            <HardDrive class="h-4 w-4 text-primary" /> HDD Data
          </div>
          <p v-if="metrics" class="mt-1 text-xl font-extrabold tabular-nums text-foreground">
            {{ metrics.hddUsedGb.toFixed(1) }}<span class="text-sm font-medium text-muted-foreground"> / {{ metrics.hddTotalGb }} GB</span>
          </p>
          <p v-else class="mt-1 text-xl font-extrabold text-muted-foreground">—</p>
          <div class="mt-2 h-2 w-full overflow-hidden rounded-full bg-secondary">
            <div class="h-full bg-primary transition-all" :style="{ width: `${pct(metrics?.hddUsedGb ?? 0, metrics?.hddTotalGb ?? 1)}%` }" />
          </div>
          <p class="mt-1 text-[10.5px] tabular-nums text-muted-foreground">
            {{ pct(metrics?.hddUsedGb ?? 0, metrics?.hddTotalGb ?? 1).toFixed(0) }}% terpakai · data
          </p>
        </div>

        <!-- Load Average -->
        <div class="rounded-xl border border-border bg-card p-4 shadow-lawu-sm">
          <div class="flex items-center gap-2 text-xs text-muted-foreground">
            <Gauge class="h-4 w-4 text-primary" /> Beban Sistem
          </div>
          <p v-if="metrics" class="mt-1 text-xl font-extrabold tabular-nums text-foreground">{{ metrics.loadAvg.toFixed(2) }}</p>
          <p v-else class="mt-1 text-xl font-extrabold text-muted-foreground">—</p>
          <p
            v-if="metrics"
            class="mt-2 text-[10.5px] font-semibold uppercase tracking-wide"
            :class="loadLevelLabel(metrics?.loadAvg ?? 0).class"
          >
            {{ loadLevelLabel(metrics?.loadAvg ?? 0).label }} · load avg 1m
          </p>
          <p v-else class="mt-2 text-[10.5px] text-muted-foreground">Load average 1 menit</p>
        </div>

        <!-- Llama tok/detik -->
        <div class="rounded-xl border border-border bg-card p-4 shadow-lawu-sm">
          <div class="flex items-center gap-2 text-xs text-muted-foreground">
            <Zap class="h-4 w-4 text-primary" /> Throughput Model
          </div>
          <p v-if="metrics" class="mt-1 text-xl font-extrabold tabular-nums text-foreground">
            {{ metrics.llamaTokPerSec.toFixed(1) }}
            <span class="text-sm font-medium text-muted-foreground">tok/det</span>
          </p>
          <p v-else class="mt-1 text-xl font-extrabold text-muted-foreground">—</p>
          <p class="mt-2 inline-flex items-center gap-1.5 rounded-md px-2 py-0.5 text-[10.5px] font-semibold"
             :class="metrics && metrics.llamaTokPerSec > 0 ? 'bg-primary/10 text-primary' : 'bg-muted text-muted-foreground'">
            <span class="inline-block h-1.5 w-1.5 rounded-full" :class="metrics && metrics.llamaTokPerSec > 0 ? 'bg-primary animate-lawu-pulse' : 'bg-muted-foreground'" />
            {{ metrics && metrics.llamaTokPerSec > 0 ? 'Aktif' : 'Idle' }}
          </p>
        </div>
      </div>

      <!-- Skeleton -->
      <div v-if="!metrics" class="mt-3 grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3">
        <div v-for="i in 6" :key="i" class="rounded-xl border border-border bg-card p-4 shadow-lawu-sm">
          <div class="h-4 w-1/3 animate-pulse rounded bg-muted" />
          <div class="mt-2 h-7 w-2/3 animate-pulse rounded bg-muted" />
          <div class="mt-3 h-2 w-full animate-pulse rounded bg-muted" />
        </div>
      </div>
    </section>

    <!-- 3. Status Service Detail (tabel) -->
    <section aria-label="Status service detail" class="mt-8">
      <div class="flex items-center justify-between">
        <h2 class="text-sm font-bold text-foreground">Status Service Detail</h2>
        <span class="inline-flex items-center gap-1.5 rounded-md bg-secondary px-2 py-1 text-[10.5px] text-secondary-foreground">
          <Activity class="h-3 w-3" /> {{ servicesView.length }} services · refresh 8 dtk
        </span>
      </div>
      <div class="mt-3 overflow-hidden rounded-xl border border-border bg-card shadow-lawu-sm">
        <div class="overflow-x-auto">
          <table class="min-w-full divide-y divide-border text-left text-sm">
            <thead class="bg-secondary/60 text-[11px] uppercase tracking-wider text-muted-foreground">
              <tr>
                <th class="px-4 py-2.5 font-medium">Service</th>
                <th class="px-4 py-2.5 font-medium">Status</th>
                <th class="px-4 py-2.5 font-medium">PID</th>
                <th class="px-4 py-2.5 font-medium">Uptime</th>
                <th class="px-4 py-2.5 font-medium">Memory (MB)</th>
                <th class="px-4 py-2.5 font-medium text-right">Aksi</th>
              </tr>
            </thead>
            <tbody class="divide-y divide-border">
              <tr v-for="s in servicesView" :key="s.name" class="hover:bg-secondary/40">
                <td class="px-4 py-2.5">
                  <div class="flex items-center gap-2">
                    <Server class="h-3.5 w-3.5 text-muted-foreground" />
                    <span class="font-mono text-[12.5px] text-foreground">{{ s.name }}</span>
                  </div>
                  <p v-if="s.sub" class="mt-0.5 pl-5 text-[10.5px] text-muted-foreground">{{ s.sub }}</p>
                </td>
                <td class="px-4 py-2.5">
                  <span
                    class="inline-flex items-center gap-1 rounded-md px-2 py-0.5 text-[10.5px] font-semibold"
                    :class="s.active
                      ? 'bg-primary/10 text-primary'
                      : 'bg-accent/10 text-accent'"
                  >
                    <span class="inline-block h-1.5 w-1.5 rounded-full" :class="s.active ? 'bg-primary' : 'bg-accent'" />
                    {{ s.active ? 'Active' : 'Failed' }}
                  </span>
                </td>
                <td class="px-4 py-2.5 tabular-nums text-muted-foreground">{{ s.pid ?? '—' }}</td>
                <td class="px-4 py-2.5 tabular-nums text-muted-foreground">{{ fmtUptime(s.uptimeSec) }}</td>
                <td class="px-4 py-2.5 tabular-nums text-muted-foreground">{{ fmtMem(s.memoryMb) }}</td>
                <td class="px-4 py-2.5 text-right">
                  <button
                    type="button"
                    @click="restartService(s.name)"
                    class="inline-flex items-center gap-1 rounded-md border border-border bg-background px-2 py-1 text-[10.5px] font-medium text-foreground hover:bg-secondary"
                    :aria-label="`Restart service ${s.name}`"
                  >
                    <RotateCw class="h-3 w-3" /> Restart
                  </button>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
      <p class="mt-2 text-[10.5px] text-muted-foreground">
        Catatan: tombol Restart masih placeholder — endpoint <code class="rounded bg-muted px-1 py-0.5 font-mono">POST /api/monitor/service/:name/restart</code>
        akan hadir di backend v3.7.0.
      </p>
    </section>

    <!-- 4. Statistik Korpus (3 card horizontal) -->
    <section aria-label="Statistik korpus" class="mt-8">
      <h2 class="mb-3 text-sm font-bold text-foreground">Statistik Korpus</h2>
      <div class="grid grid-cols-1 gap-3 sm:grid-cols-3">
        <div class="rounded-xl border border-border bg-card p-4 shadow-lawu-sm">
          <p class="flex items-center gap-1.5 text-xs font-medium text-muted-foreground">
            <Database class="h-3.5 w-3.5" /> Total Dokumen
          </p>
          <p class="mt-1 text-2xl font-extrabold tracking-tight tabular-nums text-foreground">
            {{ totalDocs.toLocaleString('id-ID') }}
          </p>
          <p class="mt-0.5 text-[10.5px] text-muted-foreground">terindex di Tantivy</p>
        </div>
        <div class="rounded-xl border border-border bg-card p-4 shadow-lawu-sm">
          <p class="flex items-center gap-1.5 text-xs font-medium text-muted-foreground">
            <BookOpen class="h-3.5 w-3.5" /> Ebook Terindex
          </p>
          <p class="mt-1 text-2xl font-extrabold tracking-tight tabular-nums text-foreground">{{ totalEbooks }}</p>
          <p class="mt-0.5 text-[10.5px] text-muted-foreground">PDF · EPUB · DOCX · TXT</p>
        </div>
        <div class="rounded-xl border border-border bg-card p-4 shadow-lawu-sm">
          <p class="flex items-center gap-1.5 text-xs font-medium text-muted-foreground">
            <Globe class="h-3.5 w-3.5" /> Web Crawl
          </p>
          <p class="mt-1 text-2xl font-extrabold tracking-tight tabular-nums text-foreground">{{ totalWebCrawl }}</p>
          <p class="mt-0.5 text-[10.5px] text-muted-foreground">URL korpus Indonesia</p>
        </div>
      </div>
    </section>

    <!-- 5. Timeline Alert -->
    <section aria-label="Timeline alert" class="mt-8">
      <div class="flex items-center justify-between">
        <h2 class="text-sm font-bold text-foreground">Timeline Alert</h2>
        <button @click="loadAlerts" class="inline-flex items-center gap-1.5 rounded-md px-2 py-1 text-xs text-muted-foreground hover:bg-secondary">
          <RefreshCw class="h-3.5 w-3.5" /> Muat ulang
        </button>
      </div>
      <div class="mt-3 max-h-96 space-y-2 overflow-y-auto pr-1 lawu-scroll">
        <div v-if="alerts.length === 0" class="rounded-xl border border-dashed border-border bg-card p-8 text-center text-sm text-muted-foreground">
          Belum ada alert. Jalankan Uji Simulasi untuk membuat entri pertama.
        </div>
        <div v-for="a in alerts" :key="a.id" class="flex items-start gap-3 rounded-xl border border-border bg-card p-3.5 shadow-lawu-sm">
          <span class="mt-0.5 shrink-0 rounded-md px-2 py-0.5 text-xs font-bold" :class="formatLevel(a.level).class">
            {{ formatLevel(a.level).label }}
          </span>
          <div class="min-w-0 flex-1">
            <div class="flex flex-wrap items-center gap-2">
              <p class="text-sm font-semibold text-foreground">{{ a.title }}</p>
              <span class="inline-flex items-center gap-1 rounded-md bg-secondary px-1.5 py-0.5 text-[10px] text-secondary-foreground">
                <ShieldCheck class="h-3 w-3" /> {{ a.delivered ? 'Terkirim' : 'Simulasi' }}
              </span>
            </div>
            <p class="mt-1 text-xs leading-relaxed text-muted-foreground">{{ a.message }}</p>
            <p class="mt-1.5 font-mono text-[10.5px] text-muted-foreground">
              {{ a.source }} · {{ new Date(a.createdAt).toLocaleString('id-ID', { day: '2-digit', month: 'short', hour: '2-digit', minute: '2-digit' }) }}
            </p>
          </div>
        </div>
      </div>
    </section>

    <!-- 6. Telegram Alert Config -->
    <section aria-label="Telegram alert config" class="mt-8">
      <div class="rounded-xl border border-border bg-card p-4 shadow-lawu-sm sm:p-5">
        <div class="flex items-center justify-between">
          <h3 class="flex items-center gap-2 text-sm font-bold text-foreground">
            <MessageCircleWarning class="h-4 w-4 text-primary" /> Alert Telegram
          </h3>
          <span class="rounded-md bg-secondary px-2 py-0.5 text-xs font-medium text-secondary-foreground">Mode Simulasi</span>
        </div>
        <p class="mt-2 text-xs text-muted-foreground">
          Bot asli menyusul. Pesan sekarang dicatat ke timeline alert di atas.
        </p>
        <div class="mt-3 grid gap-3 sm:grid-cols-2">
          <div>
            <label class="text-xs text-muted-foreground" for="bot-token">Bot API Token</label>
            <input id="bot-token" v-model="tokenInput" type="password" placeholder="123456:ABC-DEF…" class="mt-1 h-10 w-full rounded-lg border border-border bg-background px-3 text-sm" />
          </div>
          <div>
            <label class="text-xs text-muted-foreground" for="chat-id">Chat ID</label>
            <input id="chat-id" v-model="chatIdInput" type="text" placeholder="mis. 123456789" class="mt-1 h-10 w-full rounded-lg border border-border bg-background px-3 text-sm" />
          </div>
        </div>
        <button
          type="button"
          @click="runTest"
          class="mt-3 flex w-full items-center justify-center gap-1.5 rounded-lg bg-primary px-3 py-2 text-sm font-semibold text-primary-foreground hover:bg-primary/90"
        >
          <Send class="h-3.5 w-3.5" /> Uji Simulasi
        </button>
        <pre v-if="lastPreview" class="mt-3 max-h-28 overflow-auto rounded-lg bg-foreground/95 p-2.5 font-mono text-[10.5px] leading-relaxed text-background lawu-scroll">{{ lastPreview }}</pre>
      </div>
    </section>

    <!-- Statistik dasar -->
    <p class="mt-8 text-center text-[10.5px] text-muted-foreground">
      Statistik tambahan: {{ totalChats }} obrolan · {{ totalAlertsCount }} alert ·
      Lawu Fine · AI Lokal Indonesia
    </p>
  </div>
</template>
