<script setup lang="ts">

import { computed, onMounted, ref, watch } from 'vue'

import {

  executeGuardedTask,

  fetchRecentWorkspaceTasks,

  previewGuardedTask,

  runWorkspaceTask,

} from '../api'

import type {

  GuardedTaskPreviewResponse,

  GuardedTaskReceipt,

  RecentTaskDryRunFilter,

  RecentTaskListResponse,

  RecentTaskRecord,

  RecentTaskStatusFilter,

  RecentTasksFocusRequest,

  StatisticsWorkspaceLinkPayload,

  WorkspaceTaskRunResponse,

} from '../types'

import { Button } from './button'

import FileGovernanceSummary from './FileGovernanceSummary.vue'

import TaskReceiptComponent from './TaskReceiptComponent.vue'

import UnifiedConfirmDialog from './UnifiedConfirmDialog.vue'

import { resolveRecentTaskGovernanceContext } from './recent-task-governance'

import { resolveDiagnosticsCenterFocusFromRecentTask } from './statistics-diagnostics-focus'



const emit = defineEmits<{
  (event: 'link-panel', payload: StatisticsWorkspaceLinkPayload): void
}>()

const props = withDefaults(

  defineProps<{

    title?: string

    description?: string

    limit?: number

    workspace?: string

    focusRequest?: RecentTasksFocusRequest | null

  }>(),

  {

    title: '最近任务',

    description: '跨工作台查看最近执行结果，并支持安全重放。',

    limit: 20,

    workspace: '',

    focusRequest: null,

  },

)



const entries = ref<RecentTaskRecord[]>([])

const stats = ref<RecentTaskListResponse['stats'] | null>(null)

const selectedId = ref('')

const statusFilter = ref<RecentTaskStatusFilter>('all')

const dryRunFilter = ref<RecentTaskDryRunFilter>('all')

const searchFilter = ref('')

const actionFilter = ref('')

const loading = ref(false)

const busy = ref(false)

const requestError = ref('')

const preview = ref<GuardedTaskPreviewResponse | null>(null)

const receipt = ref<GuardedTaskReceipt | null>(null)

const runResult = ref<WorkspaceTaskRunResponse | null>(null)

const replaySource = ref<RecentTaskRecord | null>(null)

const dialogOpen = ref(false)



function normalizeText(value: string | null | undefined) {

  return String(value ?? '').trim().toLowerCase()

}



function buildSearchText(entry: RecentTaskRecord) {

  return [

    entry.summary,

    entry.workspace,

    entry.action,

    entry.target,

    entry.audit_action,

    entry.process.command_line,

  ]

    .map(normalizeText)

    .join('\n')

}



function formatTime(ts: number) {

  return new Date(ts * 1000).toLocaleString()

}



function errorMessage(err: unknown): string {

  if (err instanceof Error && err.message.trim()) return err.message

  return '请求失败，请检查全局错误提示。'

}



const actionItems = computed(() =>
  Array.from(new Set(entries.value.map((entry) => entry.action).filter((action) => action.trim()))).sort(),
)

const activeFilterItems = computed(() => {
  const items: Array<{ key: string; label: string; value: string }> = []

  if (statusFilter.value !== 'all') {
    items.push({ key: 'status', label: '??', value: statusFilter.value })
  }

  const searchKeyword = searchFilter.value.trim()
  if (searchKeyword) {
    items.push({ key: 'search', label: '??', value: searchKeyword })
  }

  if (actionFilter.value) {
    items.push({ key: 'action', label: '??', value: actionFilter.value })
  }

  if (dryRunFilter.value !== 'all') {
    items.push({
      key: 'dry_run',
      label: 'Dry Run',
      value: dryRunFilter.value === 'dry-run' ? '? Dry Run' : '????',
    })
  }

  return items
})

