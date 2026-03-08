<script setup lang="ts">
import { ref } from 'vue'
import type { EnvScope, EnvSnapshotMeta } from '../types'

const props = defineProps<{
  snapshots: EnvSnapshotMeta[]
  scope: EnvScope
  loading?: boolean
}>()

const emit = defineEmits<{
  (e: 'refresh'): void
  (e: 'create', desc?: string): void
  (e: 'prune', payload: { keep: number }): void
  (e: 'restore', payload: { id: string; scope: EnvScope }): void
}>()

const desc = ref('')
const keep = ref(50)

function createSnapshot() {
  emit('create', desc.value.trim() || undefined)
  desc.value = ''
}

function restore(id: string) {
  if (!window.confirm(`Restore snapshot ${id}?`)) return
  emit('restore', { id, scope: props.scope })
}

function pruneSnapshots() {
  const value = Math.min(10000, Math.max(0, Number(keep.value) || 0))
  if (!window.confirm(`Prune snapshots and keep latest ${value}?`)) return
  emit('prune', { keep: value })
}
</script>

<template>
  <section class="env-card">
    <header class="env-card__header">
      <h3>Snapshots</h3>
      <button @click="emit('refresh')" :disabled="loading">Refresh</button>
    </header>

    <div class="toolbar">
      <input v-model="desc" placeholder="snapshot description" />
      <button @click="createSnapshot" :disabled="loading">Create</button>
    </div>

    <div class="toolbar">
      <label>
        keep latest
        <input v-model.number="keep" type="number" min="0" max="10000" />
      </label>
      <button @click="pruneSnapshots" :disabled="loading">Prune</button>
    </div>

    <div class="table-wrap">
      <table class="snapshots-table">
        <colgroup>
          <col class="col-id" />
          <col class="col-created" />
          <col class="col-desc" />
          <col class="col-action" />
        </colgroup>
        <thead>
          <tr>
            <th>ID</th>
            <th>Created</th>
            <th>Description</th>
            <th>Action</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="s in snapshots" :key="s.id">
            <td class="mono" :title="s.id">{{ s.id }}</td>
            <td class="created-cell" :title="s.created_at">{{ s.created_at }}</td>
            <td class="desc-cell" :title="s.description">{{ s.description }}</td>
            <td><button @click="restore(s.id)" :disabled="loading">Restore</button></td>
          </tr>
        </tbody>
      </table>
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
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: var(--space-3);
}

.env-card__header h3 {
  font: var(--type-title-sm);
}

.mono {
  font-family: var(--font-family-mono);
}

.table-wrap {
  max-width: 100%;
  overflow-x: auto;
}

.snapshots-table {
  width: 100%;
  min-width: 700px;
  table-layout: fixed;
}

.col-id {
  width: 122px;
}

.col-created {
  width: 184px;
}

.col-action {
  width: 86px;
}

.created-cell,
.desc-cell {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
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
