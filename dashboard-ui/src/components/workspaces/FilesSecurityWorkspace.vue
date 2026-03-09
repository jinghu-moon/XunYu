<script setup lang="ts">
import { computed, nextTick, ref } from 'vue'
import type { RecentTasksFocusRequest, StatisticsWorkspaceLinkPayload, WorkspaceCapabilities } from '../../types'
import type { TaskFormState } from '../../workspace-tools'
import { filesSecurityTaskGroups } from '../../workspace-tools'
import BatchGovernancePanel from '../BatchGovernancePanel.vue'
import FileGovernancePanel from '../FileGovernancePanel.vue'
import RecentTasksPanel from '../RecentTasksPanel.vue'
import RecipePanel from '../RecipePanel.vue'
import { Button } from '../button'
import DiffPanel from '../DiffPanel.vue'
import RedirectPanel from '../RedirectPanel.vue'
import TaskToolbox from '../TaskToolbox.vue'
import WorkspaceFrame from '../WorkspaceFrame.vue'

const emit = defineEmits<{
  (event: 'link-panel', payload: StatisticsWorkspaceLinkPayload): void
}>()

defineProps<{
  capabilities?: WorkspaceCapabilities | null
}>()

type TaskPresetMap = Record<string, Partial<TaskFormState>>

const currentDirectory = ref('')
const selectedPath = ref('')
const batchPaths = ref<string[]>([])
const recentTasksFocus = ref<RecentTasksFocusRequest | null>(null)
const recentTasksFocusKey = ref(0)
const recentTasksAnchor = ref<HTMLElement | null>(null)
const taskPresets = ref<TaskPresetMap>({})
const presetVersion = ref(0)
const syncMessage = ref('等待从上方文件管理器同步上下文。')

const hasDirectory = computed(() => Boolean(currentDirectory.value.trim()))
const hasSelection = computed(() => Boolean(selectedPath.value.trim()))
const hasBatch = computed(() => batchPaths.value.length > 0)
const canQueueSelection = computed(
  () => hasSelection.value && !batchPaths.value.includes(selectedPath.value.trim()),
)
const batchPreview = computed(() => batchPaths.value.slice(0, 6))
const batchOverflow = computed(() => Math.max(batchPaths.value.length - batchPreview.value.length, 0))

function normalizePath(path: string): string {
  return path.trim()
}


function parentDirectory(path: string): string {
  const normalized = normalizePath(path)
  if (!normalized) return ''
  if (normalized === '/' || /^[A-Za-z]:[\\/]?$/.test(normalized)) {
    return normalized
  }
  const sep = normalized.includes('\\') ? '\\' : '/'
  const trimmed = normalized.replace(/[\\/]+$/, '')
  const idx = trimmed.lastIndexOf(sep)
  if (idx <= 0) {
    return /^[A-Za-z]:/.test(trimmed) ? `${trimmed.slice(0, 2)}\\` : '/'
  }
  const head = trimmed.slice(0, idx)
  if (/^[A-Za-z]:$/.test(head)) return `${head}\\`
  return head
}

function pushTaskPreset(target: TaskPresetMap, taskId: string, values: Partial<TaskFormState>) {
  target[taskId] = {
    ...(target[taskId] ?? {}),
    ...values,
  }
}

function mergePresetMaps(...maps: TaskPresetMap[]): TaskPresetMap {
  return maps.reduce<TaskPresetMap>((acc, current) => {
    for (const [taskId, values] of Object.entries(current)) {
      pushTaskPreset(acc, taskId, values)
    }
    return acc
  }, {})
}

function buildDirectoryPresets(): TaskPresetMap {
  const dir = normalizePath(currentDirectory.value)
  if (!dir) return {}
  const next: TaskPresetMap = {}
  pushTaskPreset(next, 'tree', { path: dir })
  pushTaskPreset(next, 'find', { paths: dir })
  pushTaskPreset(next, 'bak-list', { dir })
  pushTaskPreset(next, 'bak-create', { dir })
  return next
}