const filteredEntries = computed(() =>

  entries.value.filter((entry) => {

    const searchKeyword = normalizeText(searchFilter.value)

    const matchesStatus = statusFilter.value === 'all' || entry.status === statusFilter.value

    const matchesDryRun =

      dryRunFilter.value === 'all'

        ? true

        : dryRunFilter.value === 'dry-run'

          ? entry.dry_run

          : !entry.dry_run

    const matchesAction = !actionFilter.value || entry.action === actionFilter.value

    const matchesSearch = !searchKeyword || buildSearchText(entry).includes(searchKeyword)

    return matchesStatus && matchesDryRun && matchesAction && matchesSearch

  }),

)



const selectedRecord = computed(

  () => filteredEntries.value.find((entry) => entry.id === selectedId.value) ?? filteredEntries.value[0] ?? null,

)

const canFocusDiagnosticsCenter = computed(() => Boolean(selectedRecord.value))



const replayLabel = computed(() => {

  if (!selectedRecord.value?.replay) return '不可重放'

  return selectedRecord.value.replay.kind === 'run' ? '重新执行' : '重新预演'

})



const selectedGovernanceContext = computed(() =>

  selectedRecord.value ? resolveRecentTaskGovernanceContext(selectedRecord.value) : null,

)



const replayResultGovernanceContext = computed(() =>

  replaySource.value && runResult.value

    ? resolveRecentTaskGovernanceContext(

        replaySource.value,

        runResult.value.process,

        'execute',

        runResult.value.details ?? null,

      )

    : null,

)



const receiptGovernanceContext = computed(() =>

  replaySource.value && receipt.value

    ? resolveRecentTaskGovernanceContext(

        replaySource.value,

        receipt.value.process,

        'execute',

        receipt.value.details ?? null,

      )

    : null,

)



async function loadRecentTasks() {

  loading.value = true

  requestError.value = ''

  try {

    const response = await fetchRecentWorkspaceTasks(props.limit, props.workspace || undefined)

    entries.value = response.entries

    stats.value = response.stats

    if (!entries.value.some((entry) => entry.id === selectedId.value)) {

      selectedId.value = entries.value[0]?.id ?? ''

    }

  } catch (err) {

    requestError.value = errorMessage(err)

  } finally {

    loading.value = false

  }

}



function selectRecord(id: string) {

  selectedId.value = id

  receipt.value = null

  runResult.value = null

  replaySource.value = null

}



function applyFocusRequest(request: RecentTasksFocusRequest | null | undefined) {

  if (!request) return

  statusFilter.value = request.status ?? 'all'

  dryRunFilter.value = request.dry_run ?? 'all'

  searchFilter.value = request.search ?? ''

  actionFilter.value = request.action ?? ''

  if (request.selected_task_id) {

    selectedId.value = request.selected_task_id

  }

  preview.value = null

  dialogOpen.value = false

  receipt.value = null

  runResult.value = null

  replaySource.value = null

}

function clearActiveFilters() {
  statusFilter.value = 'all'
  dryRunFilter.value = 'all'
  searchFilter.value = ''
  actionFilter.value = ''
  preview.value = null
  dialogOpen.value = false
  receipt.value = null
  runResult.value = null
  replaySource.value = null
}

function focusDiagnosticsCenterForSelectedRecord() {
  if (!selectedRecord.value) return
  emit('link-panel', {
    panel: 'diagnostics-center',
    request: resolveDiagnosticsCenterFocusFromRecentTask(selectedRecord.value),
  })
}

async function replaySelectedRecord() {

  if (!selectedRecord.value?.replay) return

  replaySource.value = selectedRecord.value

  busy.value = true

  requestError.value = ''

  receipt.value = null

  runResult.value = null

  try {

    if (selectedRecord.value.replay.kind === 'run') {

      runResult.value = await runWorkspaceTask(selectedRecord.value.replay.request)

      selectedId.value = ''

      await loadRecentTasks()

      return

    }

    preview.value = await previewGuardedTask(selectedRecord.value.replay.request)

    dialogOpen.value = true

  } catch (err) {

    requestError.value = errorMessage(err)

  } finally {

    busy.value = false

  }

}



