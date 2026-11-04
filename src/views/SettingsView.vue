<script setup lang="ts">
import { useThemeStore } from '../stores/theme'

const themeStore = useThemeStore()
</script>

<template>
  <div class="mx-auto w-full max-w-3xl px-4 py-6">
    <section class="rounded-xl border border-border bg-card p-5 shadow-sm">
      <h2 class="text-base font-semibold text-foreground">Endpoint Model</h2>
      <p class="mt-1.5 text-sm text-muted-foreground">
        Isi alamat endpoint model Anda (format /v1/chat/completions) bila sudah tersedia. Kosongkan untuk tetap memakai mode simulasi.
      </p>
      <div class="mt-4 space-y-3">
        <div>
          <label class="text-xs text-muted-foreground" for="llm-url">Base URL</label>
          <input id="llm-url" type="text" placeholder="http://<alamat-endpoint-model>:8080" class="mt-1 h-10 w-full rounded-lg border border-border bg-background px-3 text-sm" />
        </div>
        <div class="grid grid-cols-1 gap-3 sm:grid-cols-2">
          <div>
            <label class="text-xs text-muted-foreground" for="llm-model">Nama Model</label>
            <input id="llm-model" type="text" value="lawu-2.5-fine" class="mt-1 h-10 w-full rounded-lg border border-border bg-background px-3 text-sm" />
          </div>
          <div>
            <label class="text-xs text-muted-foreground" for="llm-key">API Key (opsional)</label>
            <input id="llm-key" type="password" placeholder="kosongkan bila tanpa key" class="mt-1 h-10 w-full rounded-lg border border-border bg-background px-3 text-sm" />
          </div>
        </div>
      </div>
    </section>

    <section class="mt-5 rounded-xl border border-border bg-card p-5 shadow-sm">
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
          <span class="absolute top-0.5 h-5 w-5 rounded-full bg-white shadow transition-transform" :class="themeStore.isDark ? 'translate-x-5' : 'translate-x-0.5'" />
        </button>
      </div>
    </section>
  </div>
</template>
