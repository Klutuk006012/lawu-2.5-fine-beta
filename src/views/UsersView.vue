<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import {
  Users, Plus, Pencil, Trash2, X, Loader2, CheckCircle2, AlertCircle, ShieldCheck,
} from 'lucide-vue-next'
import { api } from '@/lib/api'
import { useAuthStore } from '@/stores/auth'

interface UserRow {
  id: number
  email: string
  role: 'admin' | 'guest'
  createdAt: string
  lastLoginAt: string | null
}

interface UsersResponse {
  users: UserRow[]
}

interface UserMutationResponse {
  user: UserRow
}

const auth = useAuthStore()
const users = ref<UserRow[]>([])
const loading = ref(false)
const toast = ref<{ msg: string; ok: boolean } | null>(null)

// Modal state
const modalOpen = ref(false)
const modalMode = ref<'create' | 'edit'>('create')
const editingId = ref<number | null>(null)
const formEmail = ref('')
const formPassword = ref('')
const formRole = ref<'admin' | 'guest'>('guest')
const formSubmitting = ref(false)
const formError = ref<string | null>(null)

const deleteTarget = ref<UserRow | null>(null)

function showToast(msg: string, ok = true) {
  toast.value = { msg, ok }
  setTimeout(() => { toast.value = null }, 5000)
}

async function load() {
  loading.value = true
  try {
    const res = await api.get<UsersResponse>('/api/users')
    users.value = res.data.users ?? []
  } catch (err: unknown) {
    const e = err as { response?: { data?: { error?: string } } }
    showToast(e?.response?.data?.error ?? 'Gagal memuat daftar user.', false)
  } finally {
    loading.value = false
  }
}

function openCreate() {
  modalMode.value = 'create'
  editingId.value = null
  formEmail.value = ''
  formPassword.value = ''
  formRole.value = 'guest'
  formError.value = null
  modalOpen.value = true
}

function openEdit(u: UserRow) {
  modalMode.value = 'edit'
  editingId.value = u.id
  formEmail.value = u.email
  formPassword.value = ''
  formRole.value = u.role
  formError.value = null
  modalOpen.value = true
}

function closeModal() {
  modalOpen.value = false
  formError.value = null
}

async function submitForm() {
  if (!formEmail.value.trim()) {
    formError.value = 'Email wajib diisi.'
    return
  }
  if (modalMode.value === 'create' && !formPassword.value) {
    formError.value = 'Password wajib diisi saat membuat user baru.'
    return
  }
  formSubmitting.value = true
  formError.value = null
  try {
    if (modalMode.value === 'create') {
      const res = await api.post<UserMutationResponse>('/api/users', {
        email: formEmail.value.trim(),
        password: formPassword.value,
        role: formRole.value,
      })
      users.value.push(res.data.user)
      showToast(`User ${res.data.user.email} ditambahkan.`)
    } else if (editingId.value !== null) {
      const body: Record<string, unknown> = {
        email: formEmail.value.trim(),
        role: formRole.value,
      }
      if (formPassword.value) body.password = formPassword.value
      const res = await api.patch<UserMutationResponse>(`/api/users/${editingId.value}`, body)
      const idx = users.value.findIndex((u) => u.id === editingId.value)
      if (idx >= 0) users.value[idx] = res.data.user
      showToast(`User ${res.data.user.email} diperbarui.`)
    }
    modalOpen.value = false
  } catch (err: unknown) {
    const e = err as { response?: { data?: { error?: string } } }
    formError.value = e?.response?.data?.error ?? 'Gagal menyimpan user.'
  } finally {
    formSubmitting.value = false
  }
}

async function confirmDelete() {
  if (!deleteTarget.value) return
  const target = deleteTarget.value
  try {
    await api.delete(`/api/users/${target.id}`)
    users.value = users.value.filter((u) => u.id !== target.id)
    showToast(`User ${target.email} dihapus.`)
  } catch (err: unknown) {
    const e = err as { response?: { data?: { error?: string } } }
    showToast(e?.response?.data?.error ?? 'Gagal menghapus user.', false)
  } finally {
    deleteTarget.value = null
  }
}

function fmtDate(s: string | null): string {
  if (!s) return '—'
  try {
    return new Date(s).toLocaleString('id-ID', { dateStyle: 'medium', timeStyle: 'short' })
  } catch {
    return s
  }
}

const canDelete = (u: UserRow) => auth.user?.id !== u.id

const totalUsers = computed(() => users.value.length)
const adminCount = computed(() => users.value.filter((u) => u.role === 'admin').length)

onMounted(load)
</script>

