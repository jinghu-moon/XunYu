<script setup lang="ts">
import type { GuardedTaskReceipt } from '../types'
import type { TaskFormState, WorkspaceTaskDefinition } from '../features/tasks'
import FileGovernanceSummary from './FileGovernanceSummary.vue'
import TaskReceiptComponent from './TaskReceiptComponent.vue'

const props = withDefaults(
  defineProps<{
    task: WorkspaceTaskDefinition
    form: TaskFormState
    receipt: GuardedTaskReceipt
    recentLinkTestId?: string
    auditLinkTestId?: string
  }>(),
  {
    recentLinkTestId: 'task-receipt-link-recent',
    auditLinkTestId: 'task-receipt-link-audit',
  },
)

const emit = defineEmits<{
  (event: 'focus-recent-tasks'): void
  (event: 'focus-audit'): void
}>()

const TEXT = {
  recentTasks: '\u56de\u5230\u6700\u8fd1\u4efb\u52a1',
  audit: '\u67e5\u770b\u5ba1\u8ba1',
} as const
</script>

<template>
  <section class="task-receipt-section">
    <FileGovernanceSummary
      :task="props.task"
      :form="props.form"
      phase="execute"
      :process="props.receipt.process"
      :details="props.receipt.details"
    />

    <div class="task-receipt-section__links">
      <button
        :data-testid="props.recentLinkTestId"
        class="task-receipt-section__link"
        type="button"
        @click="emit('focus-recent-tasks')"
      >
        {{ TEXT.recentTasks }}
      </button>
      <button
        :data-testid="props.auditLinkTestId"
        class="task-receipt-section__link"
        type="button"
        @click="emit('focus-audit')"
      >
        {{ TEXT.audit }}
      </button>
    </div>

    <TaskReceiptComponent :receipt="props.receipt" />
  </section>
</template>

<style scoped>
.task-receipt-section {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}

.task-receipt-section__links {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
}

.task-receipt-section__link {
  padding: 0;
  border: none;
  background: transparent;
  color: var(--color-primary);
  cursor: pointer;
  font: var(--type-caption);
}
</style>
