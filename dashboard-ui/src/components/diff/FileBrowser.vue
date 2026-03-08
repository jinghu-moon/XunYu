<script setup lang="ts">
import { ref, watch } from 'vue'
import { fetchFiles } from '../../api'
import type { FileEntry } from '../../types'

const props = defineProps<{
  initialPath?: string
}>()

const emit = defineEmits<{
  select: [path: string]
  close: []
}>()

const currentPath = ref(props.initialPath || 'C:\\')
const entries = ref<FileEntry[]>([])
const loading = ref(false)
const error = ref('')

async function loadDir(path: string) {
  loading.value = true
  error.value = ''
  try {
    entries.value = await fetchFiles(path)
    currentPath.value = path
  } catch (e: any) {
    error.value = e?.message || 'Failed to load directory'
    entries.value = []
  } finally {
    loading.value = false
  }
}

function goUp() {
  const sep = currentPath.value.includes('/') ? '/' : '\\'
  const parts = currentPath.value.replace(/[\\/]+$/, '').split(/[\\/]/)
  if (parts.length <= 1) return
  parts.pop()
  let parent = parts.join(sep)
  // Windows root: e.g. "C:" → "C:\\"
  if (/^[A-Za-z]:$/.test(parent)) parent += '\\'
  loadDir(parent)
}

function onClick(entry: FileEntry) {
  if (entry.is_dir) {
    const sep = currentPath.value.includes('/') ? '/' : '\\'
    const next = currentPath.value.replace(/[\\/]+$/, '') + sep + entry.name
    loadDir(next)
  } else {
    const sep = currentPath.value.includes('/') ? '/' : '\\'
    const full = currentPath.value.replace(/[\\/]+$/, '') + sep + entry.name
    emit('select', full)
  }
}

function formatSize(size?: number): string {
  if (size == null) return ''
  if (size < 1024) return `${size} B`
  if (size < 1024 * 1024) return `${(size / 1024).toFixed(1)} KB`
  return `${(size / (1024 * 1024)).toFixed(1)} MB`
}

function onBackdropClick(e: MouseEvent) {
  if ((e.target as HTMLElement)?.classList.contains('fb-backdrop')) {
    emit('close')
  }
}

watch(() => props.initialPath, (v) => {
  if (v) loadDir(v)
}, { immediate: false })

loadDir(currentPath.value)
</script>

<template>
  <div class="fb-backdrop" @click="onBackdropClick">
    <div class="fb-modal">
      <header class="fb-header">
        <button class="fb-btn fb-btn-up" @click="goUp" title="Parent directory">&uarr;</button>
        <span class="fb-path">{{ currentPath }}</span>
        <button class="fb-btn fb-btn-close" @click="$emit('close')">&times;</button>
      </header>

      <div class="fb-body">
        <div v-if="loading" class="fb-status">Loading...</div>
        <div v-else-if="error" class="fb-status fb-error">{{ error }}</div>
        <div v-else-if="entries.length === 0" class="fb-status">Empty directory</div>
        <ul v-else class="fb-list">
          <li
            v-for="entry in entries"
            :key="entry.name"
            class="fb-item"
            :class="{ 'fb-item--dir': entry.is_dir }"
            @click="onClick(entry)"
          >
            <span class="fb-icon">{{ entry.is_dir ? '📁' : '📄' }}</span>
            <span class="fb-name">{{ entry.name }}</span>
            <span v-if="!entry.is_dir" class="fb-size">{{ formatSize(entry.size) }}</span>
          </li>
        </ul>
      </div>
    </div>
  </div>
</template>

<style scoped>
.fb-backdrop {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.5);
  z-index: var(--z-modal);
  display: flex;
  align-items: center;
  justify-content: center;
}

.fb-modal {
  background: var(--surface-panel);
  border: var(--border);
  border-radius: var(--radius-lg);
  box-shadow: var(--shadow-md);
  width: min(560px, 90vw);
  max-height: 70vh;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.fb-header {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-3) var(--space-4);
  border-bottom: var(--border);
  flex-shrink: 0;
}

.fb-path {
  flex: 1;
  font: var(--type-body-sm);
  color: var(--text-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-family: var(--font-family-mono);
}

.fb-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: var(--height-sm);
  height: var(--height-sm);
  border: var(--border);
  border-radius: var(--radius-sm);
  background: var(--surface-card-muted);
  color: var(--text-primary);
  cursor: pointer;
  font-size: var(--text-md);
  line-height: 1;
  transition: background var(--duration-fast) ease;
}

.fb-btn:hover {
  background: var(--ds-color-3);
}

.fb-body {
  flex: 1;
  overflow-y: auto;
  min-height: 200px;
}

.fb-status {
  padding: var(--space-6);
  text-align: center;
  font: var(--type-body-sm);
  color: var(--text-secondary);
}

.fb-error {
  color: var(--color-danger);
}

.fb-list {
  list-style: none;
  margin: 0;
  padding: 0;
}

.fb-item {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-2) var(--space-4);
  cursor: pointer;
  transition: background var(--duration-fast) ease;
  font: var(--type-body-sm);
}

.fb-item:hover {
  background: var(--gray-alpha-100);
}

.fb-item--dir {
  font-weight: var(--weight-medium);
}

.fb-icon {
  flex-shrink: 0;
  width: 1.25rem;
  text-align: center;
}

.fb-name {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.fb-size {
  flex-shrink: 0;
  color: var(--text-tertiary);
  font: var(--type-body-sm);
  font-family: var(--font-family-mono);
  font-size: var(--text-xs);
}
</style>
