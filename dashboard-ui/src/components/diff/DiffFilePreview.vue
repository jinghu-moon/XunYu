<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, watch } from 'vue'
import { fetchFileContent, fetchFileInfo, fetchValidateFile } from '../../api'
import type { DiffFileContent, DiffFileInfo, ValidateFileResponse } from '../../types'

const props = withDefaults(defineProps<{
  path: string
  refreshKey?: number
}>(), {
  refreshKey: 0,
})

const PREVIEW_LIMIT = 120
const VALIDATABLE_EXTS = new Set(['toml', 'yaml', 'yml', 'json', 'json5'])

const info = ref<DiffFileInfo | null>(null)
const content = ref<DiffFileContent | null>(null)
const validation = ref<ValidateFileResponse | null>(null)
const infoLoading = ref(false)
const contentLoading = ref(false)
const validationLoading = ref(false)
const validationError = ref('')
const showValidationPanel = ref(false)
const error = ref('')
const requestSeq = ref(0)
const contentViewportRef = ref<HTMLElement | null>(null)
const highlightedLine = ref<number | null>(null)
const lineHighlightMs = 2200
let highlightTimer: ReturnType<typeof window.setTimeout> | null = null

const activePath = computed(() => props.path.trim())
const canValidate = computed(() => {
  const ext = activePath.value.split('.').pop()?.toLowerCase() || ''
  return VALIDATABLE_EXTS.has(ext)
})
const invalidValidation = computed(() => {
  if (!validation.value) return null
  if (validation.value.valid) return null
  return validation.value
})
const canLoadMore = computed(() => {
  if (!content.value || contentLoading.value) return false
  if (content.value.is_binary) return false
  return content.value.truncated
})

function formatSize(size?: number): string {
  if (size == null) return '-'
  if (size < 1024) return `${size} B`
  if (size < 1024 * 1024) return `${(size / 1024).toFixed(1)} KB`
  return `${(size / (1024 * 1024)).toFixed(2)} MB`
}

function formatTimestamp(ts?: number | null): string {
  if (!ts) return '-'
  const d = new Date(ts * 1000)
  if (Number.isNaN(d.getTime())) return '-'
  return d.toLocaleString()
}

function resetState() {
  clearHighlightTimer()
  info.value = null
  content.value = null
  infoLoading.value = false
  contentLoading.value = false
  validationLoading.value = false
  validation.value = null
  validationError.value = ''
  showValidationPanel.value = false
  error.value = ''
  highlightedLine.value = null
}

function clearHighlightTimer() {
  if (highlightTimer == null) return
  window.clearTimeout(highlightTimer)
  highlightTimer = null
}

function setLineHighlight(line: number) {
  highlightedLine.value = line
  clearHighlightTimer()
  highlightTimer = window.setTimeout(() => {
    if (highlightedLine.value === line) {
      highlightedLine.value = null
    }
    highlightTimer = null
  }, lineHighlightMs)
}

async function loadInfo(path: string, seq: number) {
  infoLoading.value = true
  try {
    const data = await fetchFileInfo(path)
    if (seq !== requestSeq.value) return
    info.value = data
  } finally {
    if (seq === requestSeq.value) infoLoading.value = false
  }
}

async function loadContent(path: string, seq: number, offset = 0, append = false) {
  contentLoading.value = true
  try {
    const data = await fetchFileContent({ path, offset, limit: PREVIEW_LIMIT })
    if (seq !== requestSeq.value) return
    if (append && content.value && !content.value.is_binary && !data.is_binary) {
      content.value = {
        ...data,
        offset: content.value.offset,
        lines: content.value.lines.concat(data.lines),
      }
    } else {
      content.value = data
    }
  } finally {
    if (seq === requestSeq.value) contentLoading.value = false
  }
}

