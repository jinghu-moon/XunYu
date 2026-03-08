<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { fetchConvertFile, fetchFileContent } from '../../api'

const props = defineProps<{
  path?: string
}>()

const filePath = ref('')
const toFormat = ref<'json' | 'json5' | 'yaml' | 'toml'>('json')
const loading = ref(false)
const writing = ref(false)
const originalLoading = ref(false)
const fromFormat = ref('')
const convertedText = ref('')
const originalLines = ref<string[]>([])
const error = ref('')
const writtenPath = ref('')

const canRun = computed(() => Boolean(filePath.value.trim()))
const resultLines = computed(() => {
  if (!convertedText.value) return []
  return convertedText.value.split('\n')
})

async function loadOriginal(path: string) {
  originalLoading.value = true
  try {
    const data = await fetchFileContent({ path, offset: 0, limit: 300 })
    originalLines.value = data.is_binary ? ['[binary file]'] : data.lines
  } catch {
    originalLines.value = []
  } finally {
    originalLoading.value = false
  }
}

async function previewConvert() {
  if (!canRun.value) return
  loading.value = true
  error.value = ''
  writtenPath.value = ''
  try {
    await loadOriginal(filePath.value.trim())
    const res = await fetchConvertFile({
      path: filePath.value.trim(),
      to_format: toFormat.value,
      preview: true,
    })
    fromFormat.value = res.from_format
    convertedText.value = res.content
  } catch (e: unknown) {
    convertedText.value = ''
    error.value = e instanceof Error ? e.message : String(e)
  } finally {
    loading.value = false
  }
}

async function writeConvert() {
  if (!canRun.value) return
  writing.value = true
  error.value = ''
  try {
    const res = await fetchConvertFile({
      path: filePath.value.trim(),
      to_format: toFormat.value,
      preview: false,
    })
    fromFormat.value = res.from_format
    convertedText.value = res.content
    writtenPath.value = res.written_path || ''
  } catch (e: unknown) {
    error.value = e instanceof Error ? e.message : String(e)
  } finally {
    writing.value = false
  }
}

watch(
  () => props.path,
  (path) => {
    if (!path) return
    filePath.value = path
    void previewConvert()
  },
  { immediate: true },
)
</script>

<template>
  <div class="cv">
    <div class="cv-header">
      <h4>Convert</h4>
      <span class="cv-hint">Cross-format config conversion</span>
    </div>

    <div class="cv-toolbar">
      <input
        v-model="filePath"
        class="cv-input"
        type="text"
        placeholder="Config file path..."
        @keydown.enter="previewConvert"
      />
      <select v-model="toFormat" class="cv-select">
        <option value="json">json</option>
        <option value="json5">json5</option>
        <option value="yaml">yaml</option>
        <option value="toml">toml</option>
      </select>
      <button class="cv-btn cv-btn--muted" :disabled="!canRun || loading" @click="previewConvert">
        {{ loading ? 'Previewing...' : 'Preview' }}
      </button>
      <button class="cv-btn cv-btn--accent" :disabled="!canRun || writing" @click="writeConvert">
        {{ writing ? 'Writing...' : 'Write' }}
      </button>
    </div>

    <div v-if="error" class="cv-error">{{ error }}</div>
    <div v-if="writtenPath" class="cv-success">Written: {{ writtenPath }}</div>

    <div class="cv-meta">
      <span>From: {{ fromFormat || '-' }}</span>
      <span>To: {{ toFormat }}</span>
      <span>Lines: {{ resultLines.length }}</span>
    </div>

    <div class="cv-panels">
      <div class="cv-panel">
        <div class="cv-panel-title">Original</div>
        <div class="cv-panel-body">
          <div v-if="originalLoading" class="cv-empty">Loading...</div>
          <pre v-else-if="originalLines.length">{{ originalLines.join('\n') }}</pre>
          <div v-else class="cv-empty">No preview</div>
        </div>
      </div>
      <div class="cv-panel">
        <div class="cv-panel-title">Converted</div>
        <div class="cv-panel-body">
          <div v-if="loading" class="cv-empty">Converting...</div>
          <pre v-else-if="convertedText">{{ convertedText }}</pre>
          <div v-else class="cv-empty">Run preview to view output</div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.cv {
  border: var(--border);
  border-radius: var(--radius-sm);
  background: var(--surface-panel);
  padding: var(--space-2);
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}

.cv-header {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: var(--space-2);
}

.cv-header h4 {
  font: var(--type-body-sm);
  color: var(--text-primary);
}

.cv-hint {
  color: var(--text-tertiary);
  font: var(--type-caption);
}

.cv-toolbar {
  display: flex;
  gap: var(--space-2);
  align-items: center;
  flex-wrap: wrap;
}

.cv-input {
  flex: 1;
  min-width: 220px;
  font-family: var(--font-family-mono);
  padding: var(--comp-padding-sm);
  border: var(--border);
  border-radius: var(--radius-sm);
  background: var(--surface-card-muted);
  color: var(--text-primary);
}

.cv-select {
  padding: var(--comp-padding-sm);
  border: var(--border);
  border-radius: var(--radius-sm);
  background: var(--surface-card-muted);
  color: var(--text-primary);
}

.cv-btn {
  padding: var(--comp-padding-sm);
  border-radius: var(--radius-sm);
  border: var(--border);
  cursor: pointer;
  font: var(--type-body-sm);
}

.cv-btn--muted {
  background: var(--surface-card-muted);
  color: var(--text-secondary);
}

.cv-btn--accent {
  background: var(--color-info-bg);
  color: var(--color-info);
}

.cv-btn:disabled {
  opacity: var(--disabled-opacity);
  cursor: not-allowed;
}

.cv-error {
  padding: var(--space-2);
  border-radius: var(--radius-sm);
  background: var(--color-danger-bg);
  color: var(--color-danger);
  font: var(--type-body-sm);
}

.cv-success {
  padding: var(--space-2);
  border-radius: var(--radius-sm);
  background: var(--color-success-bg);
  color: var(--color-success);
  font: var(--type-body-sm);
}

.cv-meta {
  display: flex;
  gap: var(--space-3);
  color: var(--text-secondary);
  font: var(--type-caption);
  font-family: var(--font-family-mono);
}

.cv-panels {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: var(--space-2);
}

.cv-panel {
  border: var(--border);
  border-radius: var(--radius-sm);
  overflow: hidden;
  background: var(--surface-card-muted);
}

.cv-panel-title {
  padding: var(--space-1) var(--space-2);
  border-bottom: var(--border);
  color: var(--text-secondary);
  font: var(--type-caption);
}

.cv-panel-body {
  max-height: 260px;
  overflow: auto;
  padding: var(--space-2);
}

.cv-panel-body pre {
  margin: 0;
  font: var(--type-body-sm);
  font-family: var(--font-family-mono);
  white-space: pre-wrap;
  word-break: break-word;
}

.cv-empty {
  color: var(--text-tertiary);
  font: var(--type-body-sm);
}

@media (max-width: 900px) {
  .cv-panels {
    grid-template-columns: 1fr;
  }
}
</style>
