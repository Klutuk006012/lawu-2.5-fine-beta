import { createRouter, createWebHistory, type RouteRecordRaw } from 'vue-router'
import { useAuthStore } from '@/stores/auth'

const routes: RouteRecordRaw[] = [
  { path: '/', redirect: '/chat' },
  {
    path: '/login',
    name: 'login',
    component: () => import('@/views/LoginView.vue'),
    meta: { guestOnly: true },
  },
  {
    path: '/chat',
    name: 'chat',
    component: () => import('@/views/ChatView.vue'),
    meta: { public: true },
  },
  {
    path: '/search',
    name: 'search',
    component: () => import('@/views/SearchView.vue'),
    meta: { requiresAuth: true },
  },
  {
    path: '/corpus',
    name: 'corpus',
    component: () => import('@/views/CorpusView.vue'),
    meta: { requiresAuth: true, requiresAdmin: true },
  },
  {
    path: '/ebooks',
    name: 'ebooks',
    component: () => import('@/views/EbooksView.vue'),
    meta: { requiresAuth: true, requiresAdmin: true },
  },
  {
    path: '/monitor',
    name: 'monitor',
    component: () => import('@/views/MonitorView.vue'),
    meta: { requiresAuth: true },
  },
  {
    path: '/training',
    name: 'training',
    component: () => import('@/views/TrainingView.vue'),
    meta: { requiresAuth: true },
  },
  {
    path: '/guide',
    name: 'guide',
    component: () => import('@/views/GuideView.vue'),
    meta: { public: true },
  },
  {
    path: '/settings',
    name: 'settings',
    component: () => import('@/views/SettingsView.vue'),
    meta: { requiresAuth: true },
  },
  {
    path: '/users',
    name: 'users',
    component: () => import('@/views/UsersView.vue'),
    meta: { requiresAuth: true, requiresAdmin: true },
  },
  {
    path: '/api-keys',
    name: 'api-keys',
    component: () => import('@/views/ApiKeysView.vue'),
    meta: { requiresAuth: true },
  },
  // 404 fallback
  {
    path: '/:pathMatch(.*)*',
    redirect: '/chat',
  },
]

const router = createRouter({
  history: createWebHistory(),
  routes,
  scrollBehavior(_to, _from, saved) {
    return saved ?? { top: 0 }
  },
})

router.beforeEach(async (to, _from, next) => {
  const auth = useAuthStore()
  // Jika ada token tapi belum load user, fetch dulu.
  if (auth.token && !auth.user) {
    try {
      await auth.fetchMe()
    } catch {
      // ignore — fetchMe sudah handle logout on error.
    }
  }
  if (to.meta.requiresAuth && !auth.isAuthenticated) {
    return next({ path: '/login', query: { next: to.fullPath } })
  }
  if (to.meta.requiresAdmin && !auth.isAdmin) {
    return next('/chat')
  }
  if (to.meta.guestOnly && auth.isAuthenticated) {
    return next('/monitor')
  }
  next()
})

export default router
