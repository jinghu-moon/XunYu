import { reactive } from 'vue'

export type ToastLevel = 'error' | 'warning' | 'success' | 'info'

export type ToastItem = {
  id: number
  level: ToastLevel
  title: string
  detail?: string
  ts: number
}

const state = reactive({
  toasts: [] as ToastItem[],
  loadingCount: 0,
})

let seq = 1
const timers = new Map<number, number>()
const TOAST_MARK = Symbol('toast_mark')

export function useFeedbackState() {
  return state
}

export function beginLoading() {
  state.loadingCount += 1
}

export function endLoading() {
  state.loadingCount = Math.max(0, state.loadingCount - 1)
}

export function pushToast(opts: { level?: ToastLevel; title: string; detail?: string; ttlMs?: number }) {
  const id = seq++
  const item: ToastItem = {
    id,
    level: opts.level ?? 'info',
    title: opts.title,
    detail: opts.detail,
    ts: Date.now(),
  }
  state.toasts.unshift(item)
  if (state.toasts.length > 6) {
    state.toasts.length = 6
  }
  const ttl = opts.ttlMs ?? (item.level === 'error' ? 7000 : 4500)
  if (ttl > 0) {
    const timer = window.setTimeout(() => removeToast(id), ttl)
    timers.set(id, timer)
  }
  return id
}

export function removeToast(id: number) {
  const idx = state.toasts.findIndex(t => t.id === id)
  if (idx >= 0) {
    state.toasts.splice(idx, 1)
  }
  const timer = timers.get(id)
  if (timer != null) {
    clearTimeout(timer)
    timers.delete(id)
  }
}

export function isToastMarked(err: unknown): boolean {
  return !!(err && typeof err === 'object' && (err as any)[TOAST_MARK])
}

export function notifyError(err: unknown, context?: string) {
  if (isToastMarked(err)) return
  const detail = formatError(err)
  const title = context ? `Error: ${context}` : 'Request failed'
  pushToast({ level: 'error', title, detail })
  if (err && typeof err === 'object') {
    ;(err as any)[TOAST_MARK] = true
  }
}

function formatError(err: unknown): string {
  if (err instanceof Error) {
    return err.message || err.name
  }
  if (typeof err === 'string') return err
  try {
    return JSON.stringify(err)
  } catch {
    return String(err)
  }
}