function buildSelectionPresets(): TaskPresetMap {
  const path = normalizePath(selectedPath.value)
  if (!path) return {}
  const dir = normalizePath(currentDirectory.value) || parentDirectory(path)
  const next: TaskPresetMap = {}
  if (dir) {
    pushTaskPreset(next, 'tree', { path: dir })
    pushTaskPreset(next, 'bak-create', { dir })
  }
  pushTaskPreset(next, 'find', { paths: path })
  pushTaskPreset(next, 'bak-create', { include: path })
  pushTaskPreset(next, 'rm', { path })
  pushTaskPreset(next, 'mv', { src: path })
  pushTaskPreset(next, 'ren', { src: path })
  for (const taskId of [
    'lock-who',
    'protect-status',
    'protect-set',
    'protect-clear',
    'acl-view',
    'acl-diff',
    'acl-add',
    'acl-effective',
    'acl-backup',
    'acl-copy',
    'acl-restore',
    'acl-purge',
    'acl-inherit',
    'acl-owner',
    'acl-repair',
    'encrypt',
    'decrypt',
  ]) {
    pushTaskPreset(next, taskId, { path })
  }
  return next
}

function buildBatchFindPresets(): TaskPresetMap {
  if (!hasBatch.value) return {}
  const next = buildDirectoryPresets()
  pushTaskPreset(next, 'find', { paths: batchPaths.value.join('\n') })
  return next
}

function buildBatchBackupPresets(): TaskPresetMap {
  if (!hasBatch.value) return {}
  const next = buildDirectoryPresets()
  const fallbackDir = normalizePath(currentDirectory.value) || parentDirectory(batchPaths.value[0])
  if (fallbackDir) {
    pushTaskPreset(next, 'bak-create', { dir: fallbackDir })
  }
  pushTaskPreset(next, 'bak-create', { include: batchPaths.value.join('\n') })
  return next
}

function applyTaskPresets(presets: TaskPresetMap, message: string) {
  taskPresets.value = presets
  presetVersion.value += 1
  syncMessage.value = message
}

function syncDirectoryContext() {
  applyTaskPresets(buildDirectoryPresets(), `已将目录上下文同步到 tree / find / bak：${currentDirectory.value || '-'}`)
}

function syncSelectionContext() {
  applyTaskPresets(
    mergePresetMaps(buildDirectoryPresets(), buildSelectionPresets()),
    `已将文件上下文同步到删除、移动、ACL、加解密等任务：${selectedPath.value || '-'}`,
  )
}

function syncAllContext() {
  applyTaskPresets(
    mergePresetMaps(buildDirectoryPresets(), buildSelectionPresets()),
    '已将当前目录与当前文件同步到文件任务区。',
  )
}

function syncBatchToFind() {
  applyTaskPresets(buildBatchFindPresets(), `已将 ${batchPaths.value.length} 个条目填入高级查找。`)
}

function syncBatchToBackup() {
  applyTaskPresets(buildBatchBackupPresets(), `已将 ${batchPaths.value.length} 个条目填入备份 include。`)
}

function addSelectionToBatch() {
  const path = normalizePath(selectedPath.value)
  if (!path || batchPaths.value.includes(path)) return
  batchPaths.value = [...batchPaths.value, path]
  syncMessage.value = `已加入批量队列：${path}`
}

function removeBatchPath(path: string) {
  batchPaths.value = batchPaths.value.filter((item) => item !== path)
  syncMessage.value = batchPaths.value.length ? '已更新批量队列。' : '批量队列已清空。'
}

function clearBatch() {
  batchPaths.value = []
  syncMessage.value = '已清空批量队列。'
}

async function focusRecentTasks(request: Omit<RecentTasksFocusRequest, 'key'>) {
  recentTasksFocusKey.value += 1
  recentTasksFocus.value = {
    key: recentTasksFocusKey.value,
    ...request,
  }
  await nextTick()
  recentTasksAnchor.value?.scrollIntoView?.({ behavior: 'smooth', block: 'start' })
}

