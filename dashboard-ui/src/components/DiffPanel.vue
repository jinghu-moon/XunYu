<script setup lang="ts">
import { reactive, ref } from 'vue'
import { fetchConvertFile, fetchDiff } from '../api'
import type { ConfigDiffNode, ConfigDiffStats, DiffResult } from '../types'
import DiffOptions from './diff/DiffOptions.vue'
import DiffFileManager from './diff/DiffFileManager.vue'
import DiffConvertPanel from './diff/DiffConvertPanel.vue'
import DiffStats from './diff/DiffStats.vue'
import ConfigDiffTree from './diff/ConfigDiffTree.vue'
import CodeDiffPanel from './diff/CodeDiffPanel.vue'
import LineDiffPanel from './diff/LineDiffPanel.vue'

const emit = defineEmits<{
  (event: 'selectionChange', value: string): void
  (event: 'directoryChange', value: string): void
}>()

const oldPath = ref('')
const newPath = ref('')
const viewMode = ref<'unified' | 'split'>('unified')
const showOptions = ref(false)
const showConvert = ref(false)
const convertPath = ref('')
const busy = ref(false)
const result = ref<DiffResult | null>(null)
const configSemantic = ref<{ root: ConfigDiffNode; stats: ConfigDiffStats } | null>(null)
const configSemanticError = ref('')
const errorMsg = ref('')

const options = reactive({
  mode: 'auto',
  algorithm: 'histogram',
  context: 3,
  ignore_space_change: false,
  ignore_all_space: false,
  ignore_blank_lines: false,
  strip_trailing_cr: false,
  force_text: false,
})

const CONFIG_EXTS = new Set(['toml', 'yaml', 'yml', 'json', 'json5'])

function updateOptions(next: typeof options) {
  Object.assign(options, next)
}


async function runDiff() {
  const oldTrimmed = oldPath.value.trim()
  const newTrimmed = newPath.value.trim()
  if (!oldTrimmed || !newTrimmed) return

  busy.value = true
  errorMsg.value = ''
  result.value = null
  configSemantic.value = null
  configSemanticError.value = ''
  try {
    let semanticPromise: Promise<{ root: ConfigDiffNode; stats: ConfigDiffStats } | null> =
      Promise.resolve(null)
    if (shouldBuildConfigSemantic(oldTrimmed, newTrimmed)) {
      semanticPromise = buildConfigSemanticDiff(oldTrimmed, newTrimmed).catch((e: unknown) => {
        configSemanticError.value = e instanceof Error ? e.message : String(e)
        return null
      })
    }

    result.value = await fetchDiff({
      old_path: oldTrimmed,
      new_path: newTrimmed,
      mode: options.mode as any,
      algorithm: options.algorithm as any,
      context: options.context,
      ignore_space_change: options.ignore_space_change,
      ignore_all_space: options.ignore_all_space,
      ignore_blank_lines: options.ignore_blank_lines,
      strip_trailing_cr: options.strip_trailing_cr,
      force_text: options.force_text,
    })
    configSemantic.value = await semanticPromise
  } catch (e: any) {
    errorMsg.value = e?.message || 'Diff failed'
  } finally {
    busy.value = false
  }
}

function extensionOf(path: string): string {
  const name = path.split(/[\\/]/).pop() ?? ''
  const dot = name.lastIndexOf('.')
  if (dot <= 0 || dot === name.length - 1) return ''
  return name.slice(dot + 1).toLowerCase()
}

function isConfigPath(path: string): boolean {
  return CONFIG_EXTS.has(extensionOf(path))
}

function shouldBuildConfigSemantic(oldP: string, newP: string): boolean {
  return isConfigPath(oldP) && isConfigPath(newP)
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value != null && !Array.isArray(value)
}

function joinConfigPath(parent: string, key: string): string {
  if (!parent) return key
  if (key.startsWith('[')) return `${parent}${key}`
  return `${parent}.${key}`
}

