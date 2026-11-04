<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { Radio, RefreshCw, MessageCircleWarning, Send, ShieldCheck } from 'lucide-vue-next'
import { isBackendReachable, fetchWikiStats } from '../lib/lawu-backend'
import type { WikiStatsResponse } from '../lib/lawu-backend'

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
  metrics?: {
    cpuPct: number
    ramUsedGb: number
    ramTotalGb: number
    ssdUsedGb: number
    ssdTotalGb: number
    hddUsedGb: number
    hddTotalGb: number
    llamaTokPerSec: number
    loadAvg: number
  }
  docs?: {
    total: number
    byType: { fileType: string; count: number }[]
    byCategory: { category: string; count: number }[]
    sourceSizeMb: number
    indexSizeMb: number
    lastIndexedAt: string | null
  }
  chats?: { conversations: number; messages: number }
  alerts?: { total24h: number }
}

const stats = ref<StatsResponse | null>(null)
const alerts = ref<AlertItem[]>([])
const wikiStats = ref<WikiStatsResponse | null>(null)
const backendOnline = ref(false)
const tokenInput = ref('')
const chatIdInput = ref('')
const lastPreview = ref<string | null>(null)
let timer: ReturnType<typeof setInterval> | null = null

async function loadAll() {
  try {
    const [s, a] = await Promise.all([
      fetch('/api/stats').then((r) => r.json()),
      fetch('/api/alerts').then((r) => r.json()),
    ])
    stats.value = s
    alerts.value = Array.isArray(a) ? a : []
  } catch {
    // ignore
  }
  backendOnline.value = await isBackendReachable()
  wikiStats.value = await fetchWikiStats()
}

async function runTest() {
  try {
    const res = await fetch('/api/telegram/test', { method: 'POST' })
    const data = await res.json()
    if (data.payloadPreview?.text) lastPreview.value = data.payloadPreview.text
    await loadAll()
  } catch (err) {
    // silent
  }
}

function formatLevel(level: string) {
  const styles: Record<string, { label: string; class: string }> = {
    info: { label: 'Info', class: 'bg-[#e8f0fe] text-[#1a73e8]' },
    success: { label: 'Sukses', class: 'bg-[#e6f4ea] text-[#137333]' },
    warning: { label: 'Peringatan', class: 'bg-[#fef7e0] text-[#b06000]' },
    error: { label: 'Kritis', class: 'bg-[#fce8e6] text-[#c5221f]' },
  }
  return styles[level] ?? styles.info
}

onMounted(() => {
  loadAll()
  timer = setInterval(() => {
    fetch('/api/stats').then((r) => r.json()).then((s) => { stats.value = s }).catch(() => {})
  }, 8000)
})

onUnmounted(() => {
  if (timer) clearInterval(timer)
})
</script>

