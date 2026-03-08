<script setup lang="ts">
import type { EnvDoctorFixResult, EnvDoctorReport, EnvScope } from '../types'

defineProps<{
  scope: EnvScope
  report: EnvDoctorReport | null
  fixResult: EnvDoctorFixResult | null
  loading?: boolean
}>()

const emit = defineEmits<{
  (e: 'run'): void
  (e: 'fix'): void
}>()
</script>

<template>
  <section class="env-card">
    <header class="env-card__header">
      <h3>Doctor</h3>
      <div class="toolbar">
        <button @click="emit('run')" :disabled="loading">Run</button>
        <button @click="emit('fix')" :disabled="loading">Fix</button>
      </div>
    </header>

    <div class="doctor-summary" v-if="report">
      <span>Issues: {{ report.issues.length }}</span>
      <span>Errors: {{ report.errors }}</span>
      <span>Warnings: {{ report.warnings }}</span>
      <span>Fixable: {{ report.fixable }}</span>
    </div>
    <div class="doctor-summary" v-if="fixResult">
      <span>Fixed: {{ fixResult.fixed }}</span>
    </div>

    <ul class="issue-list" v-if="report?.issues.length">
      <li v-for="issue in report.issues" :key="`${issue.scope}-${issue.name}-${issue.message}`">
        <span class="severity">{{ issue.severity }}</span>
        <span>{{ issue.message }}</span>
      </li>
    </ul>
    <p v-else class="muted">No doctor report loaded.</p>
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

.doctor-summary {
  display: flex;
  gap: var(--space-4);
  color: var(--text-secondary);
  margin-bottom: var(--space-2);
}

.issue-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: grid;
  gap: var(--space-2);
}

.issue-list li {
  display: grid;
  grid-template-columns: 80px 1fr;
  gap: var(--space-2);
  border: var(--border);
  border-radius: var(--radius-sm);
  padding: var(--space-2);
}

.severity {
  text-transform: uppercase;
  color: var(--color-warning);
  font: var(--type-caption);
}

.muted {
  color: var(--text-secondary);
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
