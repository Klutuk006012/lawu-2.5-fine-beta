<script setup lang="ts">
import { ref } from 'vue'
import { Server, BookOpen, ExternalLink } from 'lucide-vue-next'

interface Step {
  num: string
  title: string
  desc: string
  status: 'done' | 'pending' | 'in-progress'
}

const steps = ref<Step[]>([
  { num: '0', title: 'Bersihkan Server', desc: 'Audit → stop service lama → backup → hapus → verify', status: 'done' },
  { num: '1', title: 'Prerequisites', desc: 'apt update + install build deps, nginx, python venv', status: 'done' },
  { num: '2', title: 'User lawu & Direktori', desc: 'useradd lawu (no sudo), /opt/lawu, /mnt/data, symlink', status: 'done' },
  { num: '3', title: 'Mount & Format HDD', desc: 'parted GPT → mkfs.ext4 → fstab automount (nofail)', status: 'done' },
  { num: '4', title: 'Arsitektur', desc: 'Diagram end-to-end alur data', status: 'done' },
  { num: '5', title: 'Kernel Tuning', desc: 'sysctl (vm.swappiness=20 utk RAM 16GB) + limits.conf', status: 'done' },
  { num: '6', title: 'Install Rust', desc: 'rustup stable, RUSTFLAGS=-C target-cpu=native', status: 'done' },
  { num: '7', title: 'Project Cargo', desc: 'cargo new lawu-server + Cargo.toml (axum 0.8, tantivy 0.24)', status: 'done' },
  { num: '8', title: 'Tantivy Schema', desc: 'schema.rs multi-field BM25', status: 'done' },
  { num: '9', title: 'Axum API', desc: '/health, /api/search, /api/stats', status: 'done' },
  { num: '10', title: 'Build llama.cpp', desc: 'clone b12069+, cmake STATIC linking, -j 6', status: 'done' },
  { num: '11', title: 'Model GGUF', desc: 'Q4_K_M (4.4GB) ke HDD', status: 'done' },
  { num: '12', title: 'Korpus Wikipedia-ID', desc: '1.38jt artikel terindex, query < 1ms', status: 'done' },
  { num: '13', title: 'Serve + systemd', desc: 'llama-server -t 6 -c 4096, systemd auto-start', status: 'done' },
  { num: '14', title: 'Otomasi', desc: 'watch_ebooks.sh, backup, cron', status: 'pending' },
  { num: '15', title: 'Load Testing', desc: 'k6: p95 /api/search < 300ms dengan 20 VUs', status: 'pending' },
  { num: '16', title: 'Nginx Reverse Proxy', desc: 'port 80 → Axum :3001 + llama :8080', status: 'pending' },
  { num: '17', title: 'Integrasi Dashboard', desc: 'Vue.js 3 (Vite + Pinia) deploy ke srv-fine', status: 'in-progress' },
  { num: '18', title: 'Hardening', desc: 'UFW + SSH key-only + fail2ban (TERAKHIR!)', status: 'pending' },
  { num: '19', title: 'Go-Live Checklist', desc: 'Audit final sebelum publik', status: 'pending' },
])

const statusBadge = {
  done: { label: '✓ Selesai', class: 'bg-[#e6f4ea] text-[#137333]' },
  pending: { label: '⏳ Tertunda', class: 'bg-secondary text-muted-foreground' },
  'in-progress': { label: '▶ Berjalan', class: 'bg-[#e8f0fe] text-[#1a73e8]' },
}
</script>

<template>
  <div class="mx-auto w-full max-w-3xl px-4 py-6">
    <section class="rounded-xl border border-border bg-card p-5 shadow-sm">
      <h2 class="flex items-center gap-2 text-lg font-semibold text-foreground">
        <Server class="h-5 w-5 text-primary" />
        Panduan Implementasi Server
      </h2>
      <p class="mt-2 text-sm text-muted-foreground">
        Lawu-2.5-Fine — 20 langkah setup dari nol hingga go-live. Backend Rust Axum + Tantivy + llama.cpp di Debian 13, murni CPU (i5-10400, 16GB RAM).
      </p>

      <div class="mt-4 grid grid-cols-3 gap-3">
        <div class="rounded-lg bg-[#e6f4ea] px-3 py-2 text-center">
          <p class="text-2xl font-bold text-[#137333]">{{ steps.filter(s => s.status === 'done').length }}</p>
          <p class="text-[11px] text-muted-foreground">Selesai</p>
        </div>
        <div class="rounded-lg bg-[#e8f0fe] px-3 py-2 text-center">
          <p class="text-2xl font-bold text-[#1a73e8]">{{ steps.filter(s => s.status === 'in-progress').length }}</p>
          <p class="text-[11px] text-muted-foreground">Berjalan</p>
        </div>
        <div class="rounded-lg bg-secondary px-3 py-2 text-center">
          <p class="text-2xl font-bold text-muted-foreground">{{ steps.filter(s => s.status === 'pending').length }}</p>
          <p class="text-[11px] text-muted-foreground">Tertunda</p>
        </div>
      </div>
    </section>

    <section class="mt-5">
      <h3 class="flex items-center gap-2 text-sm font-semibold text-foreground">
        <BookOpen class="h-4 w-4 text-primary" />
        Daftar Langkah
      </h3>
      <ol class="mt-3 space-y-2">
        <li
          v-for="step in steps"
          :key="step.num"
          class="flex items-start gap-3 rounded-xl border border-border bg-card p-3.5 shadow-sm"
        >
          <span class="flex h-7 w-7 shrink-0 items-center justify-center rounded-full bg-primary text-xs font-semibold text-primary-foreground">
            {{ step.num }}
          </span>
          <div class="min-w-0 flex-1">
            <div class="flex items-center justify-between gap-2">
              <p class="text-sm font-semibold text-foreground">{{ step.title }}</p>
              <span class="shrink-0 rounded-md px-2 py-0.5 text-[10px]" :class="statusBadge[step.status].class">
                {{ statusBadge[step.status].label }}
              </span>
            </div>
            <p class="mt-0.5 text-xs text-muted-foreground">{{ step.desc }}</p>
          </div>
        </li>
      </ol>
    </section>

    <section class="mt-5 rounded-xl border border-border bg-card p-5 shadow-sm">
      <h3 class="text-sm font-semibold text-foreground">Dokumen Referensi</h3>
      <ul class="mt-2 space-y-1.5 text-sm">
        <li>
          <a href="#" class="inline-flex items-center gap-1.5 text-primary hover:underline">
            <ExternalLink class="h-3.5 w-3.5" />
            PRD v3.4.0 — Spesifikasi lengkap proyek
          </a>
        </li>
        <li>
          <a href="#" class="inline-flex items-center gap-1.5 text-primary hover:underline">
            <ExternalLink class="h-3.5 w-3.5" />
            Incident Log — Pelajaran dari setup
          </a>
        </li>
      </ul>
    </section>
  </div>
</template>