<template>
  <div class="mx-auto w-full max-w-4xl px-4 py-6">
    <!-- Metrik -->
    <section aria-label="Metrik sistem">
      <div class="flex items-center justify-between">
        <h2 class="text-lg font-semibold tracking-tight text-foreground">Metrik Sistem (Simulasi)</h2>
        <span class="inline-flex items-center gap-1 rounded-md bg-secondary px-2 py-1 text-xs">
          <Radio class="h-3 w-3 animate-pulse" />
          polling 8 dtk
        </span>
      </div>
      <p class="mt-1 text-sm text-muted-foreground">
        Pratinjau metrik yang akan disajikan saat layanan produksi aktif.
      </p>

      <div class="mt-3 grid grid-cols-1 gap-3 md:grid-cols-2">
        <div class="rounded-xl border border-border bg-card p-4 shadow-sm">
          <div v-if="stats?.metrics" class="space-y-4">
            <div>
              <div class="flex items-center justify-between text-xs">
                <span class="text-muted-foreground">Beban CPU</span>
                <span class="font-medium tabular-nums">{{ stats.metrics.cpuPct }} / 100 %</span>
              </div>
              <div class="mt-1.5 h-2 w-full overflow-hidden rounded-full bg-secondary">
                <div class="h-full bg-primary" :style="{ width: `${Math.min(100, stats.metrics.cpuPct)}%` }" />
              </div>
            </div>
            <div>
              <div class="flex items-center justify-between text-xs">
                <span class="text-muted-foreground">Memori Terpakai</span>
                <span class="font-medium tabular-nums">{{ stats.metrics.ramUsedGb }} / {{ stats.metrics.ramTotalGb }} GB</span>
              </div>
              <div class="mt-1.5 h-2 w-full overflow-hidden rounded-full bg-secondary">
                <div class="h-full bg-primary" :style="{ width: `${Math.min(100, (stats.metrics.ramUsedGb / stats.metrics.ramTotalGb) * 100)}%` }" />
              </div>
            </div>
            <div class="grid grid-cols-2 gap-2 pt-1">
              <div class="rounded-lg bg-secondary px-3 py-2">
                <p class="text-[11px] text-muted-foreground">Throughput Model</p>
                <p class="text-sm font-semibold tabular-nums">{{ stats.metrics.llamaTokPerSec }} tok/dtk</p>
              </div>
              <div class="rounded-lg bg-secondary px-3 py-2">
                <p class="text-[11px] text-muted-foreground">Load Average</p>
                <p class="text-sm font-semibold tabular-nums">{{ stats.metrics.loadAvg }}</p>
              </div>
            </div>
          </div>
          <div v-else class="space-y-3">
            <div class="h-4 w-full animate-pulse rounded bg-muted" />
            <div class="h-4 w-2/3 animate-pulse rounded bg-muted" />
            <div class="h-4 w-1/2 animate-pulse rounded bg-muted" />
          </div>
        </div>

        <!-- Panel Telegram -->
        <div class="rounded-xl border border-border bg-card p-4 shadow-sm">
          <div class="flex items-center justify-between">
            <h3 class="flex items-center gap-2 text-sm font-semibold text-foreground">
              <MessageCircleWarning class="h-4 w-4 text-primary" />
              Alert Telegram
            </h3>
            <span class="rounded-md bg-[#fef7e0] px-2 py-0.5 text-xs text-[#b06000]">Mode Simulasi</span>
          </div>
          <p class="mt-2 text-xs text-muted-foreground">
            Bot asli menyusul. Pesan sekarang dicatat ke timeline alert di bawah.
          </p>
          <div class="mt-3 space-y-3">
            <div>
              <label class="text-xs text-muted-foreground" for="bot-token">Bot API Token</label>
              <input id="bot-token" v-model="tokenInput" type="password" placeholder="123456:ABC-DEF…" class="mt-1 h-10 w-full rounded-lg border border-border bg-background px-3 text-sm" />
            </div>
            <div>
              <label class="text-xs text-muted-foreground" for="chat-id">Chat ID</label>
              <input id="chat-id" v-model="chatIdInput" type="text" placeholder="mis. 123456789" class="mt-1 h-10 w-full rounded-lg border border-border bg-background px-3 text-sm" />
            </div>
            <button @click="runTest" class="flex w-full items-center justify-center gap-1.5 rounded-lg bg-primary px-3 py-2 text-sm font-medium text-primary-foreground hover:bg-primary/90">
              <Send class="h-3.5 w-3.5" />
              Uji Simulasi
            </button>
            <pre v-if="lastPreview" class="max-h-28 overflow-auto rounded-lg bg-[#202124] p-2.5 font-mono text-[10.5px] leading-relaxed text-[#e8eaed]">{{ lastPreview }}</pre>
          </div>
        </div>
      </div>
    </section>

    <!-- Timeline alert -->
    <section aria-label="Timeline alert" class="mt-8">
      <div class="flex items-center justify-between">
        <h2 class="text-lg font-semibold tracking-tight text-foreground">Timeline Alert</h2>
        <button @click="loadAll" class="inline-flex items-center gap-1.5 rounded-md px-2 py-1 text-xs text-muted-foreground hover:bg-secondary">
          <RefreshCw class="h-3.5 w-3.5" />
          Muat ulang
        </button>
      </div>
      <div class="mt-3 max-h-96 space-y-2 overflow-y-auto pr-1">
        <div v-if="alerts.length === 0" class="rounded-xl border border-dashed border-border bg-card p-8 text-center text-sm text-muted-foreground">
          Belum ada alert. Jalankan Uji Simulasi untuk membuat entri pertama.
        </div>
        <div v-for="a in alerts" :key="a.id" class="flex items-start gap-3 rounded-xl border border-border bg-card p-3.5 shadow-sm">
          <span class="mt-0.5 shrink-0 rounded-md px-2 py-0.5 text-xs font-medium" :class="formatLevel(a.level).class">
            {{ formatLevel(a.level).label }}
          </span>
          <div class="min-w-0 flex-1">
            <div class="flex flex-wrap items-center gap-2">
              <p class="text-sm font-semibold text-foreground">{{ a.title }}</p>
              <span class="inline-flex items-center gap-1 rounded-md bg-secondary px-1.5 py-0.5 text-[10px]">
                <ShieldCheck class="h-3 w-3" />
                {{ a.delivered ? 'Terkirim' : 'Simulasi' }}
              </span>
            </div>
            <p class="mt-1 text-xs leading-relaxed text-muted-foreground">{{ a.message }}</p>
            <p class="mt-1.5 font-mono text-[10.5px] text-muted-foreground">{{ a.source }} · {{ new Date(a.createdAt).toLocaleString('id-ID', { day: '2-digit', month: 'short', hour: '2-digit', minute: '2-digit' }) }}</p>
          </div>
        </div>
      </div>
    </section>
  </div>
</template>
