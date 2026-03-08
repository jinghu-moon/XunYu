<script setup lang="ts">
import { computed } from 'vue'
import type { TaskProcessOutput } from '../types'
import type { TaskFormState, WorkspaceTaskDefinition } from '../workspace-tools'
import { buildFileGovernanceSummary } from './file-governance-summary'

const props = defineProps<{
  task: WorkspaceTaskDefinition
  form: TaskFormState
  phase: 'preview' | 'execute'
  process: TaskProcessOutput
}>()

const target = computed(() => props.task.target?.(props.form) ?? '')
const summary = computed(() => buildFileGovernanceSummary(props.task, props.form, props.phase, props.process, target.value))
</script>

<template>
  <section v-if="summary" :data-testid="`governance-summary-${props.phase}`" class="governance-summary">
    <header class="governance-summary__header">
      <h5 class="governance-summary__title">{{ summary.title }}</h5>
      <span class="governance-summary__badge">{{ props.phase === 'preview' ? '解释层' : '结果层' }}</span>
    </header>
    <p v-if="summary.note" class="governance-summary__note">{{ summary.note }}</p>
    <dl class="governance-summary__grid">
      <div v-for="item in summary.items" :key="item.label" class="governance-summary__item">
        <dt>{{ item.label }}</dt>
        <dd>{{ item.value }}</dd>
      </div>
    </dl>
  </section>
</template>

<style scoped>
.governance-summary {
  border: var(--border);
  border-radius: var(--radius-md);
  background: var(--surface-panel);
  padding: var(--space-3);
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
}

.governance-summary__header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: var(--space-3);
}

.governance-summary__title {
  font: var(--type-title-xs);
  color: var(--text-primary);
}

.governance-summary__badge {
  display: inline-flex;
  align-items: center;
  padding: 2px var(--space-3);
  border-radius: var(--radius-full);
  background: var(--ds-background-2);
  color: var(--text-secondary);
  font: var(--type-caption);
}

.governance-summary__note,
.governance-summary__item dt {
  color: var(--text-secondary);
  font: var(--type-body-sm);
}

.governance-summary__grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
  gap: var(--space-3);
}

.governance-summary__item {
  border: var(--border);
  border-radius: var(--radius-sm);
  background: var(--surface-card);
  padding: var(--space-3);
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
}

.governance-summary__item dd {
  margin: 0;
  color: var(--text-primary);
  font: var(--type-body-sm);
  word-break: break-word;
}
</style>