async function confirmGuardedReplay() {

  if (!preview.value) return

  busy.value = true

  requestError.value = ''

  try {

    receipt.value = await executeGuardedTask({ token: preview.value.token, confirm: true })

    dialogOpen.value = false

    preview.value = null

    selectedId.value = ''

    await loadRecentTasks()

  } catch (err) {

    requestError.value = errorMessage(err)

  } finally {

    busy.value = false

  }

}



watch(

  () => props.focusRequest?.key,

  async () => {

    if (!props.focusRequest) return

    applyFocusRequest(props.focusRequest)

    await loadRecentTasks()

  },

)



onMounted(() => {

  void loadRecentTasks()

})

</script>



<template>

  <section class="recent-tasks" data-testid="recent-tasks-panel">

    <header class="recent-tasks__header">

      <div>

        <h3 class="recent-tasks__title">{{ props.title }}</h3>

        <p class="recent-tasks__desc">{{ props.description }}</p>

      </div>

      <div class="recent-tasks__actions">

        <Button data-testid="refresh-button" preset="secondary" :loading="loading" @click="loadRecentTasks">

          刷新

        </Button>

      </div>

    </header>



    <div class="recent-tasks__summary">

      <span class="recent-tasks__chip">总数 {{ stats?.total ?? 0 }}</span>

      <span class="recent-tasks__chip recent-tasks__chip--ok">成功 {{ stats?.succeeded ?? 0 }}</span>

      <span class="recent-tasks__chip recent-tasks__chip--error">失败 {{ stats?.failed ?? 0 }}</span>

      <span class="recent-tasks__chip">Dry Run {{ stats?.dry_run ?? 0 }}</span>

    </div>



    <div class="recent-tasks__filters">
      <label class="recent-tasks__filter">
        <span>??</span>
        <select v-model="statusFilter" data-testid="status-filter">
          <option value="all">??</option>
          <option value="succeeded">??</option>
          <option value="failed">??</option>
          <option value="previewed">??</option>
        </select>
      </label>
      <label class="recent-tasks__filter recent-tasks__filter--wide">
        <span>??</span>
        <input
          v-model="searchFilter"
          data-testid="recent-search-filter"
          type="text"
          placeholder="??? / ?? / ????"
        />
      </label>
      <label class="recent-tasks__filter">
        <span>??</span>
        <select v-model="actionFilter" data-testid="recent-action-filter">
          <option value="">??</option>
          <option v-for="action in actionItems" :key="action" :value="action">{{ action }}</option>
        </select>
      </label>
      <label class="recent-tasks__filter">
        <span>Dry Run</span>
        <select v-model="dryRunFilter" data-testid="dryrun-filter">
          <option value="all">??</option>
          <option value="dry-run">? Dry Run</option>
          <option value="executed">????</option>
        </select>
      </label>
    </div>

    <div v-if="activeFilterItems.length" class="recent-tasks__focus" data-testid="recent-active-filters">
      <span class="recent-tasks__focus-label">????</span>
      <span
        v-for="item in activeFilterItems"
        :key="item.key"
        class="recent-tasks__chip recent-tasks__chip--focus"
      >
        {{ item.label }}?{{ item.value }}
      </span>
      <Button data-testid="clear-recent-filters" size="sm" preset="secondary" @click="clearActiveFilters">
        ????
      </Button>
    </div>

    <p v-if="requestError" class="recent-tasks__error">{{ requestError }}</p>



    <div class="recent-tasks__layout">

      <div class="recent-tasks__list">

        <button

          v-for="entry in filteredEntries"

          :key="entry.id"

          :data-testid="`task-item-${entry.id}`"

          :class="['recent-tasks__item', selectedRecord?.id === entry.id ? 'is-active' : '']"

          type="button"

          @click="selectRecord(entry.id)"

        >

          <div class="recent-tasks__item-top">

            <strong>{{ entry.summary }}</strong>

            <span :class="['recent-tasks__badge', `is-${entry.status}`]">{{ entry.status }}</span>

          </div>

          <div class="recent-tasks__item-meta">

            <span>{{ entry.workspace }}</span>

            <span>{{ entry.phase }}</span>

            <span>{{ formatTime(entry.created_at) }}</span>

          </div>

        </button>

        <div v-if="!filteredEntries.length" class="recent-tasks__empty">暂无匹配任务。</div>

      </div>



      <section v-if="selectedRecord" class="recent-tasks__detail">

        <div class="recent-tasks__detail-header">

          <div>

            <h4 class="recent-tasks__detail-title">{{ selectedRecord.summary }}</h4>

            <p class="recent-tasks__detail-subtitle">

              {{ selectedRecord.workspace }} / {{ selectedRecord.action }} / {{ selectedRecord.target || '-' }}

            </p>

          </div>

          <span :class="['recent-tasks__badge', `is-${selectedRecord.status}`]">{{ selectedRecord.status }}</span>

        </div>



        <div class="recent-tasks__detail-meta">

          <div><strong>模式</strong> {{ selectedRecord.mode }}</div>

          <div><strong>阶段</strong> {{ selectedRecord.phase }}</div>

          <div><strong>Dry Run</strong> {{ selectedRecord.dry_run ? '是' : '否' }}</div>

          <div><strong>时间</strong> {{ formatTime(selectedRecord.created_at) }}</div>

          <div><strong>审计</strong> {{ selectedRecord.audit_action || '-' }}</div>

          <div><strong>耗时</strong> {{ selectedRecord.process.duration_ms }} ms</div>

        </div>



        <div class="recent-tasks__detail-actions">

          <Button

            data-testid="replay-button"

            preset="primary"

            :disabled="!selectedRecord.replay"

            :loading="busy"

            @click="replaySelectedRecord"

          >

            {{ replayLabel }}

          </Button>

          <Button

            data-testid="recent-link-diagnostics"

            preset="secondary"

            :disabled="!canFocusDiagnosticsCenter"

            @click="focusDiagnosticsCenterForSelectedRecord"

          >

            ??????

          </Button>

        </div>



        <FileGovernanceSummary

          v-if="selectedGovernanceContext"

          :task="selectedGovernanceContext.task"

          :form="selectedGovernanceContext.form"

          :phase="selectedGovernanceContext.phase"

          :process="selectedGovernanceContext.process"

          :details="selectedGovernanceContext.details"

        />



        <pre class="recent-tasks__output">{{ selectedRecord.process.command_line }}