function buildConfigNode(
  key: string,
  parentPath: string,
  oldValue: unknown,
  newValue: unknown,
): ConfigDiffNode {
  const path = joinConfigPath(parentPath, key)
  const oldIsArray = Array.isArray(oldValue)
  const newIsArray = Array.isArray(newValue)
  const oldIsObject = isRecord(oldValue)
  const newIsObject = isRecord(newValue)

  if (oldIsArray || newIsArray) {
    const left = oldIsArray ? oldValue : []
    const right = newIsArray ? newValue : []
    const maxLen = Math.max(left.length, right.length)
    const children: ConfigDiffNode[] = Array.from({ length: maxLen }, (_, idx) =>
      buildConfigNode(`[${idx}]`, path, left[idx], right[idx]),
    )
    const status =
      oldValue === undefined
        ? 'added'
        : newValue === undefined
          ? 'removed'
          : children.some((n) => n.status !== 'unchanged')
            ? 'modified'
            : 'unchanged'
    return {
      key,
      path,
      kind: 'array',
      status,
      oldValue,
      newValue,
      children,
    }
  }

  if (oldIsObject || newIsObject) {
    const left = oldIsObject ? oldValue : {}
    const right = newIsObject ? newValue : {}
    const keys = new Set<string>([...Object.keys(left), ...Object.keys(right)])
    const children = Array.from(keys)
      .sort((a, b) => a.localeCompare(b))
      .map((childKey) => buildConfigNode(childKey, path, left[childKey], right[childKey]))
    const status =
      oldValue === undefined
        ? 'added'
        : newValue === undefined
          ? 'removed'
          : children.some((n) => n.status !== 'unchanged')
            ? 'modified'
            : 'unchanged'
    return {
      key,
      path,
      kind: 'object',
      status,
      oldValue,
      newValue,
      children,
    }
  }

  const status =
    oldValue === undefined
      ? 'added'
      : newValue === undefined
        ? 'removed'
        : Object.is(oldValue, newValue)
          ? 'unchanged'
          : 'modified'

  return {
    key,
    path,
    kind: 'value',
    status,
    oldValue,
    newValue,
    children: [],
  }
}

function collectConfigStats(node: ConfigDiffNode, stats: ConfigDiffStats): void {
  if (node.children?.length) {
    for (const child of node.children) {
      collectConfigStats(child, stats)
    }
    return
  }
  if (node.status === 'added') stats.added += 1
  else if (node.status === 'removed') stats.removed += 1
  else if (node.status === 'modified') stats.modified += 1
  else stats.unchanged += 1
}

async function buildConfigSemanticDiff(
  oldP: string,
  newP: string,
): Promise<{ root: ConfigDiffNode; stats: ConfigDiffStats }> {
  const [oldConverted, newConverted] = await Promise.all([
    fetchConvertFile({ path: oldP, to_format: 'json', preview: true }),
    fetchConvertFile({ path: newP, to_format: 'json', preview: true }),
  ])

  let oldJson: unknown
  let newJson: unknown
  try {
    oldJson = JSON.parse(oldConverted.content)
    newJson = JSON.parse(newConverted.content)
  } catch (e: unknown) {
    throw new Error(e instanceof Error ? `Config semantic parse failed: ${e.message}` : 'Config semantic parse failed')
  }

  const root = buildConfigNode('root', '', oldJson, newJson)
  const stats: ConfigDiffStats = { added: 0, removed: 0, modified: 0, unchanged: 0 }
  collectConfigStats(root, stats)
  return { root, stats }
}

function updateOldPath(path: string) {
  oldPath.value = path
}

function updateNewPath(path: string) {
  newPath.value = path
}

function swapPaths() {
  const old = oldPath.value
  oldPath.value = newPath.value
  newPath.value = old
}

function openConvert(path: string) {
  convertPath.value = path
  showConvert.value = true
}

function forwardSelectionChange(path: string) {
  emit('selectionChange', path)
}

function forwardDirectoryChange(path: string) {
  emit('directoryChange', path)
}
</script>