async function loadValidation(path: string, seq: number) {
  if (!canValidate.value) {
    if (seq === requestSeq.value) {
      validation.value = null
      validationError.value = ''
      validationLoading.value = false
      showValidationPanel.value = false
    }
    return
  }

  validationLoading.value = true
  validationError.value = ''
  try {
    const data = await fetchValidateFile({ path })
    if (seq !== requestSeq.value) return
    validation.value = data
    if (data.valid) {
      showValidationPanel.value = false
    }
  } catch (e: unknown) {
    if (seq !== requestSeq.value) return
    validation.value = null
    validationError.value = e instanceof Error ? e.message : String(e)
  } finally {
    if (seq === requestSeq.value) validationLoading.value = false
  }
}

async function refresh() {
  const path = activePath.value
  if (!path) {
    resetState()
    return
  }
  requestSeq.value += 1
  const seq = requestSeq.value
  error.value = ''
  content.value = null
  const results = await Promise.allSettled([
    loadInfo(path, seq),
    loadContent(path, seq),
    loadValidation(path, seq),
  ])
  if (seq !== requestSeq.value) return
  const rejected = results.find((r) => r.status === 'rejected')
  if (rejected && rejected.status === 'rejected') {
    error.value = rejected.reason instanceof Error ? rejected.reason.message : String(rejected.reason)
  }
}

async function loadMore() {
  if (!content.value || !canLoadMore.value) return
  const path = activePath.value
  if (!path) return
  const seq = requestSeq.value
  const nextOffset = content.value.offset + content.value.lines.length
  try {
    await loadContent(path, seq, nextOffset, true)
  } catch (e: unknown) {
    if (seq !== requestSeq.value) return
    error.value = e instanceof Error ? e.message : String(e)
  }
}

async function jumpToValidationLine(line?: number) {
  if (!line || line < 1) return
  const path = activePath.value
  if (!path || !content.value || content.value.is_binary) return
  if (line > content.value.total_lines) return

  const seq = requestSeq.value
  const currentStartLine = content.value.offset + 1
  const currentEndLine = content.value.offset + content.value.lines.length

  if (line < currentStartLine || line > currentEndLine) {
    const halfWindow = Math.floor(PREVIEW_LIMIT / 2)
    const maxOffset = Math.max(0, content.value.total_lines - PREVIEW_LIMIT)
    const targetOffset = Math.min(Math.max(0, line - 1 - halfWindow), maxOffset)
    try {
      await loadContent(path, seq, targetOffset, false)
    } catch (e: unknown) {
      if (seq === requestSeq.value) {
        error.value = e instanceof Error ? e.message : String(e)
      }
      return
    }
    if (seq !== requestSeq.value || !content.value || content.value.is_binary) return
  }

  setLineHighlight(line)
  await nextTick()
  const row = contentViewportRef.value?.querySelector<HTMLElement>(`tr[data-line="${line}"]`)
  row?.scrollIntoView({ block: 'center', behavior: 'smooth' })
}

watch(
  () => [activePath.value, props.refreshKey],
  async () => {
    try {
      await refresh()
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : String(e)
      infoLoading.value = false
      contentLoading.value = false
      validationLoading.value = false
    }
  },
  { immediate: true },
)

onBeforeUnmount(() => {
  clearHighlightTimer()
})
</script>

