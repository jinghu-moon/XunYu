<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import {
  executeGuardedTask,
  previewGuardedTask,
  runWorkspaceTask,
} from '../api'
import type {
  GuardedTaskPreviewResponse,
  GuardedTaskReceipt,
  StatisticsWorkspaceLinkPayload,
  WorkspaceCapabilities,
  WorkspaceTaskRunResponse,
} from '../types'
import type { TaskFieldDefinition, TaskFieldValue, TaskFormState, WorkspaceTaskDefinition } from '../workspace-tools'
import { Button } from './button'
import TaskReceiptComponent from './TaskReceiptComponent.vue'
import UnifiedConfirmDialog from './UnifiedConfirmDialog.vue'
import FileGovernanceSummary from './FileGovernanceSummary.vue'

type TaskExecutionState =
  | 'idle'
  | 'previewing'
  | 'awaiting_confirm'
  | 'running'
  | 'succeeded'
  | 'failed'

const emit = defineEmits<{
  (event: 'link-panel', payload: StatisticsWorkspaceLinkPayload): void
}>()

const props = withDefaults(
  defineProps<{
    task: WorkspaceTaskDefinition
    capabilities?: WorkspaceCapabilities | null
    initialValues?: Partial<TaskFormState> | null
    presetVersion?: number
  }>(),
  {
    capabilities: null,
    initialValues: null,
    presetVersion: 0,
  },
)

function createInitialState(fields: TaskFieldDefinition[]): TaskFormState {
  return fields.reduce<TaskFormState>((state, field) => {
    state[field.key] = field.defaultValue ?? (field.type === 'checkbox' ? false : '')
    return state
  }, {})
}

function errorMessage(err: unknown): string {
  if (err instanceof Error && err.message.trim()) return err.message
  return '请求失败，请检查全局错误提示。'
}

const form = reactive(createInitialState(props.task.fields))

function applyInitialValues() {
  if (!props.initialValues) return
  for (const field of props.task.fields) {
    if (!Object.prototype.hasOwnProperty.call(props.initialValues, field.key)) continue
    const nextValue = props.initialValues[field.key]
    if (nextValue !== undefined) {
      form[field.key] = nextValue
    }
  }
  validationError.value = ''
}

const runBusy = ref(false)
const previewBusy = ref(false)
const executeBusy = ref(false)
const dialogOpen = ref(false)
const state = ref<TaskExecutionState>('idle')
const preview = ref<GuardedTaskPreviewResponse | null>(null)
const receipt = ref<GuardedTaskReceipt | null>(null)
const result = ref<WorkspaceTaskRunResponse | null>(null)
const validationError = ref('')
const requestError = ref('')

const isSupported = computed(() => {
  if (!props.task.feature || !props.capabilities) return true
  return props.capabilities[props.task.feature] !== false
})

const actionLabel = computed(() => (props.task.mode === 'guarded' ? '预演并确认' : '运行'))
const processOutput = computed(() => result.value?.process ?? null)
const previewOutput = computed(() => preview.value?.process ?? null)
const stateLabel = computed(() => {
  switch (state.value) {
    case 'previewing':
      return '预演中'
    case 'awaiting_confirm':
      return '待确认'
    case 'running':
      return '执行中'
    case 'succeeded':
      return '成功'
    case 'failed':
      return '失败'
    default:
      return '待执行'
  }
})
const stateTone = computed(() => {
  if (state.value === 'succeeded') return 'is-ok'
  if (state.value === 'failed') return 'is-error'
  return ''
})


function emitRecentTasksLink(payload: {
  status: 'succeeded' | 'failed'
  dryRun: 'dry-run' | 'executed'
  search?: string
  action?: string
}) {
  emit('link-panel', {
    panel: 'recent-tasks',
    request: {
      status: payload.status,
      dry_run: payload.dryRun,
      search: payload.search,
      action: payload.action,
    },
  })
}

function emitAuditLink(payload: { search?: string; action?: string; result: 'success' | 'failed' | 'dry_run' }) {
  emit('link-panel', {
    panel: 'audit',
    request: {
      search: payload.search,
      action: payload.action,
      result: payload.result,
    },
  })
}

