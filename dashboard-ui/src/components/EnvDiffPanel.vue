<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import type { EnvDiffEntry, EnvDiffResult, EnvScope, EnvSnapshotMeta } from '../types'

const props = defineProps<{
  diff: EnvDiffResult | null
  snapshots: EnvSnapshotMeta[]
  snapshotId: string | null
  since: string | null
  scope: EnvScope
  loading?: boolean
}>()

const emit = defineEmits<{
  (e: 'refresh'): void
  (e: 'snapshot-change', snapshotId: string | null): void
  (e: 'since-change', since: string | null): void
}>()

const filter = ref<'all' | 'added' | 'removed' | 'changed'>('all')
const selected = ref(props.snapshotId ?? '')
const selectedSince = ref(props.since ?? '')

watch(
  () => props.snapshotId,
  (next) => {
    selected.value = next ?? ''
  },
)

watch(
  () => props.since,
  (next) => {
    selectedSince.value = next ?? ''
  },
)

const totalChanges = computed(() => {
  const d = props.diff
  if (!d) return 0
  return d.added.length + d.removed.length + d.changed.length
})

const filteredEntries = computed<EnvDiffEntry[]>(() => {
  const d = props.diff
  if (!d) return []
  if (filter.value === 'added') return d.added
  if (filter.value === 'removed') return d.removed
  if (filter.value === 'changed') return d.changed
  return [...d.added, ...d.removed, ...d.changed].sort((a, b) => a.name.localeCompare(b.name))
})

function onSnapshotSelect() {
  if (selected.value) {
    emit('since-change', null)
  }
  emit('snapshot-change', selected.value || null)
}

function onSinceApply() {
  const next = selectedSince.value.trim()
  if (next) {
    emit('snapshot-change', null)
    emit('since-change', next)
    return
  }
  emit('since-change', null)
}
</script>

<template>
  <section class="env-card">
    <header class="env-card__header">
      <h3>Diff Live</h3>
      <div class="toolbar">
        <select v-model="selected" :disabled="loading" @change="onSnapshotSelect">
          <option value="">baseline: latest snapshot</option>
          <option v-for="item in snapshots" :key="item.id" :value="item.id">
            {{ item.id }} · {{ item.description || '(no desc)' }}
          </option>
        </select>
        <input
          v-model="selectedSince"
          type="text"
          :disabled="loading"
          placeholder="since: 2026-03-01 / RFC3339"
          @keyup.enter="onSinceApply"
        />
        <button type="button" @click="onSinceApply" :disabled="loading">Apply Since</button>
        <button type="button" @click="emit('refresh')" :disabled="loading">Refresh</button>
      </div>
    </header>

    <p class="summary">
      scope={{ scope }} changes={{ totalChanges }}
      <span v-if="diff"> (+{{ diff.added.length }} / -{{ diff.removed.length }} / ~{{ diff.changed.length }}) </span>
    </p>

    <div class="filters">
      <button type="button" :class="{ active: filter === 'all' }" @click="filter = 'all'">All</button>
      <button type="button" :class="{ active: filter === 'added' }" @click="filter = 'added'">
        Added
      </button>
      <button type="button" :class="{ active: filter === 'removed' }" @click="filter = 'removed'">
        Removed
      </button>
      <button type="button" :class="{ active: filter === 'changed' }" @click="filter = 'changed'">
        Changed
      </button>
    </div>

    <p v-if="!diff" class="hint">No diff loaded.</p>
    <p v-else-if="!filteredEntries.length" class="hint">No entries for current filter.</p>

    <div v-else class="diff-list">
      <article v-for="item in filteredEntries" :key="`${item.kind}:${item.name}`" class="diff-row">
        <header class="diff-row__header">
          <span class="kind" :class="item.kind">{{ item.kind }}</span>
          <span class="name mono">{{ item.name }}</span>
        </header>
        <div class="values">
          <p v-if="item.old_value" class="mono old">old: {{ item.old_value }}</p>
          <p v-if="item.new_value" class="mono new">new: {{ item.new_value }}</p>
        </div>
        <ul v-if="item.path_diff?.length" class="path-diff mono">
          <li v-for="(seg, idx) in item.path_diff" :key="`${seg.segment}:${idx}`" :class="seg.kind">
            {{ seg.kind }}: {{ seg.segment }}
          </li>
        </ul>
      </article>
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
  margin-bottom: var(--space-2);
  gap: var(--space-2);
}

.env-card__header h3 {
  font: var(--type-title-sm);
}

.summary {
  color: var(--text-secondary);
  margin-bottom: var(--space-2);
}

.toolbar input {
  min-width: 240px;
}

.filters {
  display: inline-flex;
  gap: var(--space-1);
  margin-bottom: var(--space-3);
}

.filters button.active {
  background: var(--surface-card-muted);
}

.hint {
  color: var(--text-secondary);
}

.diff-list {
  display: grid;
  gap: var(--space-2);
}

.diff-row {
  border: var(--border);
  border-radius: var(--radius-sm);
  padding: var(--space-2);
  background: var(--surface-card);
}

.diff-row__header {
  display: flex;
  align-items: center;
  gap: var(--space-2);
}

.kind {
  display: inline-block;
  min-width: 64px;
  text-align: center;
  border: var(--border);
  border-radius: var(--radius-sm);
  padding: 2px var(--space-1);
  text-transform: uppercase;
  font: var(--type-caption);
}

.kind.added {
  color: var(--state-success);
}

.kind.removed {
  color: var(--state-error);
}

.kind.changed {
  color: var(--state-warning);
}

.name {
  font-weight: var(--weight-medium);
}

.values {
  margin-top: var(--space-1);
}

.old {
  color: var(--state-error);
}

.new {
  color: var(--state-success);
}

.path-diff {
  margin-top: var(--space-1);
  padding-left: var(--space-4);
}

.path-diff .added {
  color: var(--state-success);
}

.path-diff .removed {
  color: var(--state-error);
}

.mono {
  font-family: var(--font-family-mono);
}
</style>