<template>
  <div class="fp">
    <div class="fp-header">
      <div class="fp-title-wrap">
        <h4 class="fp-title">Preview</h4>
        <span class="fp-path">{{ activePath || '-' }}</span>
      </div>
      <div class="fp-actions">
        <span v-if="canValidate" class="fp-validate" :class="{
          'fp-validate--ok': validation && validation.valid,
          'fp-validate--bad': validation && !validation.valid,
        }">
          <template v-if="validationLoading">Validating...</template>
          <template v-else-if="validation && validation.valid">Valid</template>
          <template v-else-if="validation && !validation.valid">{{ validation.errors.length }} error(s)</template>
          <template v-else>Not validated</template>
        </span>
        <button class="fp-btn" :disabled="!activePath || infoLoading || contentLoading" @click="refresh">
          Refresh
        </button>
      </div>
    </div>

    <div v-if="!activePath" class="fp-placeholder">Select a file to preview content and metadata.</div>
    <template v-else>
      <div v-if="error" class="fp-error">{{ error }}</div>
      <div v-if="validationError" class="fp-error">{{ validationError }}</div>

      <div class="fp-meta">
        <div class="fp-meta-item">
          <span class="fp-k">Size</span>
          <span class="fp-v">{{ infoLoading && !info ? 'Loading...' : formatSize(info?.size) }}</span>
        </div>
        <div class="fp-meta-item">
          <span class="fp-k">Lines</span>
          <span class="fp-v">{{ infoLoading && !info ? 'Loading...' : (info?.line_count ?? '-') }}</span>
        </div>
        <div class="fp-meta-item">
          <span class="fp-k">Language</span>
          <span class="fp-v">{{ infoLoading && !info ? 'Loading...' : (info?.language || '-') }}</span>
        </div>
        <div class="fp-meta-item">
          <span class="fp-k">Class</span>
          <span class="fp-v">{{ infoLoading && !info ? 'Loading...' : (info?.file_class || '-') }}</span>
        </div>
        <div class="fp-meta-item fp-meta-item--wide">
          <span class="fp-k">Modified</span>
          <span class="fp-v">{{ infoLoading && !info ? 'Loading...' : formatTimestamp(info?.modified) }}</span>
        </div>
        <div v-if="invalidValidation" class="fp-meta-item fp-meta-item--wide">
          <button class="fp-validation-btn" @click="showValidationPanel = !showValidationPanel">
            {{ showValidationPanel ? 'Hide validation details' : 'Show validation details' }}
          </button>
        </div>
      </div>

      <div v-if="invalidValidation && showValidationPanel" class="fp-validation-panel">
        <ul>
          <li v-for="(item, idx) in invalidValidation.errors" :key="idx">
            <button
              type="button"
              class="fp-validation-item"
              :class="{ 'fp-validation-item--disabled': !item.line }"
              :disabled="!item.line"
              @click="jumpToValidationLine(item.line)"
            >
              <span class="fp-validation-loc" v-if="item.line">L{{ item.line }}<template v-if="item.col">:{{ item.col }}</template></span>
              <span>{{ item.message }}</span>
            </button>
          </li>
        </ul>
      </div>

      <div class="fp-content" ref="contentViewportRef">
        <div v-if="contentLoading && !content" class="fp-placeholder">Loading content...</div>
        <div v-else-if="content?.is_binary" class="fp-placeholder">Binary file, preview is unavailable.</div>
        <div v-else-if="content && content.lines.length === 0" class="fp-placeholder">Empty file.</div>
        <table v-else-if="content" class="fp-table">
          <tbody>
            <tr
              v-for="(line, idx) in content.lines"
              :key="content.offset + idx"
              :data-line="content.offset + idx + 1"
              :class="{ 'fp-row--highlight': highlightedLine === content.offset + idx + 1 }"
            >
              <td class="fp-ln">{{ content.offset + idx + 1 }}</td>
              <td class="fp-text"><pre>{{ line }}</pre></td>
            </tr>
          </tbody>
        </table>
      </div>

      <div class="fp-footer">
        <span class="fp-progress" v-if="content && !content.is_binary">
          {{ content.lines.length }} / {{ content.total_lines }} lines
        </span>
        <button class="fp-btn" :disabled="!canLoadMore" @click="loadMore">
          {{ contentLoading ? 'Loading...' : 'Load More' }}
        </button>
      </div>
    </template>
  </div>
</template>

<style scoped>
.fp {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  border: var(--border);
  border-radius: var(--radius-sm);
  padding: var(--space-2);
  background: var(--surface-panel);
}

.fp-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-2);
}

.fp-actions {
  display: inline-flex;
  align-items: center;
  gap: var(--space-2);
}

.fp-validate {
  padding: 0 var(--space-1);
  border-radius: var(--radius-sm);
  border: var(--border);
  color: var(--text-secondary);
  font: var(--type-caption);
  white-space: nowrap;
}

.fp-validate--ok {
  color: var(--color-success);
  background: var(--color-success-bg);
}