function focusRecentTasksForResult() {
  if (!processOutput.value) return
  emitRecentTasksLink({
    status: processOutput.value.success ? 'succeeded' : 'failed',
    dryRun: 'executed',
    search: result.value?.target || undefined,
    action: result.value?.action || props.task.action,
  })
}

function focusAuditForResult() {
  if (!result.value) return
  emitAuditLink({
    search: result.value.target || undefined,
    result: result.value.process.success ? 'success' : 'failed',
  })
}

function focusRecentTasksForReceipt() {
  if (!receipt.value) return
  emitRecentTasksLink({
    status: receipt.value.status,
    dryRun: receipt.value.dry_run ? 'dry-run' : 'executed',
    search: receipt.value.target || undefined,
    action: receipt.value.action,
  })
}

function focusAuditForReceipt() {
  if (!receipt.value) return
  emitAuditLink({
    search: receipt.value.target || undefined,
    action: receipt.value.audit_action || undefined,
    result: receipt.value.status === 'failed' ? 'failed' : 'success',
  })
}

watch(
  () => props.presetVersion,
  () => {
    applyInitialValues()
  },
  { immediate: true },
)

function isFieldEmpty(field: TaskFieldDefinition): boolean {
  const value = form[field.key] as TaskFieldValue
  if (field.type === 'checkbox') return value !== true
  return typeof value !== 'string' || !value.trim()
}

function validate() {
  const missing = props.task.fields.filter((field) => field.required && isFieldEmpty(field))
  validationError.value = missing.length ? `缺少必填项：${missing.map((field) => field.label).join('、')}` : ''
  return missing.length === 0
}

async function runTask() {
  if (!validate() || !props.task.buildRunArgs) return
  runBusy.value = true
  state.value = 'running'
  requestError.value = ''
  receipt.value = null
  preview.value = null
  try {
    result.value = await runWorkspaceTask({
      workspace: props.task.workspace,
      action: props.task.action,
      target: props.task.target?.(form) ?? '',
      args: props.task.buildRunArgs(form),
    })
    state.value = result.value.process.success ? 'succeeded' : 'failed'
  } catch (err) {
    state.value = 'failed'
    requestError.value = errorMessage(err)
  } finally {
    runBusy.value = false
  }
}

async function previewTask() {
  if (!validate() || !props.task.buildPreviewArgs || !props.task.buildExecuteArgs) return
  previewBusy.value = true
  state.value = 'previewing'
  requestError.value = ''
  result.value = null
  receipt.value = null
  try {
    preview.value = await previewGuardedTask({
      workspace: props.task.workspace,
      action: props.task.action,
      target: props.task.target?.(form) ?? '',
      preview_args: props.task.buildPreviewArgs(form),
      execute_args: props.task.buildExecuteArgs(form),
      preview_summary: props.task.previewSummary?.(form) ?? '',
    })
    state.value = 'awaiting_confirm'
    dialogOpen.value = true
  } catch (err) {
    preview.value = null
    dialogOpen.value = false
    state.value = 'failed'
    requestError.value = errorMessage(err)
  } finally {
    previewBusy.value = false
  }
}

async function confirmTask() {
  if (!preview.value) return
  executeBusy.value = true
  state.value = 'running'
  requestError.value = ''
  try {
    receipt.value = await executeGuardedTask({ token: preview.value.token, confirm: true })
    dialogOpen.value = false
    preview.value = null
    state.value = receipt.value.process.success ? 'succeeded' : 'failed'
  } catch (err) {
    preview.value = null
    dialogOpen.value = false
    state.value = 'failed'
    requestError.value = errorMessage(err)
  } finally {
    executeBusy.value = false
  }
}
</script>

