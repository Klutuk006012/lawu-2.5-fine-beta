import { createRouter, createWebHistory } from 'vue-router'

const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: '/', redirect: '/chat' },
    { path: '/chat', name: 'chat', component: () => import('../views/ChatView.vue') },
    { path: '/search', name: 'search', component: () => import('../views/SearchView.vue') },
    { path: '/corpus', name: 'corpus', component: () => import('../views/CorpusView.vue') },
    { path: '/ebooks', name: 'ebooks', component: () => import('../views/EbooksView.vue') },
    { path: '/monitor', name: 'monitor', component: () => import('../views/MonitorView.vue') },
    { path: '/guide', name: 'guide', component: () => import('../views/GuideView.vue') },
    { path: '/settings', name: 'settings', component: () => import('../views/SettingsView.vue') },
  ],
})

export default router