.fp-validate--bad {
  color: var(--color-danger);
  background: var(--color-danger-bg);
}

.fp-title-wrap {
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.fp-title {
  font: var(--type-body-sm);
  color: var(--text-primary);
}

.fp-path {
  color: var(--text-tertiary);
  font: var(--type-caption);
  font-family: var(--font-family-mono);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.fp-btn {
  padding: var(--comp-padding-xs);
  border: var(--border);
  border-radius: var(--radius-sm);
  background: var(--surface-card-muted);
  color: var(--text-secondary);
  font: var(--type-body-sm);
  cursor: pointer;
  transition: background var(--duration-fast) ease, color var(--duration-fast) ease, opacity var(--duration-fast) ease;
}

.fp-btn:hover:not(:disabled) {
  background: var(--gray-alpha-100);
  color: var(--text-primary);
}

.fp-btn:disabled {
  opacity: var(--disabled-opacity);
  cursor: not-allowed;
}

.fp-placeholder {
  padding: var(--space-3);
  text-align: center;
  color: var(--text-secondary);
  font: var(--type-body-sm);
}

.fp-error {
  padding: var(--space-2);
  border-radius: var(--radius-sm);
  background: var(--color-danger-bg);
  color: var(--color-danger);
  font: var(--type-body-sm);
}

.fp-meta {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: var(--space-1) var(--space-2);
}

.fp-meta-item {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}

.fp-meta-item--wide {
  grid-column: span 4;
}

.fp-k {
  color: var(--text-tertiary);
  font: var(--type-caption);
}

.fp-v {
  color: var(--text-primary);
  font: var(--type-body-sm);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.fp-validation-btn {
  width: fit-content;
  border: var(--border);
  border-radius: var(--radius-sm);
  background: var(--color-danger-bg);
  color: var(--color-danger);
  font: var(--type-caption);
  padding: 0 var(--space-1);
  cursor: pointer;
}

.fp-validation-panel {
  border: var(--border);
  border-radius: var(--radius-sm);
  background: var(--surface-card-muted);
  padding: var(--space-2);
}

.fp-validation-panel ul {
  margin: 0;
  padding: 0;
  list-style: none;
}

.fp-validation-panel li {
  margin-bottom: 2px;
}

.fp-validation-item {
  width: 100%;
  display: flex;
  align-items: flex-start;
  gap: var(--space-1);
  border: none;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--text-primary);
  font: var(--type-body-sm);
  text-align: left;
  padding: var(--space-1);
  cursor: pointer;
}

.fp-validation-item:hover:not(:disabled) {
  background: var(--gray-alpha-100);
}

.fp-validation-item:disabled {
  cursor: default;
  opacity: 0.75;
}

.fp-validation-item--disabled:hover {
  background: transparent;
}

.fp-validation-loc {
  color: var(--color-danger);
  font-family: var(--font-family-mono);
  margin-right: var(--space-1);
}

.fp-content {
  border: var(--border);
  border-radius: var(--radius-sm);
  max-height: 220px;
  overflow: auto;
}

.fp-table {
  width: 100%;
  border-collapse: collapse;
  table-layout: fixed;
  font-family: var(--font-family-mono);
  font-size: var(--text-xs);
}

.fp-ln {
  width: 56px;
  text-align: right;
  color: var(--text-tertiary);
  padding: 0 var(--space-2);
  user-select: none;
  vertical-align: top;
  border-right: var(--border);
}

.fp-text {
  padding: 0 var(--space-2);
  vertical-align: top;
}

.fp-text pre {
  margin: 0;
  white-space: pre-wrap;
  word-break: break-word;
}

.fp-row--highlight {
  background: var(--color-warning-bg);
}

.fp-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-2);
}

.fp-progress {
  color: var(--text-secondary);
  font: var(--type-caption);
  font-family: var(--font-family-mono);
}

@media (max-width: 768px) {
  .fp-meta {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  .fp-meta-item--wide {
    grid-column: span 2;
  }
}
</style>