<template>
  <div class="mx-auto w-full max-w-5xl px-4 py-6">
    <!-- Header -->
    <div class="flex flex-wrap items-end justify-between gap-2">
      <div>
        <div class="flex items-center gap-2 text-[11px] font-semibold uppercase tracking-widest text-red-600 dark:text-red-400">
          <span class="inline-block h-2 w-2 rounded-sm bg-red-600 dark:bg-red-400" aria-hidden />
          Admin Only
        </div>
        <h1 class="mt-1 text-lg font-semibold tracking-tight text-foreground">Manajemen User</h1>
        <p class="mt-0.5 text-sm text-muted-foreground">Kelola akun yang dapat mengakses dashboard Lawu Fine.</p>
      </div>
      <button
        type="button"
        @click="openCreate"
        class="inline-flex h-10 items-center gap-2 rounded-lg bg-primary px-4 text-sm font-semibold text-primary-foreground shadow-sm hover:bg-primary/90"
      >
        <Plus class="h-4 w-4" /> Tambah User
      </button>
    </div>

    <!-- Stat cards -->
    <div class="mt-4 grid grid-cols-2 gap-3 sm:grid-cols-3">
      <div class="rounded-xl border border-border bg-card p-4 shadow-sm">
        <p class="flex items-center gap-1.5 text-xs font-medium text-muted-foreground"><Users class="h-3.5 w-3.5" /> Total User</p>
        <p class="mt-1 text-2xl font-bold tracking-tight text-foreground">{{ totalUsers }}</p>
      </div>
      <div class="rounded-xl border border-border bg-card p-4 shadow-sm">
        <p class="flex items-center gap-1.5 text-xs font-medium text-muted-foreground"><ShieldCheck class="h-3.5 w-3.5" /> Admin</p>
        <p class="mt-1 text-2xl font-bold tracking-tight text-foreground">{{ adminCount }}</p>
      </div>
      <div class="rounded-xl border border-border bg-card p-4 shadow-sm">
        <p class="flex items-center gap-1.5 text-xs font-medium text-muted-foreground"><Users class="h-3.5 w-3.5" /> Non-Admin</p>
        <p class="mt-1 text-2xl font-bold tracking-tight text-foreground">{{ totalUsers - adminCount }}</p>
      </div>
    </div>

    <!-- Toast -->
    <div
      v-if="toast"
      class="mt-4 flex items-start gap-2 rounded-lg border px-3 py-2 text-sm"
      :class="toast.ok
        ? 'border-emerald-500/30 bg-emerald-500/10 text-emerald-700 dark:text-emerald-400'
        : 'border-destructive/40 bg-destructive/10 text-destructive'"
    >
      <component :is="toast.ok ? CheckCircle2 : AlertCircle" class="mt-0.5 h-4 w-4 shrink-0" />
      <span>{{ toast.msg }}</span>
    </div>

    <!-- Table -->
    <div class="mt-4 overflow-hidden rounded-xl border border-border bg-card shadow-sm">
      <div v-if="loading" class="flex items-center gap-2 px-4 py-10 text-sm text-muted-foreground">
        <Loader2 class="h-4 w-4 animate-spin" /> Memuat daftar user…
      </div>
      <div v-else-if="users.length === 0" class="px-4 py-10 text-center text-sm text-muted-foreground">
        Belum ada user terdaftar. Klik "Tambah User" untuk membuat yang pertama.
      </div>
      <table v-else class="min-w-full divide-y divide-border text-left text-sm">
        <thead class="bg-secondary/60 text-[11px] uppercase tracking-wider text-muted-foreground">
          <tr>
            <th class="px-4 py-2.5 font-medium">ID</th>
            <th class="px-4 py-2.5 font-medium">Email</th>
            <th class="px-4 py-2.5 font-medium">Role</th>
            <th class="px-4 py-2.5 font-medium">Dibuat</th>
            <th class="px-4 py-2.5 font-medium">Login Terakhir</th>
            <th class="px-4 py-2.5 font-medium text-right">Aksi</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-border">
          <tr v-for="u in users" :key="u.id" class="hover:bg-secondary/40">
            <td class="px-4 py-2.5 tabular-nums text-muted-foreground">{{ u.id }}</td>
            <td class="px-4 py-2.5 font-medium text-foreground">{{ u.email }}</td>
            <td class="px-4 py-2.5">
              <span
                class="inline-flex items-center gap-1 rounded-md px-2 py-0.5 text-[10.5px] font-medium"
                :class="u.role === 'admin'
                  ? 'bg-emerald-500/10 text-emerald-700 dark:text-emerald-400'
                  : 'bg-secondary text-muted-foreground'"
              >
                {{ u.role }}
              </span>
            </td>
            <td class="px-4 py-2.5 text-muted-foreground">{{ fmtDate(u.createdAt) }}</td>
            <td class="px-4 py-2.5 text-muted-foreground">{{ fmtDate(u.lastLoginAt) }}</td>
            <td class="px-4 py-2.5">
              <div class="flex items-center justify-end gap-1">
                <button
                  type="button"
                  @click="openEdit(u)"
                  class="rounded-md p-1.5 text-muted-foreground hover:bg-secondary hover:text-foreground"
                  :aria-label="`Edit user ${u.email}`"
                >
                  <Pencil class="h-4 w-4" />
                </button>
                <button
                  type="button"
                  :disabled="!canDelete(u)"
                  @click="deleteTarget = u"
                  class="rounded-md p-1.5 text-muted-foreground hover:bg-destructive/10 hover:text-destructive disabled:cursor-not-allowed disabled:opacity-30"
                  :aria-label="`Hapus user ${u.email}`"
                  :title="canDelete(u) ? 'Hapus user' : 'Tidak bisa menghapus diri sendiri'"
                >
                  <Trash2 class="h-4 w-4" />
                </button>
              </div>
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- Create / Edit modal -->
    <div
      v-if="modalOpen"
      class="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-4"
      role="dialog"
      aria-modal="true"
      @click.self="closeModal"
    >
      <div class="w-full max-w-md rounded-2xl border border-border bg-card p-5 shadow-xl">
        <div class="flex items-center justify-between">
          <h3 class="text-sm font-semibold text-foreground">
            {{ modalMode === 'create' ? 'Tambah User Baru' : 'Edit User' }}
          </h3>
          <button type="button" @click="closeModal" class="rounded-md p-1.5 text-muted-foreground hover:bg-secondary" aria-label="Tutup">
            <X class="h-4 w-4" />
          </button>
        </div>
        <form class="mt-4 space-y-3" @submit.prevent="submitForm">
          <div>
            <label class="text-xs text-muted-foreground" for="u-email">Email</label>
            <input
              id="u-email"
              v-model="formEmail"
              type="email"
              required
              class="mt-1 h-10 w-full rounded-lg border border-border bg-background px-3 text-sm focus:border-primary focus:outline-none"
            />
          </div>
          <div>
            <label class="text-xs text-muted-foreground" for="u-pass">
              Password
              <span v-if="modalMode === 'edit'" class="text-[10px] text-muted-foreground/70">(kosongkan jika tidak diubah)</span>
            </label>
            <input
              id="u-pass"
              v-model="formPassword"
              type="password"
              :required="modalMode === 'create'"
              autocomplete="new-password"
              class="mt-1 h-10 w-full rounded-lg border border-border bg-background px-3 text-sm focus:border-primary focus:outline-none"
            />
          </div>
          <div>
            <label class="text-xs text-muted-foreground" for="u-role">Role</label>
            <select
              id="u-role"
              v-model="formRole"
              class="mt-1 h-10 w-full rounded-lg border border-border bg-background px-3 text-sm focus:border-primary focus:outline-none"
            >
              <option value="guest">guest</option>
              <option value="admin">admin</option>
            </select>
          </div>
          <div
            v-if="formError"
            class="rounded-lg border border-destructive/40 bg-destructive/10 px-3 py-2 text-xs text-destructive"
          >
            {{ formError }}
          </div>
          <div class="flex justify-end gap-2 pt-1">
            <button
              type="button"
              @click="closeModal"
              class="rounded-lg border border-border bg-background px-3 py-1.5 text-sm text-foreground hover:bg-secondary"
            >
              Batal
            </button>
            <button
              type="submit"
              :disabled="formSubmitting"
              class="flex items-center gap-1.5 rounded-lg bg-primary px-3 py-1.5 text-sm font-medium text-primary-foreground hover:bg-primary/90 disabled:opacity-50"
            >
              <Loader2 v-if="formSubmitting" class="h-4 w-4 animate-spin" />
              Simpan
            </button>
          </div>
        </form>
      </div>
    </div>

    <!-- Delete confirm modal -->
    <div
      v-if="deleteTarget"
      class="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-4"
      role="dialog"
      aria-modal="true"
      @click.self="deleteTarget = null"
    >
      <div class="w-full max-w-sm rounded-2xl border border-border bg-card p-5 shadow-xl">
        <h3 class="text-sm font-semibold text-foreground">Hapus user?</h3>
        <p class="mt-1 text-xs text-muted-foreground">
          User <span class="font-medium text-foreground">{{ deleteTarget.email }}</span> akan dihapus permanen.
        </p>
        <div class="mt-4 flex justify-end gap-2">
          <button
            type="button"
            @click="deleteTarget = null"
            class="rounded-lg border border-border bg-background px-3 py-1.5 text-sm text-foreground hover:bg-secondary"
          >
            Batal
          </button>
          <button
            type="button"
            @click="confirmDelete"
            class="rounded-lg bg-destructive px-3 py-1.5 text-sm font-medium text-destructive-foreground hover:bg-destructive/90"
          >
            Hapus
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
