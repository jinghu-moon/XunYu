<script setup lang="ts">
import { computed, ref } from 'vue'
import type { EnvSchema, EnvScope, EnvValidationReport } from '../types'

const props = defineProps<{
  schema: EnvSchema | null
  validation: EnvValidationReport | null
  scope: EnvScope
  loading?: boolean
}>()

const emit = defineEmits<{
  (e: 'refresh-schema'): void
  (e: 'add-required', payload: { pattern: string; warnOnly: boolean }): void
  (e: 'add-regex', payload: { pattern: string; regex: string; warnOnly: boolean }): void
  (e: 'add-enum', payload: { pattern: string; values: string[]; warnOnly: boolean }): void
  (e: 'remove-rule', pattern: string): void
  (e: 'reset-schema'): void
  (e: 'run-validate', payload: { scope: EnvScope; strict: boolean }): void
}>()

const ruleType = ref<'required' | 'regex' | 'enum'>('required')
const pattern = ref('')
const regex = ref('')
const enumValues = ref('')
const warnOnly = ref(false)
const strict = ref(false)

const schemaRules = computed(() => props.schema?.rules ?? [])

function onAddRule() {
  const p = pattern.value.trim()
  if (!p) return
  if (ruleType.value === 'required') {
    emit('add-required', { pattern: p, warnOnly: warnOnly.value })
    return
  }
  if (ruleType.value === 'regex') {
    const r = regex.value.trim()
    if (!r) return
    emit('add-regex', { pattern: p, regex: r, warnOnly: warnOnly.value })
    return
  }
  const values = enumValues.value
    .split(',')
    .map((s) => s.trim())
    .filter(Boolean)
  if (!values.length) return
  emit('add-enum', { pattern: p, values, warnOnly: warnOnly.value })
}

function onValidate() {
  emit('run-validate', { scope: props.scope, strict: strict.value })
}
</script>

<template>
  <section class="env-card">
    <header class="env-card__header">
      <h3>Schema & Validate</h3>
      <div class="actions">
        <button type="button" @click="emit('refresh-schema')" :disabled="loading">Refresh</button>
        <button type="button" @click="emit('reset-schema')" :disabled="loading">Reset</button>
      </div>
    </header>

    <div class="toolbar">
      <select v-model="ruleType">
        <option value="required">required</option>
        <option value="regex">regex</option>
        <option value="enum">enum</option>
      </select>
      <input v-model="pattern" placeholder="pattern, e.g. JAVA_*" />
      <input v-if="ruleType === 'regex'" v-model="regex" placeholder="regex expression" />
      <input v-if="ruleType === 'enum'" v-model="enumValues" placeholder="enum values, comma separated" />
      <label class="checkbox">
        <input v-model="warnOnly" type="checkbox" />
        warn only
      </label>
      <button type="button" @click="onAddRule" :disabled="loading">Add Rule</button>
    </div>

    <table v-if="schemaRules.length">
      <thead>
        <tr>
          <th>Pattern</th>
          <th>Required</th>
          <th>Regex</th>
          <th>Enum</th>
          <th>Warn</th>
          <th>Action</th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="rule in schemaRules" :key="rule.pattern">
          <td class="mono">{{ rule.pattern }}</td>
          <td>{{ rule.required ? 'yes' : 'no' }}</td>
          <td class="mono">{{ rule.regex || '-' }}</td>
          <td class="mono">{{ (rule.enum_values || []).join(', ') || '-' }}</td>
          <td>{{ rule.warn_only ? 'yes' : 'no' }}</td>
          <td>
            <button type="button" @click="emit('remove-rule', rule.pattern)" :disabled="loading">Remove</button>
          </td>
        </tr>
      </tbody>
    </table>
    <p v-else class="hint">No schema rules.</p>

    <div class="validate">
      <label class="checkbox">
        <input v-model="strict" type="checkbox" />
        strict
      </label>
      <button type="button" @click="onValidate" :disabled="loading">Run Validate</button>
      <span v-if="validation" class="summary">
        vars={{ validation.total_vars }} errors={{ validation.errors }} warnings={{ validation.warnings }}
      </span>
    </div>

    <table v-if="validation?.violations?.length">
      <thead>
        <tr>
          <th>Name</th>
          <th>Pattern</th>
          <th>Kind</th>
          <th>Severity</th>
          <th>Message</th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="(item, idx) in validation.violations" :key="`${item.pattern}:${idx}`">
          <td class="mono">{{ item.name || '-' }}</td>
          <td class="mono">{{ item.pattern }}</td>
          <td>{{ item.kind }}</td>
          <td>{{ item.severity }}</td>
          <td class="mono">{{ item.message }}</td>
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
}

.validate {
  margin: var(--space-3) 0;
  display: flex;
  align-items: center;
  gap: var(--space-2);
}

.checkbox {
  display: inline-flex;
  align-items: center;
  gap: var(--space-1);
  color: var(--text-secondary);
}

.summary {
  color: var(--text-secondary);
}

.hint {
  color: var(--text-secondary);
}

.mono {
  font-family: var(--font-family-mono);
}
</style>
