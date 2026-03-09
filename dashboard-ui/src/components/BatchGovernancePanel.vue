<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import { executeGuardedTask, previewGuardedTask } from '../api'
import type {
  GuardedTaskPreviewResponse,
  RecentTasksFocusRequest,
  StatisticsWorkspaceLinkPayload,
  WorkspaceCapabilities,
} from '../types'
import type { TaskFieldDefinition, TaskFieldValue, TaskFormState } from '../workspace-tools'
import {
  buildBatchGovernancePlan,
  createAuditLinkFromBatchReceipt,
  createBatchGovernanceDialogPreview,
  createBatchGovernanceItemForm,
  createBatchGovernancePreviewRequests,
  createBatchGovernanceSharedState,
  createDiagnosticsLinkFromBatchPreview,
  createDiagnosticsLinkFromBatchReceipt,
  createRecentTasksFocusFromBatchPreview,
  createRecentTasksFocusFromBatchReceipt,
  getBatchGovernanceAction,
  getBatchGovernanceActions,
  getBatchGovernanceSharedFields,
  isBatchPreviewReady,
  normalizeBatchPaths,
  summarizeBatchPreviews,
  summarizeBatchReceipts,
  type BatchGovernanceActionId,
  type BatchGovernancePreviewItem,
  type BatchGovernanceReceiptItem,
} from './file-governance-batch'
import UnifiedConfirmDialog from './UnifiedConfirmDialog.vue'
import FileGovernanceSummary from './FileGovernanceSummary.vue'
import { Button } from './button'

const emit = defineEmits<{
  (event: 'focus-recent-tasks', request: Omit<RecentTasksFocusRequest, 'key'>): void
  (event: 'link-panel', payload: StatisticsWorkspaceLinkPayload): void
}>()

const props = withDefaults(
  defineProps<{
    paths?: string[]
    capabilities?: WorkspaceCapabilities | null
  }>(),
  {
    paths: () => [],
    capabilities: null,
  },
)

const actions = getBatchGovernanceActions()
const actionId = ref<BatchGovernanceActionId>('protect-set')
const form = reactive<TaskFormState>({})
const previewBusy = ref(false)
const executeBusy = ref(false)
const dialogOpen = ref(false)
const requestError = ref('')
const previewItems = ref<BatchGovernancePreviewItem[]>([])
const receiptItems = ref<BatchGovernanceReceiptItem[]>([])

const batchPaths = computed(() => normalizeBatchPaths(props.paths))
const currentAction = computed(() => getBatchGovernanceAction(actionId.value))
const currentFields = computed(() => getBatchGovernanceSharedFields(actionId.value))
const isSupported = computed(() => {
  const feature = currentAction.value.task.feature
  if (!feature) return true
  return props.capabilities?.[feature] !== false
})
const previewStats = computed(() => summarizeBatchPreviews(previewItems.value))
const receiptStats = computed(() => summarizeBatchReceipts(receiptItems.value))
const canConfirm = computed(() => isBatchPreviewReady(previewItems.value))
const batchPlan = computed(() => buildBatchGovernancePlan(actionId.value, batchPaths.value, form))
const dialogPreview = computed<GuardedTaskPreviewResponse | null>(() => {
  if (!previewItems.value.length) return null
  return createBatchGovernanceDialogPreview(actionId.value, previewItems.value)
})

function resetFormState() {
  const defaults = createBatchGovernanceSharedState(actionId.value)
  for (const key of Object.keys(form)) {
    delete form[key]
  }
  Object.assign(form, defaults)
}

function resetRunState() {
  requestError.value = ''
  previewItems.value = []
  receiptItems.value = []
  dialogOpen.value = false
}

function updateAction(value: string) {
  if (actions.some((action) => action.id === value)) {
    actionId.value = value as BatchGovernanceActionId
  }
}

function focusPreviewInRecentTasks(item: BatchGovernancePreviewItem) {
  emit('focus-recent-tasks', createRecentTasksFocusFromBatchPreview(actionId.value, item))
}

function focusReceiptInRecentTasks(item: BatchGovernanceReceiptItem) {
  emit('focus-recent-tasks', createRecentTasksFocusFromBatchReceipt(actionId.value, item))
}

