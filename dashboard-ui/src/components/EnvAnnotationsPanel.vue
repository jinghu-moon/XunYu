<script setup lang="ts">
import { ref } from 'vue'
import type { EnvAnnotationEntry } from '../types'

defineProps<{
  entries: EnvAnnotationEntry[]
  loading?: boolean
}>()

const emit = defineEmits<{
  (e: 'refresh'): void
  (e: 'set', payload: { name: string; note: string }): void
  (e: 'delete', name: string): void
}>()

const name = ref('')
const note = ref('')

function onSet() {
  const varName = name.value.trim()
  const text = note.value.trim()
  if (!varName || !text) return
  emit('set', { name: varName, note: text })
}

function onFill(item: EnvAnnotationEntry) {
  name.value = item.name
  note.value = item.note
}
</script>

<template>
  <section class="env-card">
    <header class="env-card__header">
      <h3>Annotations</h3>
      <button type="button" @click="emit('refresh')" :disabled="loading">Refresh</button>
    </header>

    <div class="toolbar">
      <input v-model="name" placeholder="variable name, e.g. JAVA_HOME" />
      <input v-model="note" placeholder="annotation note" />
      <button type="button" @click="onSet" :disabled="loading">Save</button>
    </div>

    <p v-if="!entries.length" class="hint">No annotations.</p>
    <table v-else>
      <thead>
        <tr>
          <th>Name</th>
          <th>Note</th>
          <th>Actions</th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="item in entries" :key="item.name">
          <td class="mono">{{ item.name }}</td>
          <td class="mono">{{ item.note }}</td>
          <td class="actions">
            <button type="button" @click="onFill(item)" :disabled="loading">Edit</button>
            <button type="button" @click="emit('delete', item.name)" :disabled="loading">Delete</button>
          </td>
        </tr>
      </tbody>
    </table>
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

.actions {
  display: inline-flex;
  gap: var(--space-1);
}

.hint {
  color: var(--text-secondary);
}

.mono {
  font-family: var(--font-family-mono);
}
</style>