<template>
  <article :class="['task-card', props.task.tone === 'danger' ? 'task-card--danger' : '']">
    <header class="task-card__header">
      <div>
        <h4 class="task-card__title">{{ props.task.title }}</h4>
        <p class="task-card__desc">{{ props.task.description }}</p>
      </div>
      <div class="task-card__header-side">
        <span :class="['task-card__badge', stateTone]">{{ stateLabel }}</span>
        <span v-if="props.task.feature" class="task-card__feature">{{ props.task.feature }}</span>
      </div>
    </header>

    <div v-if="props.task.fields.length" class="task-card__form">
      <label v-for="field in props.task.fields" :key="field.key" class="task-card__field">
        <span class="task-card__label">{{ field.label }}</span>
        <textarea
          v-if="field.type === 'textarea'"
          :value="String(form[field.key] ?? '')"
          class="task-card__textarea"
          :placeholder="field.placeholder"
          @input="form[field.key] = ($event.target as HTMLTextAreaElement).value"
        />
        <select
          v-else-if="field.type === 'select'"
          :value="String(form[field.key] ?? '')"
          class="task-card__input"
          @change="form[field.key] = ($event.target as HTMLSelectElement).value"
        >
          <option v-for="option in field.options || []" :key="option.value" :value="option.value">
            {{ option.label }}
          </option>
        </select>
        <input
          v-else-if="field.type === 'checkbox'"
          :checked="form[field.key] === true"
          type="checkbox"
          class="task-card__checkbox"
          @change="form[field.key] = ($event.target as HTMLInputElement).checked"
        />
        <input
          v-else
          :value="String(form[field.key] ?? '')"
          :type="field.type === 'number' ? 'number' : 'text'"
          class="task-card__input"
          :min="field.min"
          :max="field.max"
          :placeholder="field.placeholder"
          @input="form[field.key] = ($event.target as HTMLInputElement).value"
        />
        <small v-if="field.help" class="task-card__help">{{ field.help }}</small>
      </label>
    </div>

    <div class="task-card__actions">
      <Button
        :preset="props.task.tone === 'danger' ? 'danger' : 'primary'"
        :disabled="!isSupported"
        :loading="props.task.mode === 'guarded' ? previewBusy : runBusy"
        @click="props.task.mode === 'guarded' ? previewTask() : runTask()"
      >
        {{ actionLabel }}
      </Button>
      <span v-if="!isSupported" class="task-card__hint">当前构建未启用该 feature。</span>
      <span v-else-if="validationError" class="task-card__hint task-card__hint--error">{{ validationError }}</span>
      <span v-else-if="requestError" class="task-card__hint task-card__hint--error">{{ requestError }}</span>
    </div>

    <div v-if="preview && previewOutput" class="task-card__result">
      <div class="task-card__result-meta">
        <span :class="['task-card__badge', preview.ready_to_execute ? 'is-ok' : 'is-error']">
          {{ preview.ready_to_execute ? '预演通过' : '预演失败' }}
        </span>
        <span>{{ preview.summary }}</span>
      </div>
      <FileGovernanceSummary :task="props.task" :form="form" phase="preview" :process="previewOutput" :details="preview.details" />

      <pre class="task-card__output">{{ previewOutput.command_line }}

{{ previewOutput.stdout || previewOutput.stderr || '暂无输出' }}</pre>
    </div>

    <div v-if="processOutput" class="task-card__result">
      <div class="task-card__result-meta">
        <span :class="['task-card__badge', processOutput.success ? 'is-ok' : 'is-error']">
          {{ processOutput.success ? '鎴愬姛' : '澶辫触' }}
        </span>
        <span>{{ processOutput.duration_ms }} ms</span>
      </div>
      <FileGovernanceSummary :task="props.task" :form="form" phase="execute" :process="processOutput" :details="result?.details ?? null" />

      <div class="task-card__result-links">
        <button data-testid="task-card-link-recent" class="task-card__link" type="button" @click="focusRecentTasksForResult">
          回到最近任务
        </button>
        <button data-testid="task-card-link-audit" class="task-card__link" type="button" @click="focusAuditForResult">
          查看审计
        </button>
      </div>

      <pre class="task-card__output">{{ processOutput.command_line }}