function openPreviewDiagnostics(item: BatchGovernancePreviewItem) {
  emit('link-panel', createDiagnosticsLinkFromBatchPreview(actionId.value, item))
}

function openReceiptAudit(item: BatchGovernanceReceiptItem) {
  emit('link-panel', createAuditLinkFromBatchReceipt(actionId.value, item))
}

function openReceiptDiagnostics(item: BatchGovernanceReceiptItem) {
  emit('link-panel', createDiagnosticsLinkFromBatchReceipt(actionId.value, item))
}

function errorMessage(err: unknown): string {
  if (err instanceof Error && err.message.trim()) return err.message
  return '请求失败，请检查全局错误提示。'
}

function isFieldEmpty(field: TaskFieldDefinition): boolean {
  const value = form[field.key] as TaskFieldValue
  if (field.type === 'checkbox') return value !== true
  return typeof value !== 'string' || !value.trim()
}

function validate(): boolean {
  const missing = currentFields.value.filter((field) => field.required && isFieldEmpty(field))
  requestError.value = missing.length ? `缺少必填项：${missing.map((field) => field.label).join('、')}` : ''
  return missing.length === 0
}

async function previewBatch() {
  if (!batchPaths.value.length || !isSupported.value || !validate()) return

  previewBusy.value = true
  requestError.value = ''
  receiptItems.value = []
  dialogOpen.value = false

  try {
    const requests = createBatchGovernancePreviewRequests(actionId.value, batchPaths.value, form)
    const itemForms = batchPaths.value.map((path) => createBatchGovernanceItemForm(actionId.value, path, form))
    const results = await Promise.allSettled(requests.map((payload) => previewGuardedTask(payload)))

    previewItems.value = results.map((result, index) => {
      if (result.status === 'fulfilled') {
        return { path: batchPaths.value[index] ?? '', form: itemForms[index], preview: result.value }
      }
      return { path: batchPaths.value[index] ?? '', form: itemForms[index], error: errorMessage(result.reason) }
    })

    dialogOpen.value = true
  } catch (err) {
    previewItems.value = []
    requestError.value = errorMessage(err)
  } finally {
    previewBusy.value = false
  }
}

async function confirmBatch() {
  if (!canConfirm.value) return

  executeBusy.value = true
  requestError.value = ''

  try {
    const nextReceipts: BatchGovernanceReceiptItem[] = []

    for (const item of previewItems.value) {
      if (!item.preview?.ready_to_execute) {
        nextReceipts.push({ path: item.path, form: item.form, error: item.error || '预演未就绪，已阻止执行。' })
        continue
      }

      try {
        const receipt = await executeGuardedTask({ token: item.preview.token, confirm: true })
        nextReceipts.push({ path: item.path, form: item.form, receipt })
      } catch (err) {
        nextReceipts.push({ path: item.path, form: item.form, error: errorMessage(err) })
      }
    }

    receiptItems.value = nextReceipts
    dialogOpen.value = false
  } finally {
    executeBusy.value = false
  }
}

watch(
  actionId,
  () => {
    resetFormState()
    resetRunState()
  },
  { immediate: true },
)

watch(
  () => batchPaths.value.join('\n'),
  () => {
    resetRunState()
  },
)
</script>

<template>
  <section class="batch-governance" data-testid="batch-governance-panel">
    <header class="batch-governance__header">
      <div>
        <h3 class="batch-governance__title">批量治理</h3>
        <p class="batch-governance__desc">