{{ selectedRecord.process.stdout || selectedRecord.process.stderr || 'No command output' }}</pre>

      </section>

      <section v-else class="recent-tasks__detail recent-tasks__detail--empty">请选择一条任务查看详情。</section>

    </div>



    <div v-if="runResult" class="recent-tasks__result">

      <div class="recent-tasks__detail-header">

        <h4 class="recent-tasks__detail-title">重放结果</h4>

        <span :class="['recent-tasks__badge', runResult.process.success ? 'is-succeeded' : 'is-failed']">

          {{ runResult.process.success ? 'succeeded' : 'failed' }}

        </span>

      </div>

      <FileGovernanceSummary

        v-if="replayResultGovernanceContext"

        :task="replayResultGovernanceContext.task"

        :form="replayResultGovernanceContext.form"

        :phase="replayResultGovernanceContext.phase"

        :process="replayResultGovernanceContext.process"

        :details="replayResultGovernanceContext.details"

      />



      <pre class="recent-tasks__output">{{ runResult.process.command_line }}



{{ runResult.process.stdout || runResult.process.stderr || 'No command output' }}</pre>

    </div>



    <FileGovernanceSummary

      v-if="receiptGovernanceContext"

      :task="receiptGovernanceContext.task"

      :form="receiptGovernanceContext.form"

      :phase="receiptGovernanceContext.phase"

      :process="receiptGovernanceContext.process"

      :details="receiptGovernanceContext.details"

    />



    <TaskReceiptComponent v-if="receipt" :receipt="receipt" />



    <UnifiedConfirmDialog

      v-model="dialogOpen"

      title="重放危险任务"

      :preview="preview"

      :busy="busy"

      :confirm-disabled="!preview?.ready_to_execute"

      @confirm="confirmGuardedReplay"

    />

  </section>

