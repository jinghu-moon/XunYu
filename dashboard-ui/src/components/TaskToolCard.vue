<script setup lang="ts">
import { computed } from 'vue'
import type {
  StatisticsWorkspaceLinkPayload,
  WorkspaceCapabilities,
} from '../types'
import type { TaskFieldValue, TaskFormState, WorkspaceTaskDefinition } from '../features/tasks'
import {
  executeTaskCardAction,
  resolveTaskCardActionHint,
  resolveTaskCardBusy,
} from '../features/tasks/task-card-core'
import { useTaskCardLinks } from '../features/tasks/use-task-card-links'
import { useTaskExecution } from '../features/tasks/use-task-execution'
import TaskCardActions from './TaskCardActions.vue'
import TaskCardHeader from './TaskCardHeader.vue'
import TaskConfirmDialog from './TaskConfirmDialog.vue'
import TaskProcessResultPanel from './TaskProcessResultPanel.vue'
import TaskReceiptSection from './TaskReceiptSection.vue'
import TaskToolFields from './TaskToolFields.vue'

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

const {
  actionLabel,
  confirmTask,
  dialogOpen,
  executeBusy,
  failureHint,
  form,
  isSupported,
  preview,
  previewBusy,
  previewOutput,
  processOutput,
  receipt,
  requestError,
  result,
  runBusy,
  stateLabel,
  stateTone,
  taskNotices,
  validationError,
  previewTask,
  runTask,
} = useTaskExecution(props)

const TEXT = {
  previewReady: '预演通过',
  previewFailed: '预演失败',
  succeeded: '成功',
  failed: '失败',
} as const

const actionHint = computed(() =>
  resolveTaskCardActionHint(isSupported.value, validationError.value, requestError.value),
)

const actionBusy = computed(() =>
  resolveTaskCardBusy(props.task.mode, previewBusy.value, runBusy.value),
)

const {
  focusRecentTasksForResult,
  focusAuditForResult,
  focusRecentTasksForReceipt,
  focusAuditForReceipt,
} = useTaskCardLinks({
  action: props.task.action,
  result,
  receipt,
  emit: (payload) => {
    emit('link-panel', payload)
  },
})

function triggerTask() {
  executeTaskCardAction(props.task.mode, { previewTask, runTask })
}

function updateField(payload: { key: string; value: TaskFieldValue }) {
  form[payload.key] = payload.value
}
</script>

<template>
  <article :class="['task-card', props.task.tone === 'danger' ? 'task-card--danger' : '']">
    <TaskCardHeader
      :title="props.task.title"
      :description="props.task.description"
      :state-label="stateLabel"
      :state-tone="stateTone"
      :feature="props.task.feature ?? null"
      :notices="taskNotices"
    />

    <TaskToolFields
      v-if="props.task.fields.length"
      :fields="props.task.fields"
      :form="form"
      @update-field="updateField"
    />

    <TaskCardActions
      :tone="props.task.tone ?? 'default'"
      :action-label="actionLabel"
      :disabled="!isSupported"
      :loading="actionBusy"
      :hint-text="actionHint?.text ?? ''"
      :hint-tone="actionHint?.tone ?? 'default'"
      :failure-hint="failureHint"
      @trigger="triggerTask"
    />

    <TaskProcessResultPanel
      v-if="preview && previewOutput"
      :task="props.task"
      :form="form"
      phase="preview"
      :process="previewOutput"
      :details="preview.details"
      :badge-text="preview.ready_to_execute ? TEXT.previewReady : TEXT.previewFailed"
      :badge-tone="preview.ready_to_execute ? 'is-ok' : 'is-error'"
      :meta-text="preview.summary"
    />

    <TaskProcessResultPanel
      v-if="processOutput"
      :task="props.task"
      :form="form"
      phase="execute"
      :process="processOutput"
      :details="result?.details ?? null"
      :badge-text="processOutput.success ? TEXT.succeeded : TEXT.failed"
      :badge-tone="processOutput.success ? 'is-ok' : 'is-error'"
      :meta-text="`${processOutput.duration_ms} ms`"
      :show-links="true"
      recent-link-test-id="task-card-link-recent"
      audit-link-test-id="task-card-link-audit"
      @focus-recent-tasks="focusRecentTasksForResult"
      @focus-audit="focusAuditForResult"
    />

    <TaskReceiptSection
      v-if="receipt"
      :task="props.task"
      :form="form"
      :receipt="receipt"
      recent-link-test-id="task-card-link-recent-receipt"
      audit-link-test-id="task-card-link-audit-receipt"
      @focus-recent-tasks="focusRecentTasksForReceipt"
      @focus-audit="focusAuditForReceipt"
    />

    <TaskConfirmDialog
      v-model="dialogOpen"
      :title="props.task.title"
      :task="props.task"
      :form="form"
      :preview="preview"
      :busy="executeBusy"
      :confirm-disabled="!preview?.ready_to_execute"
      @confirm="confirmTask"
    />
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
</style>