当前已纳入 `protect:set / protect:clear`、`encrypt / decrypt` 以及 `acl:add / copy / restore / purge / inherit / owner / repair`。流程保持 Triple-Guard：先逐项 dry-run，再统一确认，最后输出逐项回执，并可直达最近任务、审计与诊断消费层。
        </p>
      </div>
      <span class="batch-governance__badge">{{ batchPaths.length }} 项</span>
    </header>

    <p v-if="!batchPaths.length" class="batch-governance__empty">
      先把文件加入批量队列，再从这里统一预演和执行治理动作。
    </p>
    <template v-else>
      <div class="batch-governance__meta">
        <span>治理动作</span>
        <select
          class="batch-governance__select"
          data-testid="batch-governance-action"
          :value="actionId"
          @change="updateAction(($event.target as HTMLSelectElement).value)"
        >
          <option v-for="action in actions" :key="action.id" :value="action.id">
            {{ action.label }}
          </option>
        </select>
      </div>

      <p class="batch-governance__hint">{{ currentAction.description }}</p>

      <div v-if="currentFields.length" class="batch-governance__form">
        <label v-for="field in currentFields" :key="field.key" class="batch-governance__field">
          <span class="batch-governance__label">{{ field.label }}</span>
          <textarea
            v-if="field.type === 'textarea'"
            :value="String(form[field.key] ?? '')"
            class="batch-governance__textarea"
            :data-testid="`batch-field-${field.key}`"
            :placeholder="field.placeholder"
            @input="form[field.key] = ($event.target as HTMLTextAreaElement).value"
          />
          <select
            v-else-if="field.type === 'select'"
            :value="String(form[field.key] ?? '')"
            class="batch-governance__select"
            :data-testid="`batch-field-${field.key}`"
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
            class="batch-governance__checkbox"
            :data-testid="`batch-field-${field.key}`"
            @change="form[field.key] = ($event.target as HTMLInputElement).checked"
          />
          <input
            v-else
            :value="String(form[field.key] ?? '')"
            :type="field.type === 'number' ? 'number' : 'text'"
            class="batch-governance__input"
            :data-testid="`batch-field-${field.key}`"
            :min="field.min"
            :max="field.max"
            :placeholder="field.placeholder"
            @input="form[field.key] = ($event.target as HTMLInputElement).value"
          />
          <small v-if="field.help" class="batch-governance__help">{{ field.help }}</small>
        </label>
      </div>

      <div class="batch-governance__actions">
        <Button
          data-testid="batch-governance-preview"
          preset="danger"
          :disabled="!isSupported"
          :loading="previewBusy"
          @click="previewBatch"
        >
          批量预演并确认
        </Button>
        <span v-if="!isSupported" class="batch-governance__message">当前构建未启用该治理能力。</span>
        <span v-else class="batch-governance__message">会对 {{ batchPaths.length }} 项路径逐项生成 dry-run 结果。</span>
      </div>


      <section class="batch-governance__section" data-testid="batch-governance-plan">
        <header class="batch-governance__section-header">
          <div>
            <h4 class="batch-governance__section-title">{{ batchPlan.title }}</h4>
            <p v-if="batchPlan.note" class="batch-governance__section-desc">{{ batchPlan.note }}</p>
          </div>
        </header>

        <dl class="batch-governance__plan-grid">
          <div v-for="item in batchPlan.items" :key="item.label" class="batch-governance__plan-item">
            <dt>{{ item.label }}</dt>
            <dd>{{ item.value }}</dd>
          </div>
        </dl>
      </section>
    </template>

    <p v-if="requestError" class="batch-governance__message batch-governance__message--error">{{ requestError }}</p>

    <section v-if="previewItems.length" class="batch-governance__section">
      <header class="batch-governance__section-header">
        <div>
          <h4 class="batch-governance__section-title">批量预演总览</h4>
          <p class="batch-governance__section-desc">
            已通过 {{ previewStats.ready }} / {{ previewStats.total }} 项；未通过 {{ previewStats.blocked }} 项。
          </p>
        </div>
      </header>

      <div class="batch-governance__list">
        <article
          v-for="item in previewItems"
          :key="`preview-${item.path}`"
          class="batch-governance__item"
          data-testid="batch-preview-item"
        >
          <div class="batch-governance__item-header">
            <strong class="batch-governance__item-path">{{ item.path }}</strong>
            <span :class="['batch-governance__item-badge', item.preview?.ready_to_execute ? 'is-ok' : 'is-error']">
              {{ item.preview?.ready_to_execute ? '已就绪' : '阻塞' }}
            </span>
          </div>
          <p v-if="item.error" class="batch-governance__message batch-governance__message--error">{{ item.error }}</p>
          <template v-else-if="item.preview">
            <p class="batch-governance__message">{{ item.preview.summary }}</p>
            <FileGovernanceSummary
              v-if="item.form"
              :task="currentAction.task"
              :form="item.form"
              phase="preview"
              :process="item.preview.process"
              :details="item.preview.details"
            />
            <div class="batch-governance__item-actions">
              <Button data-testid="batch-preview-link-recent" preset="secondary" @click="focusPreviewInRecentTasks(item)">
                回到最近任务
              </Button>
              <Button data-testid="batch-preview-link-diagnostics" preset="secondary" @click="openPreviewDiagnostics(item)">
                进入诊断中心
              </Button>
            </div>
            <details class="batch-governance__details">
              <summary>查看预演输出</summary>
              <pre class="batch-governance__output">{{ item.preview.process.command_line }}