async function handleWorkspaceLink(payload: StatisticsWorkspaceLinkPayload) {
  if (payload.panel === 'recent-tasks') {
    await focusRecentTasks(payload.request)
    return
  }
  emit('link-panel', payload)
}

function onDirectoryChange(path: string) {
  currentDirectory.value = normalizePath(path)
}

function onSelectionChange(path: string) {
  selectedPath.value = normalizePath(path)
}
</script>

<template>
  <WorkspaceFrame
    title="文件与安全"
    description="保留 Diff / Redirect 观察能力，并把 tree / find / bak / rm / acl / protect / encrypt 收口到统一文件治理工作台。"
  >
    <template #summary>
      <div class="files-security__summary">
        <span class="files-security__summary-chip">目录 {{ currentDirectory || '-' }}</span>
        <span class="files-security__summary-chip">文件 {{ selectedPath || '-' }}</span>
        <span class="files-security__summary-chip">批量 {{ batchPaths.length }}</span>
      </div>
    </template>

    <div class="files-security__top">
      <div class="files-security__main">
        <DiffPanel
          @directory-change="onDirectoryChange"
          @selection-change="onSelectionChange"
        />
        <RedirectPanel />
      </div>

      <aside class="files-security__side">
        <section class="files-security__card">
          <header class="files-security__card-header">
            <div>
              <h3 class="files-security__card-title">文件上下文桥接</h3>
              <p class="files-security__card-desc">
                从 File Manager 取当前目录和当前文件，一键填充到任务卡；危险动作仍然必须走预演、确认、回执。
              </p>
            </div>
          </header>

          <div class="files-security__context-grid">
            <div class="files-security__context-item">
              <span class="files-security__context-label">当前目录</span>
              <strong class="files-security__context-value">{{ currentDirectory || '-' }}</strong>
            </div>
            <div class="files-security__context-item">
              <span class="files-security__context-label">当前文件</span>
              <strong class="files-security__context-value">{{ selectedPath || '-' }}</strong>
            </div>
          </div>

          <div class="files-security__actions">
            <Button preset="secondary" :disabled="!hasDirectory" @click="syncDirectoryContext">同步目录任务</Button>
            <Button preset="secondary" :disabled="!hasSelection" @click="syncSelectionContext">同步文件任务</Button>
            <Button preset="primary" :disabled="!hasDirectory && !hasSelection" @click="syncAllContext">同步全部</Button>
            <Button preset="secondary" :disabled="!canQueueSelection" @click="addSelectionToBatch">加入批量队列</Button>
          </div>

          <p class="files-security__message">{{ syncMessage }}</p>

          <div class="files-security__batch">
            <div class="files-security__batch-header">
              <div>
                <h4 class="files-security__batch-title">批量队列与治理</h4>
                <p class="files-security__batch-desc">??????????? protect / encrypt / decrypt / ACL ???????????????????????????</p>
              </div>
              <Button preset="secondary" :disabled="!hasBatch" @click="clearBatch">清空</Button>
            </div>

            <div class="files-security__actions files-security__actions--batch">
              <Button preset="secondary" :disabled="!hasBatch" @click="syncBatchToFind">批量填充查找</Button>
              <Button preset="secondary" :disabled="!hasBatch" @click="syncBatchToBackup">批量填充备份</Button>
            </div>

            <div v-if="hasBatch" class="files-security__batch-list">
              <div v-for="path in batchPreview" :key="path" class="files-security__batch-item">
                <span>{{ path }}</span>
                <button type="button" class="files-security__remove-btn" @click="removeBatchPath(path)">移除</button>
              </div>
              <p v-if="batchOverflow > 0" class="files-security__batch-more">还有 {{ batchOverflow }} 项未展开。</p>
            </div>
            <p v-else class="files-security__empty">先在 File Manager 选中文件，再加入批量队列。</p>
          </div>

          <BatchGovernancePanel
            :paths="batchPaths"
            :capabilities="capabilities"
            @focus-recent-tasks="focusRecentTasks"
          />
        </section>

        <FileGovernancePanel :path="selectedPath" :capabilities="capabilities" />

        <RecentTasksPanel
          title="文件任务中心"
          description="只显示 Files & Security 工作台的最近任务，支持安全重放。"
          workspace="files-security"
          :limit="12"
          :focus-request="recentTasksFocus"
        />

        <RecipePanel
          title="文件 Recipes"
          description="沉淀文件清理、扫描、备份等顺序流程，避免重复点命令。"
          category="files-security"
        />
      </aside>
    </div>

    <section class="files-security__task-zone">
      <header class="files-security__section-header">
        <div>
          <h3 class="files-security__section-title">文件操作任务区</h3>
          <p class="files-security__section-desc">
            先在上方锁定目录 / 文件，再通过“同步”把上下文带入任务卡；高风险动作继续由统一确认弹窗和回执组件兜底。
          </p>
        </div>
      </header>

      <TaskToolbox
        v-for="group in filesSecurityTaskGroups"
        :key="group.id"
        :title="group.title"
        :description="group.description"
        :tasks="group.tasks"
        :capabilities="capabilities"
        :task-presets="taskPresets"
        :preset-version="presetVersion"
        @link-panel="handleWorkspaceLink"
      />
    </section>
  </WorkspaceFrame>