<template>
  <div class="diff-panel">
    <div class="diff-layout">
      <aside class="diff-sidebar">
        <DiffFileManager
          :old-path="oldPath"
          :new-path="newPath"
          @update:old-path="updateOldPath"
          @update:new-path="updateNewPath"
          @run-diff="runDiff"
          @open-convert="openConvert"
          @selection-change="forwardSelectionChange"
          @directory-change="forwardDirectoryChange"
        />
      </aside>

      <section class="diff-main">
        <!-- Path inputs -->
        <div class="path-section">
          <div class="path-row">
            <label class="path-label">Old</label>
            <input
              v-model="oldPath"
              type="text"
              class="path-input"
              placeholder="Path to old file..."
              @keydown.enter="runDiff"
            />
          </div>
          <div class="path-row">
            <label class="path-label">New</label>
            <input
              v-model="newPath"
              type="text"
              class="path-input"
              placeholder="Path to new file..."
              @keydown.enter="runDiff"
            />
            <button
              class="swap-btn"
              :disabled="busy || !oldPath.trim() || !newPath.trim()"
              @click="swapPaths"
            >
              Swap
            </button>
            <button
              class="run-btn"
              :disabled="busy || !oldPath.trim() || !newPath.trim()"
              @click="runDiff"
            >
              {{ busy ? 'Running...' : 'Run Diff' }}
            </button>
          </div>
        </div>

        <!-- Options toggle -->
        <div class="options-toggle">
          <button class="toggle-btn" @click="showOptions = !showOptions">
            <span class="toggle-arrow" :class="{ open: showOptions }">&#9654;</span>
            Options
          </button>
          <button class="toggle-btn" @click="showConvert = !showConvert">
            <span class="toggle-arrow" :class="{ open: showConvert }">&#9654;</span>
            Convert
          </button>
        </div>

        <!-- Options panel -->
        <div v-if="showOptions" class="options-area">
          <DiffOptions :model-value="options" @update:model-value="updateOptions" />
        </div>
        <div v-if="showConvert" class="options-area">
          <DiffConvertPanel :path="convertPath" />
        </div>

        <!-- Result area -->
        <div v-if="errorMsg" class="diff-error">{{ errorMsg }}</div>

        <div v-if="result" class="result-section">
          <div v-if="configSemanticError" class="diff-warn">{{ configSemanticError }}</div>

          <!-- Identical -->
          <div v-if="result.kind === 'identical'" class="diff-msg diff-msg--ok">
            <strong>Files are identical.</strong>
            <span v-if="result.identical_with_filters"> (with whitespace filters applied)</span>
          </div>

          <!-- Binary -->
          <div v-else-if="result.kind === 'binary'" class="diff-msg diff-msg--warn">
            <strong>Binary files differ.</strong>
          </div>

          <!-- Config semantic diff -->
          <template v-else-if="configSemantic">
            <div class="result-header">
              <span class="stats-text">Config Semantic Diff</span>
              <span class="algo-badge">{{ result.actual_algorithm }}</span>
              <span class="mode-badge mode-badge--config">CONFIG</span>
            </div>
            <DiffStats
              :added="configSemantic.stats.added"
              :removed="configSemantic.stats.removed"
              :modified="configSemantic.stats.modified"
              :unchanged="configSemantic.stats.unchanged"
              unit-label="nodes"
            />
            <div class="viewer-wrap viewer-wrap--tree">
              <ConfigDiffTree :node="configSemantic.root" />
            </div>
          </template>

          <!-- AST code diff -->
          <template v-else-if="result.kind === 'ast'">
            <div class="result-header">
              <span class="stats-text">Code Symbol Diff</span>
              <span class="algo-badge">{{ result.actual_algorithm }}</span>
              <span class="mode-badge">AST</span>
              <div class="view-switch">
                <button
                  :class="{ active: viewMode === 'unified' }"
                  @click="viewMode = 'unified'"
                >Unified</button>
                <button
                  :class="{ active: viewMode === 'split' }"
                  @click="viewMode = 'split'"
                >Split</button>
              </div>
            </div>
            <DiffStats
              :added="result.stats.added"
              :removed="result.stats.removed"
              :modified="result.stats.modified"
              :unchanged="result.stats.unchanged"
              unit-label="symbols"
            />
            <div class="viewer-wrap">
              <CodeDiffPanel :hunks="result.hunks" :view-mode="viewMode" />
            </div>
          </template>

          <!-- Line fallback diff -->
          <template v-else>
            <div class="result-header">
              <span class="stats-text">Line Diff</span>
              <span class="algo-badge">{{ result.actual_algorithm }}</span>
              <div class="view-switch">
                <button
                  :class="{ active: viewMode === 'unified' }"
                  @click="viewMode = 'unified'"
                >Unified</button>
                <button
                  :class="{ active: viewMode === 'split' }"
                  @click="viewMode = 'split'"
                >Split</button>
              </div>
            </div>
            <DiffStats
              :added="result.stats.added"
              :removed="result.stats.removed"
              :modified="result.stats.modified"
              :unchanged="result.stats.unchanged"
              unit-label="lines"
            />
            <div class="viewer-wrap">
              <LineDiffPanel :hunks="result.hunks" :view-mode="viewMode" kind="line" />
            </div>
          </template>
        </div>
      </section>
    </div>
  </div>
</template>

<style scoped>
.diff-panel {
  width: 100%;
}

.diff-layout {
  display: flex;
  gap: var(--space-4);
  align-items: flex-start;
}

.diff-sidebar {
  width: min(420px, 42vw);
  min-width: 320px;
}

.diff-main {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}

/* ── Path section ── */
.path-section {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}

.path-row {
  display: flex;
  align-items: center;
  gap: var(--space-2);
}

.path-label {
  width: 36px;
  flex-shrink: 0;
  font: var(--type-body-sm);
  color: var(--text-secondary);
  text-align: right;
}

.path-input {
  flex: 1;
  padding: var(--comp-padding-sm);
  border: var(--border);
  border-radius: var(--radius-sm);
  background: var(--surface-panel);
  color: var(--text-primary);
  font: var(--type-body-sm);
  font-family: var(--font-family-mono);
  outline: none;
  transition: border-color var(--duration-fast) ease;
}

.path-input:focus {
  border-color: var(--text-secondary);
}

