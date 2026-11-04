<script setup lang="ts">
import { computed } from 'vue'
import { useRoute } from 'vue-router'
import { Moon, Sun } from 'lucide-vue-next'

defineProps<{ theme: 'light' | 'dark' }>()
defineEmits<{ 'toggle-theme': [] }>()

const route = useRoute()

const viewTitle = computed(() => {
  switch (route.path) {
    case '/chat':
      return 'Konsol Chat'
    case '/search':
      return 'Pencarian Ebook'
    case '/monitor':
      return 'Monitoring & Alert'
    case '/guide':
      return 'Panduan Implementasi Server'
    case '/settings':
      return 'Pengaturan'
    default:
      return 'Lawu-2.5-Fine'
  }
})
</script>

<template>
  <header
    class="sticky top-0 z-40 flex h-14 items-center justify-between gap-2 border-b border-border bg-background/85 px-4 backdrop-blur"
  >
    <h1 class="truncate text-sm font-semibold tracking-tight sm:text-base text-foreground">
      {{ viewTitle }}
    </h1>
    <button
      type="button"
      :aria-label="theme === 'dark' ? 'Ganti ke mode terang' : 'Ganti ke mode gelap'"
      @click="$emit('toggle-theme')"
      class="rounded-md p-2 text-foreground transition-colors hover:bg-secondary"
    >
      <Sun v-if="theme === 'dark'" class="h-4.5 w-4.5" />
      <Moon v-else class="h-4.5 w-4.5" />
    </button>
  </header>
</template>
