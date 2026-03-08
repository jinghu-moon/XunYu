<script setup lang="ts">
import { ref } from 'vue'
import type { EnvScope } from '../types'

const props = defineProps<{
  scope: EnvScope
  loading?: boolean
}>()

const emit = defineEmits<{
  (e: 'export', payload: { scope: EnvScope; format: 'json' | 'env' | 'reg' | 'csv' }): void
  (e: 'export-all', payload: { scope: EnvScope }): void
  (e: 'import', payload: { scope: EnvScope; content: string; mode: 'merge' | 'overwrite'; dry_run: boolean }): void
}>()

const format = ref<'json' | 'env' | 'reg' | 'csv'>('json')
const mode = ref<'merge' | 'overwrite'>('merge')
const dryRun = ref(true)
const content = ref('')
const dragging = ref(false)

function runImport() {
  if (!content.value.trim()) return
  emit('import', {
    scope: props.scope === 'all' ? 'user' : props.scope,
    content: content.value,
    mode: mode.value,
    dry_run: dryRun.value,
  })
}

async function onDrop(e: DragEvent) {
  e.preventDefault()
  dragging.value = false
  const dt = e.dataTransfer
  if (!dt) return
  if (dt.files && dt.files.length > 0) {
    content.value = await dt.files[0].text()
    return
  }
  const text = dt.getData('text/plain')
  if (text.trim()) content.value = text
}
</script>

<template>
  <section class="env-card">
    <header class="env-card__header">
      <h3>Import / Export</h3>
    </header>

    <div class="toolbar">
      <select v-model="format">
        <option value="json">json</option>
        <option value="env">env</option>
        <option value="reg">reg</option>
        <option value="csv">csv</option>
      </select>
      <button @click="emit('export', { scope, format })" :disabled="loading">Export</button>
      <button @click="emit('export-all', { scope })" :disabled="loading">Export ZIP</button>
    </div>

    <div class="toolbar">
      <select v-model="mode">
        <option value="merge">merge</option>
        <option value="overwrite">overwrite</option>
      </select>
      <label class="checkbox">
        <input v-model="dryRun" type="checkbox" />
        dry run
      </label>
      <button @click="runImport" :disabled="loading">Import</button>
    </div>

    <div
      class="drop-zone"
      :class="{ 'drop-zone--active': dragging }"
      @dragover.prevent="dragging = true"
      @dragleave.prevent="dragging = false"
      @drop="onDrop"
    >
      <textarea v-model="content" rows="8" placeholder="Paste json/env/reg/csv content here"></textarea>
      <p class="drop-hint">Support paste or drag .env/.json/.reg/.csv file into textarea</p>
    </div>
  </section>
</template>

<style scoped>
.env-card {
  border: var(--border);
  border-radius: var(--radius-md);
  padding: var(--space-4);
  background: var(--surface-panel);
}

.env-card__header {
  margin-bottom: var(--space-3);
}

.env-card__header h3 {
  font: var(--type-title-sm);
}

textarea {
  width: 100%;
  border: var(--border);
  border-radius: var(--radius-sm);
  padding: var(--space-2);
  background: var(--surface-card);
  color: var(--text-primary);
  font-family: var(--font-family-mono);
}

.drop-zone {
  display: grid;
  gap: var(--space-2);
  padding: var(--space-2);
  border: 1px dashed transparent;
  border-radius: var(--radius-sm);
}

.drop-zone--active {
  border-color: var(--color-border-strong);
  background: var(--surface-card-muted);
}

.drop-hint {
  color: var(--text-secondary);
  font: var(--type-caption);
}

.checkbox {
  display: inline-flex;
  align-items: center;
  gap: var(--space-1);
}

button {
  padding: var(--comp-padding-sm);
  border: var(--border);
  border-radius: var(--radius-sm);
  background: var(--surface-card);
  color: var(--text-primary);
  cursor: pointer;
}
</style>
