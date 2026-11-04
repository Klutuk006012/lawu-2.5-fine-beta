<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { RouterView } from 'vue-router'
import Sidebar from './components/Sidebar.vue'
import Header from './components/Header.vue'

const theme = ref<'light' | 'dark'>('light')

function toggleTheme() {
  theme.value = theme.value === 'dark' ? 'light' : 'dark'
  if (theme.value === 'dark') {
    document.documentElement.classList.add('dark')
  } else {
    document.documentElement.classList.remove('dark')
  }
  localStorage.setItem('lawu-theme', theme.value)
}

onMounted(() => {
  const saved = localStorage.getItem('lawu-theme') as 'light' | 'dark' | null
  if (saved) {
    theme.value = saved
    if (saved === 'dark') document.documentElement.classList.add('dark')
  }
})
</script>

<template>
  <div class="flex min-h-screen flex-col">
    <div class="flex min-h-0 flex-1">
      <!-- Sidebar desktop -->
      <Sidebar class="sticky top-0 hidden h-screen w-64 shrink-0 md:block" />

      <!-- Area utama -->
      <div class="flex min-h-0 min-w-0 flex-1 flex-col">
        <Header :theme="theme" @toggle-theme="toggleTheme" />
        <main class="min-h-0 flex-1">
          <RouterView />
        </main>
        <footer class="mt-auto flex flex-wrap items-center justify-between gap-2 border-t border-border px-4 py-2.5 text-[11px] text-muted-foreground">
          <p class="flex items-center gap-1.5">
            <span class="inline-block h-1.5 w-1.5 rounded-full bg-[#34a853]" aria-hidden />
            Lawu-2.5-Fine · Mesin Pencari Ebook
          </p>
          <p class="flex items-center gap-1.5">
            <span class="inline-block h-1.5 w-1.5 rounded-full bg-[#34a853]" aria-hidden />
            Layanan aktif
          </p>
        </footer>
      </div>
    </div>
  </div>
</template>