{{ processOutput.stdout || processOutput.stderr || '暂无输出' }}</pre>
    </div>

    <FileGovernanceSummary
      v-if="receipt"
      :task="props.task"
      :form="form"
      phase="execute"
      :process="receipt.process"
      :details="receipt.details"
    />

    <div v-if="receipt" class="task-card__result-links">
      <button data-testid="task-card-link-recent-receipt" class="task-card__link" type="button" @click="focusRecentTasksForReceipt">
        回到最近任务
      </button>
      <button data-testid="task-card-link-audit-receipt" class="task-card__link" type="button" @click="focusAuditForReceipt">
        查看审计
      </button>
    </div>

    <TaskReceiptComponent v-if="receipt" :receipt="receipt" />

    <UnifiedConfirmDialog
      v-model="dialogOpen"
      :title="props.task.title"
      :preview="preview"
      :busy="executeBusy"
      :confirm-disabled="!preview?.ready_to_execute"
      @confirm="confirmTask"
    >
      <template #preview-extra>
        <FileGovernanceSummary
          v-if="preview && previewOutput"
          :task="props.task"
          :form="form"
          phase="preview"
          :process="previewOutput"
          :details="preview.details"
        />
      </template>
    </UnifiedConfirmDialog>
  </article>
</template>

<style scoped>
.task-card {
  border: var(--card-border);
  border-radius: var(--card-radius);
  background: var(--surface-card);
  box-shadow: var(--card-shadow);
  padding: var(--card-padding);
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}

.task-card--danger {
  border-color: rgba(255, 79, 79, 0.3);
}

.task-card__header {
  display: flex;
  justify-content: space-between;
  gap: var(--space-3);
}

.task-card__header-side {
  display: flex;
  align-items: flex-start;
  gap: var(--space-2);
  flex-wrap: wrap;
  justify-content: flex-end;
}

.task-card__title {
  font: var(--type-title-sm);
  color: var(--text-primary);
  margin-bottom: var(--space-1);
}

.task-card__desc {
  color: var(--text-secondary);
  font: var(--type-body-sm);
}

.task-card__feature {
  border-radius: var(--radius-full);
  background: var(--ds-background-2);
  padding: 2px var(--space-3);
  color: var(--text-secondary);
  font: var(--type-caption);
  height: fit-content;
}

.task-card__form {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
  gap: var(--space-3);
}

.task-card__field {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}

.task-card__label {
  color: var(--text-secondary);
  font: var(--type-caption);
}

.task-card__input,
.task-card__textarea {
  width: 100%;
}

.task-card__textarea {
  min-height: 88px;
  padding: var(--comp-padding-sm);
  border: var(--border);
  border-radius: var(--radius-sm);
  resize: vertical;
  background: var(--surface-panel);
  color: var(--text-primary);
}

.task-card__checkbox {
  width: 18px;
  height: 18px;
}

.task-card__help,
.task-card__hint {
  color: var(--text-tertiary);
  font: var(--type-caption);
}

.task-card__hint--error {
  color: var(--color-danger);
}

.task-card__actions {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  flex-wrap: wrap;
}

.task-card__result {
  border-top: var(--border);
  padding-top: var(--space-3);
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}

.task-card__result-links {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
}

.task-card__link {
  padding: 0;
  border: none;
  background: transparent;
  color: var(--color-primary);
  cursor: pointer;
  font: var(--type-caption);
}

.task-card__result-meta {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  color: var(--text-secondary);
  font: var(--type-caption);
}

.task-card__badge {
  display: inline-flex;
  align-items: center;
  padding: 2px var(--space-3);
  border-radius: var(--radius-full);
  font-weight: var(--weight-semibold);
  background: var(--ds-background-2);
  color: var(--text-secondary);
}

.task-card__badge.is-ok {
  background: var(--color-success-bg);
  color: var(--color-success);
}

.task-card__badge.is-error {
  background: var(--color-danger-bg);
  color: var(--color-danger);
}

.task-card__output {
  border: var(--border);
  border-radius: var(--radius-md);
  background: var(--ds-background-2);
  padding: var(--space-4);
  white-space: pre-wrap;
  word-break: break-word;
  color: var(--text-primary);
}
</style>
