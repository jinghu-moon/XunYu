<script setup lang="ts">
import type { EnvAuditEntry } from '../types'

defineProps<{
  entries: EnvAuditEntry[]
  loading?: boolean
}>()

const emit = defineEmits<{
  (e: 'refresh'): void
}>()
</script>

<template>
  <section class="env-card">
    <header class="env-card__header">
      <h3>Audit</h3>
      <button type="button" @click="emit('refresh')" :disabled="loading">Refresh</button>
    </header>
    <p v-if="loading" class="hint">Loading...</p>
    <p v-else-if="!entries.length" class="hint">No audit entries.</p>
    <table v-else>
      <thead>
        <tr>
          <th>At</th>
          <th>Action</th>
          <th>Scope</th>
          <th>Result</th>
          <th>Name</th>
          <th>Message</th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="(item, idx) in entries.slice(0, 80)" :key="`${item.at}:${idx}`">
          <td class="mono">{{ item.at }}</td>
          <td>{{ item.action }}</td>
          <td>{{ item.scope }}</td>
          <td :class="item.result === 'ok' ? 'ok' : 'error'">{{ item.result }}</td>
          <td class="mono">{{ item.name || '-' }}</td>
          <td class="mono">{{ item.message || '-' }}</td>
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

.hint {
  color: var(--text-secondary);
}

.mono {
  font-family: var(--font-family-mono);
}

.ok {
  color: var(--state-success);
}

.error {
  color: var(--state-error);
}
</style>
