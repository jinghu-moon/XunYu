<script setup lang="ts">
import { ref } from 'vue'
import type { EnvScope } from '../types'

const props = defineProps<{
  entries: string[]
  scope: EnvScope
  loading?: boolean
}>()

const emit = defineEmits<{
  (e: 'scope-change', scope: EnvScope): void
  (e: 'refresh'): void
  (e: 'add', payload: { entry: string; head: boolean }): void
  (e: 'remove', entry: string): void
}>()

const entry = ref('')
const head = ref(false)

function addEntry() {
  const value = entry.value.trim()
  if (!value) return
  emit('add', { entry: value, head: head.value })
  entry.value = ''
}
</script>

<template>
  <section class="env-card">
    <header class="env-card__header">
      <h3>PATH Editor</h3>
      <div class="toolbar">
        <select :value="scope" @change="emit('scope-change', ($event.target as HTMLSelectElement).value as EnvScope)">
          <option value="user">User</option>
          <option value="system">System</option>
        </select>
        <button @click="emit('refresh')" :disabled="loading">Refresh</button>
      </div>
    </header>

    <div class="toolbar">
      <input v-model="entry" placeholder="C:/tools/bin" />
      <label class="checkbox">
        <input v-model="head" type="checkbox" />
        insert at head
      </label>
      <button @click="addEntry" :disabled="loading">Add</button>
    </div>

    <ul class="path-list">
      <li v-for="item in props.entries" :key="item">
        <code>{{ item }}</code>
        <button @click="emit('remove', item)" :disabled="loading">Remove</button>
      </li>
    </ul>
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

.path-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: grid;
  gap: var(--space-2);
}

.path-list li {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: var(--space-2);
  border: var(--border);
  border-radius: var(--radius-sm);
  padding: var(--space-2);
  min-width: 0;
}

.path-list code {
  font-family: var(--font-family-mono);
  flex: 1;
  min-width: 0;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
</style>
