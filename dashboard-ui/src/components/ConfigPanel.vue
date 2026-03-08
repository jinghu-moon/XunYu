<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { IconCheck, IconRefresh } from '@tabler/icons-vue'
import { fetchConfig, patchConfig } from '../api'
import type { GlobalConfig } from '../types'
import { notifyError } from '../ui/feedback'
import { Button } from './button'

const cfg = ref<GlobalConfig | null>(null)
const depthInput = ref('')
const excludeInput = ref('')
const busy = ref(false)

const currentDepth = computed(() => cfg.value?.tree?.defaultDepth ?? null)
const currentExclude = computed(() => cfg.value?.tree?.excludeNames ?? [])

function normalizeExclude(raw: string): string[] {
  return raw
    .split(',')
    .map(s => s.trim())
    .filter(Boolean)
}

function parseDepth(raw: string): number | null | 'invalid' {
  const trimmed = raw.trim()
  if (!trimmed) return null
  const value = Number(trimmed)
  if (!Number.isFinite(value) || value < 0 || !Number.isInteger(value)) return 'invalid'
  return value
}

function syncForm(data: GlobalConfig) {
  depthInput.value = data.tree?.defaultDepth != null ? String(data.tree.defaultDepth) : ''
  excludeInput.value = (data.tree?.excludeNames ?? []).join(', ')
}

async function load() {
  busy.value = true
  try {
    const data = await fetchConfig()
    cfg.value = data
    syncForm(data)
  } finally {
    busy.value = false
  }
}

async function onSave() {
  const depth = parseDepth(depthInput.value)
  if (depth === 'invalid') {
    notifyError('defaultDepth must be a non-negative integer', 'Config')
    return
  }

  const excludeNames = normalizeExclude(excludeInput.value)
  busy.value = true
  try {
    const data = await patchConfig({
      tree: {
        defaultDepth: depth,
        excludeNames,
      },
    })
    cfg.value = data
    syncForm(data)
  } finally {
    busy.value = false
  }
}

onMounted(load)
</script>

<template>
  <div>
    <div class="toolbar">
      <Button size="sm" preset="secondary" :disabled="busy" style="display:flex;align-items:center;gap:var(--space-1)" @click="load">
        <IconRefresh :size="16" /> Refresh
      </Button>
      <div style="flex:1" />
      <Button size="sm" preset="primary" :disabled="busy" style="display:flex;align-items:center;gap:var(--space-1)" @click="onSave">
        <IconCheck :size="16" /> Save config
      </Button>
    </div>

    <div class="config-summary">
      <div class="summary-item">
        <div class="label">Current defaultDepth</div>
        <div class="value">{{ currentDepth ?? '-' }}</div>
        <div class="summary-hint">Empty value means unlimited depth</div>
      </div>
      <div class="summary-item">
        <div class="label">Current excludeNames</div>
        <div class="value">{{ currentExclude.length ? currentExclude.join(', ') : '-' }}</div>
        <div class="summary-hint">{{ currentExclude.length }} entries</div>
      </div>
    </div>

    <div class="form">
      <div class="field">
        <div class="label">Default depth</div>
        <input v-model="depthInput" type="number" min="0" placeholder="e.g. 3 (empty to clear)" />
      </div>
      <div class="field">
        <div class="label">Exclude names (csv)</div>
        <input v-model="excludeInput" placeholder="node_modules, target, .git" />
      </div>
    </div>
  </div>
</template>

<style scoped>
.form {
  display: grid;
  grid-template-columns: 1fr 2fr;
  gap: var(--space-3);
  align-items: end;
}
.config-summary {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
  gap: var(--space-3);
  margin-bottom: var(--space-5);
}
.summary-item {
  border: var(--border);
  border-radius: var(--radius-md);
  padding: var(--space-3) var(--space-4);
  background: var(--ds-background-1);
}
.summary-item .label {
  font-size: var(--text-xs);
  color: var(--text-secondary);
  margin-bottom: var(--space-2);
}
.summary-item .value {
  font-size: var(--text-sm);
  color: var(--text-primary);
  word-break: break-all;
}
.summary-hint {
  margin-top: var(--space-2);
  font-size: var(--text-xs);
  color: var(--text-tertiary);
}
.field .label {
  font-size: var(--text-xs);
  color: var(--text-secondary);
  margin-bottom: var(--space-1);
}
</style>
