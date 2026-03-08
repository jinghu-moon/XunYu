<script setup lang="ts">
import { computed } from 'vue'
import type { DiffHunk } from '../../types'
import DiffViewer from './DiffViewer.vue'

const props = defineProps<{
  hunks: DiffHunk[]
  viewMode: 'unified' | 'split'
}>()

const changedHunks = computed(() => props.hunks.filter((h) => h.kind !== 'unchanged'))
const displayHunks = computed(() => (changedHunks.value.length ? changedHunks.value : props.hunks))
const symbolSummary = computed(() => {
  const counter = new Map<string, number>()
  for (const hunk of changedHunks.value) {
    const key = hunk.symbol_type || hunk.symbol || 'symbol'
    counter.set(key, (counter.get(key) || 0) + 1)
  }
  return Array.from(counter.entries()).sort((a, b) => b[1] - a[1])
})
</script>

<template>
  <div class="cd">
    <div v-if="symbolSummary.length" class="cd-summary">
      <span v-for="item in symbolSummary.slice(0, 12)" :key="item[0]" class="cd-chip">
        {{ item[0] }} · {{ item[1] }}
      </span>
    </div>

    <div v-if="changedHunks.length" class="cd-symbols">
      <table class="cd-table">
        <thead>
          <tr>
            <th>Type</th>
            <th>Symbol</th>
            <th>Change</th>
            <th>Range</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="(h, idx) in changedHunks.slice(0, 120)" :key="idx">
            <td>{{ h.symbol_type || '-' }}</td>
            <td class="cd-symbol">{{ h.symbol || h.section || '-' }}</td>
            <td>
              <span class="cd-kind" :class="`cd-kind--${h.kind}`">{{ h.kind }}</span>
            </td>
            <td class="cd-range">-{{ h.old_start }} +{{ h.new_start }}</td>
          </tr>
        </tbody>
      </table>
    </div>

    <DiffViewer :hunks="displayHunks" :view-mode="viewMode" kind="ast" />
  </div>
</template>

<style scoped>
.cd {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}

.cd-summary {
  display: inline-flex;
  flex-wrap: wrap;
  gap: var(--space-1);
}

.cd-chip {
  border: var(--border);
  border-radius: var(--radius-full);
  background: var(--surface-card-muted);
  color: var(--text-secondary);
  font: var(--type-caption);
  padding: 0 var(--space-2);
}

.cd-symbols {
  border: var(--border);
  border-radius: var(--radius-sm);
  max-height: 220px;
  overflow: auto;
}

.cd-table {
  width: 100%;
  border-collapse: collapse;
}

.cd-table th,
.cd-table td {
  border-bottom: var(--border);
  padding: var(--space-1) var(--space-2);
  font: var(--type-body-sm);
}

.cd-symbol {
  font-family: var(--font-family-mono);
}

.cd-range {
  font-family: var(--font-family-mono);
  color: var(--text-secondary);
}

.cd-kind {
  display: inline-flex;
  border-radius: var(--radius-full);
  border: var(--border);
  font: var(--type-caption);
  padding: 0 var(--space-1);
  text-transform: lowercase;
}

.cd-kind--added {
  color: var(--color-success);
  background: var(--color-success-bg);
}

.cd-kind--removed {
  color: var(--color-danger);
  background: var(--color-danger-bg);
}

.cd-kind--modified {
  color: var(--color-warning);
  background: var(--color-warning-bg);
}
</style>
