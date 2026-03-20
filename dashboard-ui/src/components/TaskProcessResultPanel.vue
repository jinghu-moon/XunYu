<script setup lang="ts">
import { computed } from 'vue'
import type { TaskProcessOutput, WorkspaceTaskDetails } from '../types'
import type { TaskFormState, WorkspaceTaskDefinition } from '../features/tasks'
import FileGovernanceSummary from './FileGovernanceSummary.vue'

const props = withDefaults(
  defineProps<{
    task: WorkspaceTaskDefinition
    form: TaskFormState
    phase: 'preview' | 'execute'
    process: TaskProcessOutput
    details?: WorkspaceTaskDetails | null
    badgeText: string
    badgeTone: 'is-ok' | 'is-error'
    metaText: string
    showLinks?: boolean
    recentLinkTestId?: string
    auditLinkTestId?: string
  }>(),
  {
    details: null,
    showLinks: false,
    recentLinkTestId: 'task-process-link-recent',
    auditLinkTestId: 'task-process-link-audit',
  },
)

const emit = defineEmits<{
  (event: 'focus-recent-tasks'): void
  (event: 'focus-audit'): void
}>()

const TEXT = {
  recentTasks: '\u56de\u5230\u6700\u8fd1\u4efb\u52a1',
  audit: '\u67e5\u770b\u5ba1\u8ba1',
  emptyOutput: '\u6682\u65e0\u8f93\u51fa',
} as const

const outputBody = computed(
  () => props.process.stdout || props.process.stderr || TEXT.emptyOutput,
)
</script>

<template>
  <section class="task-process-result-panel">
    <div class="task-process-result-panel__meta">
      <span :class="['task-process-result-panel__badge', props.badgeTone]">{{ props.badgeText }}</span>
      <span>{{ props.metaText }}</span>
    </div>

    <FileGovernanceSummary
      :task="props.task"
      :form="props.form"
      :phase="props.phase"
      :process="props.process"
      :details="props.details"
    />

    <div v-if="props.showLinks" class="task-process-result-panel__links">
      <button
        :data-testid="props.recentLinkTestId"
        class="task-process-result-panel__link"
        type="button"
        @click="emit('focus-recent-tasks')"
      >
        {{ TEXT.recentTasks }}
      </button>
      <button
        :data-testid="props.auditLinkTestId"
        class="task-process-result-panel__link"
        type="button"
        @click="emit('focus-audit')"
      >
        {{ TEXT.audit }}
      </button>
    </div>

    <pre class="task-process-result-panel__output">{{ props.process.command_line }}

{{ outputBody }}</pre>
  </section>
</template>

<style scoped>
.task-process-result-panel {
  border-top: var(--border);
  padding-top: var(--space-3);
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}

.task-process-result-panel__meta {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  color: var(--text-secondary);
  font: var(--type-caption);
}

.task-process-result-panel__badge {
  display: inline-flex;
  align-items: center;
  padding: 2px var(--space-3);
  border-radius: var(--radius-full);
  font-weight: var(--weight-semibold);
  background: var(--ds-background-2);
  color: var(--text-secondary);
}

.task-process-result-panel__badge.is-ok {
  background: var(--color-success-bg);
  color: var(--color-success);
}

.task-process-result-panel__badge.is-error {
  background: var(--color-danger-bg);
  color: var(--color-danger);
}

.task-process-result-panel__links {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
}

.task-process-result-panel__link {
  padding: 0;
  border: none;
  background: transparent;
  color: var(--color-primary);
  cursor: pointer;
  font: var(--type-caption);
}

.task-process-result-panel__output {
  border: var(--border);
  border-radius: var(--radius-md);
  background: var(--ds-background-2);
  padding: var(--space-4);
  white-space: pre-wrap;
  word-break: break-word;
  color: var(--text-primary);
}
</style>
