<script setup lang="ts">
import { ref } from 'vue'
import type { EnvProfileMeta, EnvScope } from '../types'

const props = defineProps<{
  profiles: EnvProfileMeta[]
  scope: EnvScope
  loading?: boolean
}>()

const emit = defineEmits<{
  (e: 'refresh'): void
  (e: 'capture', payload: { name: string; scope: EnvScope }): void
  (e: 'apply', payload: { name: string; scope: EnvScope }): void
  (e: 'delete', name: string): void
  (e: 'diff', payload: { name: string; scope: EnvScope }): void
}>()

const profileName = ref('')

function onCapture() {
  const name = profileName.value.trim()
  if (!name) return
  const scope: EnvScope = props.scope === 'all' ? 'user' : props.scope
  emit('capture', { name, scope })
}

function onApply(name: string) {
  const scope: EnvScope = props.scope === 'all' ? 'user' : props.scope
  emit('apply', { name, scope })
}

function onDiff(name: string) {
  emit('diff', { name, scope: props.scope })
}
</script>

<template>
  <section class="env-card">
    <header class="env-card__header">
      <h3>Profiles</h3>
      <button type="button" @click="emit('refresh')" :disabled="loading">Refresh</button>
    </header>

    <div class="toolbar">
      <input v-model="profileName" placeholder="profile name" />
      <button type="button" @click="onCapture" :disabled="loading">Capture</button>
    </div>

    <p v-if="!profiles.length" class="hint">No profiles.</p>
    <table v-else>
      <thead>
        <tr>
          <th>Name</th>
          <th>Scope</th>
          <th>Vars</th>
          <th>Created</th>
          <th>Actions</th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="item in profiles" :key="item.name">
          <td class="mono">{{ item.name }}</td>
          <td>{{ item.scope }}</td>
          <td>{{ item.var_count }}</td>
          <td>{{ item.created_at }}</td>
          <td class="actions">
            <button type="button" @click="onApply(item.name)" :disabled="loading">Apply</button>
            <button type="button" @click="onDiff(item.name)" :disabled="loading">Diff</button>
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
  flex-wrap: wrap;
}

.hint {
  color: var(--text-secondary);
}

.mono {
  font-family: var(--font-family-mono);
}
</style>
