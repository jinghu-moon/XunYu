<script setup lang="ts">
import type { GuardedTaskPreviewResponse } from '../types'
import type { TaskFormState, WorkspaceTaskDefinition } from '../features/tasks'
import FileGovernanceSummary from './FileGovernanceSummary.vue'
import UnifiedConfirmDialog from './UnifiedConfirmDialog.vue'

const props = withDefaults(
  defineProps<{
    modelValue: boolean
    title: string
    task: WorkspaceTaskDefinition
    form: TaskFormState
    preview?: GuardedTaskPreviewResponse | null
    busy?: boolean
    confirmDisabled?: boolean
  }>(),
  {
    preview: null,
    busy: false,
    confirmDisabled: false,
  },
)

const emit = defineEmits<{
  (event: 'update:modelValue', value: boolean): void
  (event: 'confirm'): void
}>()
</script>

<template>
  <UnifiedConfirmDialog
    :model-value="props.modelValue"
    :title="props.title"
    :preview="props.preview"
    :busy="props.busy"
    :confirm-disabled="props.confirmDisabled"
    @update:model-value="emit('update:modelValue', $event)"
    @confirm="emit('confirm')"
  >
    <template #preview-extra>
      <FileGovernanceSummary
        v-if="props.preview"
        :task="props.task"
        :form="props.form"
        phase="preview"
        :process="props.preview.process"
        :details="props.preview.details"
      />
    </template>
  </UnifiedConfirmDialog>
</template>
