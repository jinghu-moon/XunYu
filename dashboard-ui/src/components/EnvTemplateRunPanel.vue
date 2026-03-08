<script setup lang="ts">
import { ref } from 'vue'
import type { EnvLiveExportFormat, EnvRunResult, EnvScope, EnvTemplateResult } from '../types'

const props = defineProps<{
  scope: EnvScope
  templateResult: EnvTemplateResult | null
  runResult: EnvRunResult | null
  loading?: boolean
}>()

const emit = defineEmits<{
  (e: 'template-expand', payload: { template: string; scope: EnvScope; validate_only: boolean }): void
  (e: 'export-live', payload: { scope: EnvScope; format: EnvLiveExportFormat }): void
  (e: 'run', payload: {
    cmd: string[]
    scope: EnvScope
    schema_check: boolean
    notify: boolean
    max_output: number
  }): void
}>()

const templateInput = ref('%PATH%')
const validateOnly = ref(false)
const exportFormat = ref<EnvLiveExportFormat>('dotenv')

const commandTokens = ref('')
const schemaCheck = ref(false)
const notify = ref(false)
const maxOutput = ref(65536)

function normalizedScope(): EnvScope {
  return props.scope === 'all' ? 'user' : props.scope
}

function onExpandTemplate() {
  const template = templateInput.value.trim()
  if (!template) return
  emit('template-expand', {
    template,
    scope: normalizedScope(),
    validate_only: validateOnly.value,
  })
}

function onRunCommand() {
  const cmd = commandTokens.value
    .split('\n')
    .map((v) => v.trim())
    .filter(Boolean)
  if (!cmd.length) return
  emit('run', {
    cmd,
    scope: normalizedScope(),
    schema_check: schemaCheck.value,
    notify: notify.value,
    max_output: Math.min(Math.max(maxOutput.value || 65536, 1024), 1024 * 1024),
  })
}
</script>

<template>
  <section class="env-card">
    <header class="env-card__header">
      <h3>Template / Export-Live / Run</h3>
    </header>

    <div class="toolbar">
      <input v-model="templateInput" placeholder="template text, e.g. Path=%PATH%" />
      <label class="checkbox">
        <input v-model="validateOnly" type="checkbox" />
        validate only
      </label>
      <button type="button" @click="onExpandTemplate" :disabled="loading">Expand</button>
    </div>

    <p v-if="templateResult" class="mono">
      valid={{ templateResult.report.valid }} refs={{ templateResult.report.references.length }}
      missing={{ templateResult.report.missing.length }} cycles={{ templateResult.report.cycles.length }}
    </p>
    <pre v-if="templateResult?.output" class="mono">{{ templateResult.output }}</pre>

    <div class="toolbar">
      <select v-model="exportFormat">
        <option value="dotenv">dotenv</option>
        <option value="sh">sh</option>
        <option value="json">json</option>
        <option value="reg">reg</option>
      </select>
      <button
        type="button"
        :disabled="loading"
        @click="emit('export-live', { scope: normalizedScope(), format: exportFormat })"
      >
        Export Live
      </button>
    </div>

    <label class="label">Command tokens (one token per line)</label>
    <textarea
      v-model="commandTokens"
      rows="6"
      class="mono"
      placeholder="Example:
node
--version"
    ></textarea>

    <div class="toolbar">
      <label class="checkbox">
        <input v-model="schemaCheck" type="checkbox" />
        schema check
      </label>
      <label class="checkbox">
        <input v-model="notify" type="checkbox" />
        notify
      </label>
      <label class="checkbox">
        max output
        <input v-model.number="maxOutput" type="number" min="1024" step="1024" />
      </label>
      <button type="button" @click="onRunCommand" :disabled="loading">Run</button>
    </div>

    <div v-if="runResult" class="result">
      <p class="mono">exit={{ runResult.exit_code ?? -1 }} success={{ runResult.success }} truncated={{ runResult.truncated }}</p>
      <div class="grid">
        <section>
          <h4>stdout</h4>
          <pre class="mono">{{ runResult.stdout || '(empty)' }}</pre>
        </section>
        <section>
          <h4>stderr</h4>
          <pre class="mono">{{ runResult.stderr || '(empty)' }}</pre>
        </section>
      </div>
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

.label {
  display: inline-block;
  margin-bottom: var(--space-2);
  color: var(--text-secondary);
}

textarea {
  width: 100%;
  border: var(--border);
  border-radius: var(--radius-sm);
  padding: var(--space-2);
  background: var(--surface-card);
  color: var(--text-primary);
}

.checkbox {
  display: inline-flex;
  align-items: center;
  gap: var(--space-1);
  color: var(--text-secondary);
}

.mono {
  font-family: var(--font-family-mono);
}

.result {
  margin-top: var(--space-3);
}

.grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: var(--space-3);
}

h4 {
  font: var(--type-title-xs);
  margin-bottom: var(--space-1);
}

pre {
  border: var(--border);
  border-radius: var(--radius-sm);
  padding: var(--space-2);
  background: var(--surface-card);
  color: var(--text-primary);
  white-space: pre-wrap;
  min-height: 120px;
}

@media (max-width: 960px) {
  .grid {
    grid-template-columns: 1fr;
  }
}
</style>
