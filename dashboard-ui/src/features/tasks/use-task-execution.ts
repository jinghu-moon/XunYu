import { computed, reactive, ref, watch } from 'vue'

import {
  executeGuardedTask,
  previewGuardedTask,
  runWorkspaceTask,
} from '../../api'
import type {
  GuardedTaskPreviewResponse,
  GuardedTaskReceipt,
  WorkspaceCapabilities,
  WorkspaceTaskRunResponse,
} from '../../types'
import type { TaskFormState, WorkspaceTaskDefinition } from './catalog'
import {
  applyTaskInitialValues,
  createInitialTaskForm,
  errorMessage,
  isTaskSupported,
  resolveTaskExecutionActionLabel,
  resolveTaskExecutionFailureHint,
  resolveTaskExecutionStateLabel,
  resolveTaskExecutionStateTone,
  type TaskExecutionState,
  validateTaskForm,
} from './task-execution-core'

type TaskExecutionProps = {
  task: WorkspaceTaskDefinition
  capabilities?: WorkspaceCapabilities | null
  initialValues?: Partial<TaskFormState> | null
  presetVersion?: number
}

export function useTaskExecution(props: TaskExecutionProps) {
  const form = reactive(createInitialTaskForm(props.task.fields))

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

  const isSupported = computed(() => isTaskSupported(props.task.feature, props.capabilities))
  const taskNotices = computed(() => props.task.notices ?? [])
  const actionLabel = computed(() => resolveTaskExecutionActionLabel(props.task.mode))
  const processOutput = computed(() => result.value?.process ?? null)
  const previewOutput = computed(() => preview.value?.process ?? null)
  const failureHint = computed(() =>
    resolveTaskExecutionFailureHint(state.value, requestError.value, processOutput.value),
  )
  const stateLabel = computed(() => resolveTaskExecutionStateLabel(state.value))
  const stateTone = computed(() => resolveTaskExecutionStateTone(state.value))

  watch(
    () => props.presetVersion,
    () => {
      applyTaskInitialValues(props.task.fields, form, props.initialValues)
      validationError.value = ''
    },
    { immediate: true },
  )

  function validate() {
    validationError.value = validateTaskForm(props.task.fields, form)
    return !validationError.value
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

  return {
    actionLabel,
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
    state,
    stateLabel,
    stateTone,
    taskNotices,
    validationError,
    confirmTask,
    previewTask,
    runTask,
  }
}