.run-btn {
  padding: var(--comp-padding-sm);
  border: var(--border);
  border-radius: var(--radius-sm);
  background: var(--color-info-bg);
  color: var(--color-info);
  font: var(--type-body-sm);
  font-weight: var(--weight-medium);
  cursor: pointer;
  white-space: nowrap;
  transition: background var(--duration-fast) ease, opacity var(--duration-fast) ease;
}

.run-btn:hover:not(:disabled) {
  background: var(--color-info);
  color: var(--ds-background-1);
}

.run-btn:disabled {
  opacity: var(--disabled-opacity);
  cursor: not-allowed;
}

.swap-btn {
  padding: var(--comp-padding-sm);
  border: var(--border);
  border-radius: var(--radius-sm);
  background: var(--surface-card-muted);
  color: var(--text-secondary);
  font: var(--type-body-sm);
  cursor: pointer;
  white-space: nowrap;
  transition: background var(--duration-fast) ease, color var(--duration-fast) ease, opacity var(--duration-fast) ease;
}

.swap-btn:hover:not(:disabled) {
  background: var(--ds-color-3);
  color: var(--text-primary);
}

.swap-btn:disabled {
  opacity: var(--disabled-opacity);
  cursor: not-allowed;
}

/* ── Options toggle ── */
.options-toggle {
  display: flex;
  gap: var(--space-3);
}

.toggle-btn {
  display: inline-flex;
  align-items: center;
  gap: var(--space-1);
  background: none;
  border: none;
  cursor: pointer;
  font: var(--type-body-sm);
  color: var(--text-secondary);
  padding: 0;
}

.toggle-btn:hover {
  color: var(--text-primary);
}

.toggle-arrow {
  display: inline-block;
  font-size: var(--text-xs);
  transition: transform var(--duration-fast) ease;
}

.toggle-arrow.open {
  transform: rotate(90deg);
}

.options-area {
  padding: var(--space-3);
  border: var(--border);
  border-radius: var(--radius-sm);
  background: var(--surface-card-muted);
}

/* ── Error ── */
.diff-error {
  padding: var(--space-3);
  border-radius: var(--radius-sm);
  background: var(--color-danger-bg);
  color: var(--color-danger);
  font: var(--type-body-sm);
}

.diff-warn {
  padding: var(--space-2);
  border-radius: var(--radius-sm);
  background: var(--color-warning-bg);
  color: var(--color-warning);
  font: var(--type-body-sm);
}

/* ── Messages ── */
.diff-msg {
  padding: var(--space-4);
  border-radius: var(--radius-sm);
  font: var(--type-body-sm);
  text-align: center;
}

.diff-msg--ok {
  background: var(--color-success-bg);
  color: var(--color-success);
}

.diff-msg--warn {
  background: var(--color-warning-bg);
  color: var(--color-warning);
}

/* ── Result header ── */
.result-header {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  flex-wrap: wrap;
}

.stats-text {
  font: var(--type-body-sm);
  color: var(--text-secondary);
}

.algo-badge,
.mode-badge {
  display: inline-flex;
  align-items: center;
  padding: 2px var(--space-2);
  border-radius: var(--radius-full);
  font: var(--type-caption);
  background: var(--color-info-bg);
  color: var(--color-info);
  border: 1px solid var(--color-info);
}

.mode-badge {
  background: var(--color-success-bg);
  color: var(--color-success);
  border-color: var(--color-success);
}

.mode-badge--config {
  background: var(--color-warning-bg);
  color: var(--color-warning);
  border-color: var(--color-warning);
}

.view-switch {
  display: inline-flex;
  margin-left: auto;
  border: var(--border);
  border-radius: var(--radius-sm);
  overflow: hidden;
}

.view-switch button {
  padding: var(--comp-padding-xs);
  background: var(--surface-panel);
  color: var(--text-secondary);
  border: none;
  cursor: pointer;
  font: var(--type-body-sm);
  transition: background var(--duration-fast) ease, color var(--duration-fast) ease;
}

.view-switch button:not(:last-child) {
  border-right: var(--border);
}

.view-switch button.active {
  background: var(--color-info-bg);
  color: var(--color-info);
}

.view-switch button:hover:not(.active) {
  background: var(--gray-alpha-100);
}

/* ── Viewer wrapper ── */
.viewer-wrap {
  overflow-x: auto;
  border: var(--border);
  border-radius: var(--radius-sm);
}

.viewer-wrap--tree {
  max-height: 520px;
  overflow: auto;
}

@media (max-width: 768px) {
  .diff-layout {
    flex-direction: column;
  }

  .diff-sidebar {
    width: 100%;
    min-width: 0;
  }

  .path-row {
    flex-wrap: wrap;
  }

  .path-label {
    width: auto;
    min-width: 36px;
    text-align: left;
  }
}
</style>