{{ item.preview.process.stdout || item.preview.process.stderr || '暂无预演输出' }}</pre>
            </details>
          </template>
        </article>
      </div>
    </section>

    <section v-if="receiptItems.length" class="batch-governance__section">
      <header class="batch-governance__section-header">
        <div>
          <h4 class="batch-governance__section-title">批量执行回执</h4>
          <p class="batch-governance__section-desc">
            成功 {{ receiptStats.succeeded }} / {{ receiptStats.total }} 项；失败 {{ receiptStats.failed }} 项。
          </p>
        </div>
      </header>

      <div class="batch-governance__list">
        <article
          v-for="item in receiptItems"
          :key="`receipt-${item.path}`"
          class="batch-governance__item"
          data-testid="batch-receipt-item"
        >
          <div class="batch-governance__item-header">
            <strong class="batch-governance__item-path">{{ item.path }}</strong>
            <span :class="['batch-governance__item-badge', item.receipt?.process.success ? 'is-ok' : 'is-error']">
              {{ item.receipt?.process.success ? '成功' : '失败' }}
            </span>
          </div>
          <p v-if="item.error" class="batch-governance__message batch-governance__message--error">{{ item.error }}</p>
          <template v-else-if="item.receipt">
            <FileGovernanceSummary
              v-if="item.form"
              :task="currentAction.task"
              :form="item.form"
              phase="execute"
              :process="item.receipt.process"
              :details="item.receipt.details"
            />
            <div class="batch-governance__receipt-meta">
              <span>{{ item.receipt.audit_action }}</span>
              <span>{{ item.receipt.token }}</span>
              <span>{{ item.receipt.process.duration_ms }} ms</span>
            </div>
            <div class="batch-governance__item-actions">
              <Button data-testid="batch-receipt-link-recent" preset="secondary" @click="focusReceiptInRecentTasks(item)">
                回到最近任务
              </Button>
              <Button data-testid="batch-receipt-link-audit" preset="secondary" @click="openReceiptAudit(item)">
                查看审计
              </Button>
              <Button data-testid="batch-receipt-link-diagnostics" preset="secondary" @click="openReceiptDiagnostics(item)">
                进入诊断中心
              </Button>
            </div>
            <details class="batch-governance__details">
              <summary>查看执行回执</summary>
              <pre class="batch-governance__output">{{ item.receipt.process.command_line }}

{{ item.receipt.process.stdout || item.receipt.process.stderr || '暂无执行输出' }}</pre>
            </details>
          </template>
        </article>
      </div>
    </section>

    <UnifiedConfirmDialog
      v-model="dialogOpen"
      :title="currentAction.label"
      :warning="`将对 ${batchPaths.length} 项路径执行 ${currentAction.label}，请确认每项预演结果。`"
      :preview="dialogPreview"
      :busy="executeBusy"
      :confirm-disabled="!canConfirm"
      @confirm="confirmBatch"
    >
      <template #preview-extra>
        <div v-if="previewItems.length" data-testid="batch-preview-dialog-list" class="batch-governance__dialog-list">
          <article
            v-for="item in previewItems"
            :key="`dialog-preview-${item.path}`"
            class="batch-governance__item"
            data-testid="batch-preview-dialog-item"
          >
            <div class="batch-governance__item-header">
              <strong class="batch-governance__item-path">{{ item.path }}</strong>
              <span :class="['batch-governance__item-badge', item.preview?.ready_to_execute ? 'is-ok' : 'is-error']">
                {{ item.preview?.ready_to_execute ? '就绪' : '阻塞' }}
              </span>
            </div>
            <p v-if="item.error" class="batch-governance__message batch-governance__message--error">{{ item.error }}</p>
            <FileGovernanceSummary
              v-else-if="item.preview && item.form"
              :task="currentAction.task"
              :form="item.form"
              phase="preview"
              :process="item.preview.process"
              :details="item.preview.details"
            />
            <p v-else class="batch-governance__message">暂无可展示的预演摘要。</p>
          </article>
        </div>
      </template>
    </UnifiedConfirmDialog>
  </section>