</template>



<style scoped>

.recent-tasks {

  display: flex;

  flex-direction: column;

  gap: var(--space-4);

}



.recent-tasks__header,

.recent-tasks__item-top,

.recent-tasks__detail-header,

.recent-tasks__detail-actions {

  display: flex;

  justify-content: space-between;

  gap: var(--space-3);

  align-items: center;

}



.recent-tasks__title,

.recent-tasks__detail-title {

  font: var(--type-title);

  color: var(--text-primary);

}



.recent-tasks__desc,

.recent-tasks__detail-subtitle,

.recent-tasks__item-meta,

.recent-tasks__detail-meta {

  color: var(--text-secondary);

  font: var(--type-body-sm);

}



.recent-tasks__summary,

.recent-tasks__filters,

.recent-tasks__detail-meta,

.recent-tasks__item-meta {

  display: flex;

  flex-wrap: wrap;

  gap: var(--space-2);

}



.recent-tasks__filter {

  display: flex;

  flex-direction: column;

  gap: var(--space-1);

}



.recent-tasks__filter--wide {

  min-width: min(320px, 100%);

  flex: 1 1 280px;

}

.recent-tasks__focus {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: var(--space-2);
}

.recent-tasks__focus-label {
  color: var(--text-secondary);
  font: var(--type-caption);
}

.recent-tasks__chip--focus {
  background: var(--color-info-bg);
  color: var(--color-info);
}



.recent-tasks__layout {

  display: grid;

  grid-template-columns: minmax(320px, 360px) minmax(0, 1fr);

  gap: var(--space-4);

}



.recent-tasks__list,

.recent-tasks__detail,

.recent-tasks__result {

  border: var(--card-border);

  border-radius: var(--card-radius);

  background: var(--surface-card);

  box-shadow: var(--card-shadow);

  padding: var(--card-padding);

}



.recent-tasks__list {

  display: flex;

  flex-direction: column;

  gap: var(--space-2);

}



.recent-tasks__item {

  text-align: left;

  border: var(--border);

  border-radius: var(--radius-md);

  background: var(--surface-panel);

  padding: var(--space-3);

  color: inherit;

  cursor: pointer;

}



.recent-tasks__item.is-active {

  border-color: var(--text-secondary);

  background: var(--ds-background-2);

}



.recent-tasks__badge,

.recent-tasks__chip {

  display: inline-flex;

  align-items: center;

  padding: 2px var(--space-3);

  border-radius: var(--radius-full);

  background: var(--ds-background-2);

  color: var(--text-secondary);

  font: var(--type-caption);

}



.recent-tasks__chip--ok,

.recent-tasks__badge.is-succeeded {

  background: var(--color-success-bg);

  color: var(--color-success);

}



.recent-tasks__chip--error,

.recent-tasks__badge.is-failed {

  background: var(--color-danger-bg);

  color: var(--color-danger);

}



.recent-tasks__badge.is-previewed {

  background: var(--color-info-bg);

  color: var(--color-info);

}



.recent-tasks__detail,

.recent-tasks__result {

  display: flex;

  flex-direction: column;

  gap: var(--space-3);

}



.recent-tasks__detail--empty,

.recent-tasks__empty,

.recent-tasks__error {

  color: var(--text-secondary);

}



.recent-tasks__error {

  color: var(--color-danger);

}



.recent-tasks__output {

  border: var(--border);

  border-radius: var(--radius-md);

  background: var(--ds-background-2);

  padding: var(--space-4);

  white-space: pre-wrap;

  word-break: break-word;

  color: var(--text-primary);

}

</style>