</template>

<style scoped>
.files-security__summary {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
}

.files-security__summary-chip {
  display: inline-flex;
  max-width: 320px;
  align-items: center;
  padding: 2px var(--space-3);
  border-radius: var(--radius-full);
  background: var(--ds-background-2);
  color: var(--text-secondary);
  font: var(--type-caption);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.files-security__top {
  display: grid;
  grid-template-columns: minmax(0, 1.8fr) minmax(340px, 0.9fr);
  gap: var(--space-5);
  align-items: start;
}

.files-security__main,
.files-security__side,
.files-security__task-zone,
.files-security__batch {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}

.files-security__card {
  border: var(--card-border);
  border-radius: var(--card-radius);
  background: var(--surface-card);
  box-shadow: var(--card-shadow);
  padding: var(--card-padding);
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}

.files-security__card-title,
.files-security__section-title,
.files-security__batch-title {
  font: var(--type-title-sm);
  color: var(--text-primary);
}

.files-security__card-desc,
.files-security__section-desc,
.files-security__batch-desc,
.files-security__message,
.files-security__empty,
.files-security__batch-more {
  color: var(--text-secondary);
  font: var(--type-body-sm);
}

.files-security__context-grid {
  display: grid;
  gap: var(--space-3);
}

.files-security__context-item {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
  padding: var(--space-3);
  border: var(--border);
  border-radius: var(--radius-md);
  background: var(--surface-panel);
}

.files-security__context-label {
  color: var(--text-secondary);
  font: var(--type-caption);
}

.files-security__context-value {
  color: var(--text-primary);
  font: var(--type-body-sm);
  font-family: var(--font-family-mono);
  word-break: break-all;
}

.files-security__actions {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
}

.files-security__actions--batch {
  margin-top: calc(var(--space-2) * -1);
}

.files-security__batch-header,
.files-security__section-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: var(--space-3);
}

.files-security__batch-list {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}

.files-security__batch-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: var(--space-3);
  padding: var(--space-2) var(--space-3);
  border: var(--border);
  border-radius: var(--radius-md);
  background: var(--surface-panel);
  font: var(--type-body-sm);
  color: var(--text-primary);
}

.files-security__batch-item span {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.files-security__remove-btn {
  border: none;
  background: transparent;
  color: var(--color-danger);
  cursor: pointer;
  font: var(--type-caption);
}

@media (max-width: 1280px) {
  .files-security__top {
    grid-template-columns: 1fr;
  }
}
</style>



