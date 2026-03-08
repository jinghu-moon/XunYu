<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import type { EnvScope, EnvVar, EnvVarKind } from '../types'

const props = defineProps<{
  vars: EnvVar[]
  scope: EnvScope
  loading?: boolean
}>()

const emit = defineEmits<{
  (e: 'refresh'): void
  (e: 'scope-change', scope: EnvScope): void
  (e: 'set-var', payload: { name: string; value: string; noSnapshot: boolean }): void
  (e: 'delete-var', name: string): void
  (e: 'show-history', name: string): void
}>()

const search = ref('')
const name = ref('')
const value = ref('')
const noSnapshot = ref(false)

const filtered = computed(() => {
  const q = search.value.trim().toLowerCase()
  if (!q) return props.vars
  return props.vars.filter((v) => v.name.toLowerCase().includes(q) || v.raw_value.toLowerCase().includes(q))
})

watch(
  () => props.scope,
  () => {
    name.value = ''
    value.value = ''
  },
)

function selectVar(v: EnvVar) {
  name.value = v.name
  value.value = v.raw_value
}

function submitSet() {
  if (!name.value.trim()) return
  emit('set-var', {
    name: name.value.trim(),
    value: value.value,
    noSnapshot: noSnapshot.value,
  })
}

function deleteCurrent() {
  if (!name.value.trim()) return
  if (!window.confirm(`Delete ${name.value} ?`)) return
  emit('delete-var', name.value.trim())
}

const kindLabels: Record<EnvVarKind, string> = {
  url: 'URL',
  path: 'Path',
  path_list: 'PathList',
  boolean: 'Bool',
  secret: 'Secret',
  json: 'JSON',
  email: 'Email',
  version: 'Version',
  integer: 'Int',
  float: 'Float',
}

function kindLabel(kind?: EnvVarKind): string {
  if (!kind) return 'Unknown'
  return kindLabels[kind]
}
</script>

<template>
  <section class="env-card">
    <header class="env-card__header">
      <h3>Variables</h3>
      <div class="toolbar">
        <select :value="scope" @change="emit('scope-change', ($event.target as HTMLSelectElement).value as EnvScope)">
          <option value="user">User</option>
          <option value="system">System</option>
          <option value="all">All</option>
        </select>
        <button @click="emit('refresh')" :disabled="loading">Refresh</button>
      </div>
    </header>

    <div class="toolbar">
      <input v-model="search" placeholder="Search name/value" />
    </div>

    <div class="env-editor">
      <input v-model="name" placeholder="NAME" />
      <input v-model="value" placeholder="VALUE" />
      <label class="checkbox">
        <input v-model="noSnapshot" type="checkbox" />
        no snapshot
      </label>
      <button @click="submitSet" :disabled="scope === 'all' || loading">Set</button>
      <button @click="deleteCurrent" :disabled="scope === 'all' || loading">Delete</button>
    </div>

    <div class="table-wrap">
      <table class="vars-table">
        <colgroup>
          <col class="col-scope" />
          <col class="col-name" />
          <col class="col-type" />
          <col class="col-value" />
          <col class="col-actions" />
        </colgroup>
        <thead>
          <tr>
            <th>Scope</th>
            <th>Name</th>
            <th>Type</th>
            <th>Value</th>
            <th>Actions</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="v in filtered" :key="`${v.scope}:${v.name}`" @click="selectVar(v)">
            <td>{{ v.scope }}</td>
            <td class="name-cell" :title="v.name">{{ v.name }}</td>
            <td>
              <div class="type-cell">
                <span
                  v-if="v.inferred_kind"
                  class="type-chip"
                  :class="`type-chip--${v.inferred_kind}`"
                >
                  {{ kindLabel(v.inferred_kind) }}
                </span>
                <span v-else class="type-chip type-chip--unknown">Unknown</span>
                <span class="reg-type">reg:{{ v.reg_type }}</span>
              </div>
            </td>
            <td class="mono value-cell" :title="v.raw_value">{{ v.raw_value }}</td>
            <td>
              <button
                class="link-btn"
                type="button"
                @click.stop="emit('show-history', v.name)"
                :disabled="loading"
              >
                History
              </button>
            </td>
          </tr>
        </tbody>
      </table>
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
  margin-bottom: var(--space-3);
}

.env-card__header h3 {
  font: var(--type-title-sm);
}

.env-editor {
  display: grid;
  grid-template-columns: 1fr 2fr auto auto auto;
  gap: var(--space-2);
  margin-bottom: var(--space-3);
}

.env-editor > * {
  min-width: 0;
}

.checkbox {
  display: inline-flex;
  align-items: center;
  gap: var(--space-1);
  color: var(--text-secondary);
  font: var(--type-body-xs);
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

tbody tr {
  cursor: pointer;
}

tbody tr:hover {
  background: var(--surface-card-muted);
}

.mono {
  font-family: var(--font-family-mono);
}

.table-wrap {
  max-width: 100%;
  overflow-x: auto;
}

.vars-table {
  width: 100%;
  min-width: 920px;
  table-layout: fixed;
}

.col-scope {
  width: 74px;
}

.col-name {
  width: 150px;
}

.col-type {
  width: 58px;
}

.col-actions {
  width: 86px;
}

.name-cell,
.value-cell {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.link-btn {
  padding: 2px 8px;
  font: var(--type-body-xs);
}

.type-cell {
  display: inline-flex;
  align-items: center;
  gap: var(--space-1);
}

.type-chip {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  padding: 0 6px;
  border-radius: var(--radius-full);
  border: var(--border);
  font: var(--type-caption);
  line-height: 1.2;
  white-space: nowrap;
}

.type-chip--unknown {
  color: var(--text-secondary);
  background: var(--surface-card-muted);
}

.type-chip--secret {
  color: var(--color-danger);
  background: var(--color-danger-bg);
  border-color: var(--color-danger);
}

.type-chip--url,
.type-chip--email {
  color: var(--color-info);
  background: var(--color-info-bg);
  border-color: var(--color-info);
}

.type-chip--path,
.type-chip--path_list {
  color: var(--color-success);
  background: var(--color-success-bg);
  border-color: var(--color-success);
}

.type-chip--json,
.type-chip--version,
.type-chip--integer,
.type-chip--float,
.type-chip--boolean {
  color: var(--color-warning);
  background: var(--color-warning-bg);
  border-color: var(--color-warning);
}

.reg-type {
  color: var(--text-tertiary);
  font: var(--type-caption);
}

@media (max-width: 880px) {
  .env-editor {
    grid-template-columns: 1fr;
  }

  .vars-table {
    min-width: 760px;
  }
}
</style>