</template>

<style scoped>
.batch-governance {
  border: var(--border);
  border-radius: var(--radius-md);
  background: var(--surface-panel);
  padding: var(--space-4);
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}

.batch-governance__header,
.batch-governance__section-header,
.batch-governance__item-header,
.batch-governance__meta {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: var(--space-3);
}

.batch-governance__title,
.batch-governance__section-title {
  font: var(--type-title-sm);
  color: var(--text-primary);
}

.batch-governance__item-actions {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
}

.batch-governance__desc,
.batch-governance__section-desc,
.batch-governance__message,
.batch-governance__hint,
.batch-governance__empty,
.batch-governance__receipt-meta {
  color: var(--text-secondary);
  font: var(--type-body-sm);
}

.batch-governance__badge,
.batch-governance__item-badge {
  display: inline-flex;
  align-items: center;
  padding: 2px var(--space-3);
  border-radius: var(--radius-full);
  background: var(--ds-background-2);
  color: var(--text-secondary);
  font: var(--type-caption);
}

.batch-governance__item-badge.is-ok {
  background: var(--color-success-bg);
  color: var(--color-success);
}

.batch-governance__item-badge.is-error {
  background: var(--color-danger-bg);
  color: var(--color-danger);
}

.batch-governance__form,
.batch-governance__list,
.batch-governance__plan-grid {
  display: grid;
  gap: var(--space-3);
}

.batch-governance__plan-grid {
  grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
}

.batch-governance__plan-item {
  border: var(--border);
  border-radius: var(--radius-sm);
  background: var(--surface-card);
  padding: var(--space-3);
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
}

.batch-governance__plan-item dt {
  color: var(--text-secondary);
  font: var(--type-caption);
}

.batch-governance__plan-item dd {
  margin: 0;
  color: var(--text-primary);
  font: var(--type-body-sm);
  word-break: break-word;
}

.batch-governance__field {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}

.batch-governance__label,
.batch-governance__help {
  color: var(--text-secondary);
  font: var(--type-caption);
}

.batch-governance__input,
.batch-governance__textarea,
.batch-governance__select {
  width: 100%;
}

.batch-governance__textarea {
  min-height: 88px;
  padding: var(--comp-padding-sm);
  border: var(--border);
  border-radius: var(--radius-sm);
  resize: vertical;
  background: var(--surface-card);
  color: var(--text-primary);
}

.batch-governance__checkbox {
  width: 18px;
  height: 18px;
}

.batch-governance__actions {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  flex-wrap: wrap;
}

.batch-governance__dialog-list,
.batch-governance__section,
.batch-governance__item {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
}

.batch-governance__item {
  border: var(--border);
  border-radius: var(--radius-md);
  background: var(--surface-card);
  padding: var(--space-3);
}

.batch-governance__item-path {
  color: var(--text-primary);
  font: var(--type-body-sm);
  font-family: var(--font-family-mono);
  word-break: break-all;
}

.batch-governance__receipt-meta {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-3);
}

.batch-governance__details {
  color: var(--text-secondary);
  font: var(--type-body-sm);
}

.batch-governance__output {
  margin-top: var(--space-2);
  border: var(--border);
  border-radius: var(--radius-md);
  background: var(--ds-background-2);
  padding: var(--space-4);
  white-space: pre-wrap;
  word-break: break-word;
  color: var(--text-primary);
}

.batch-governance__message--error {
  color: var(--color-danger);
}
</style>

