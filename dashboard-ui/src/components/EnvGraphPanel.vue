<script setup lang="ts">
import { ref, watch } from 'vue'
import type { EnvDepTree, EnvScope } from '../types'

const props = defineProps<{
  scope: EnvScope
  tree: EnvDepTree | null
  loading?: boolean
}>()

const emit = defineEmits<{
  (e: 'run', payload: { scope: EnvScope; name: string; maxDepth: number }): void
}>()

const name = ref('PATH')
const maxDepth = ref(8)

watch(
  () => props.tree,
  (next) => {
    if (next?.root) {
      name.value = next.root
    }
  },
  { immediate: false },
)

function run() {
  const root = name.value.trim()
  if (!root) return
  emit('run', {
    scope: props.scope,
    name: root,
    maxDepth: Math.min(64, Math.max(1, Number(maxDepth.value) || 8)),
  })
}
</script>

<template>
  <section class="env-card">
    <header class="env-card__header">
      <h3>Dependency Graph</h3>
      <button :disabled="loading" @click="run">Run</button>
    </header>

    <div class="toolbar">
      <input v-model="name" placeholder="root variable, e.g. PATH" />
      <label>
        depth
        <input v-model.number="maxDepth" type="number" min="1" max="64" />
      </label>
    </div>

    <pre class="graph-output">{{ tree?.lines.join('\n') || 'No graph yet. Set root and click Run.' }}</pre>

    <p v-if="tree?.missing?.length" class="graph-meta">Missing: {{ tree.missing.join(', ') }}</p>
    <p v-if="tree?.cycles?.length" class="graph-meta">Cycles: {{ tree.cycles.join(' | ') }}</p>
  </section>
</template>

<style scoped>
.env-card {
  border: var(--border);
  border-radius: var(--radius-md);
  padding: var(--space-4);
  background: var(--surface-panel);
  display: grid;
  gap: var(--space-3);
}

.env-card__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.env-card__header h3 {
  font: var(--type-title-sm);
}

.toolbar {
  display: flex;
  align-items: center;
  gap: var(--space-2);
}

input {
  border: var(--border);
  border-radius: var(--radius-sm);
  padding: var(--comp-padding-sm);
  background: var(--surface-card);
  color: var(--text-primary);
}

label {
  display: inline-flex;
  align-items: center;
  gap: var(--space-1);
  color: var(--text-secondary);
}

.graph-output {
  margin: 0;
  border: var(--border);
  border-radius: var(--radius-sm);
  padding: var(--space-3);
  background: var(--surface-card);
  color: var(--text-primary);
  max-height: 260px;
  overflow: auto;
  font: var(--type-caption);
}

.graph-meta {
  margin: 0;
  color: var(--text-secondary);
  font: var(--type-caption);
}

button {
  padding: var(--comp-padding-sm);
  border: var(--border);
  border-radius: var(--radius-sm);
  background: var(--surface-card);
  color: var(--text-primary);
  cursor: pointer;
}

button:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}
</style>
