import type { TaskProcessOutput, WorkspaceCapabilities } from '../../types'
import type { TaskFieldDefinition, TaskFieldValue, TaskFormState } from './catalog'

export type TaskExecutionState =
  | 'idle'
  | 'previewing'
  | 'awaiting_confirm'
  | 'running'
  | 'succeeded'
  | 'failed'

const DEFAULT_ERROR_MESSAGE = '\u8bf7\u6c42\u5931\u8d25\uff0c\u8bf7\u68c0\u67e5\u5168\u5c40\u9519\u8bef\u63d0\u793a\u3002'
const PERMISSION_FAILURE_MESSAGE = '\u53ef\u80fd\u7f3a\u5c11\u7ba1\u7406\u5458\u6743\u9650\uff0c\u8bf7\u4ee5\u7ba1\u7406\u5458\u65b9\u5f0f\u8fd0\u884c\u6216\u4f7f\u7528\u63d0\u6743\u53c2\u6570\u3002'
const WINDOW_FAILURE_MESSAGE = '\u672a\u627e\u5230\u5339\u914d\u7a97\u53e3\uff0c\u8bf7\u786e\u8ba4\u76ee\u6807\u7a97\u53e3\u5df2\u6253\u5f00\u4e14\u6807\u9898\u5339\u914d\u3002'
const HOTKEY_FAILURE_MESSAGE = '\u5feb\u6377\u952e\u683c\u5f0f\u9519\u8bef\u6216\u88ab\u7cfb\u7edf\u4fdd\u7559\uff0c\u68c0\u67e5\u8f93\u5165\u683c\u5f0f\u3002'
const HOSTS_FILE_FAILURE_MESSAGE = 'hosts \u6587\u4ef6\u8bbf\u95ee\u5931\u8d25\uff0c\u53ef\u80fd\u9700\u8981\u7ba1\u7406\u5458\u6743\u9650\u3002'

export function createInitialTaskForm(fields: TaskFieldDefinition[]): TaskFormState {
  return fields.reduce<TaskFormState>((state, field) => {
    state[field.key] = field.defaultValue ?? (field.type === 'checkbox' ? false : '')
    return state
  }, {})
}

export function applyTaskInitialValues(
  fields: TaskFieldDefinition[],
  form: TaskFormState,
  initialValues?: Partial<TaskFormState> | null,
) {
  if (!initialValues) return

  for (const field of fields) {
    if (!Object.prototype.hasOwnProperty.call(initialValues, field.key)) continue
    const nextValue = initialValues[field.key]
    if (nextValue !== undefined) {
      form[field.key] = nextValue
    }
  }
}

export function isTaskSupported(
  feature: keyof WorkspaceCapabilities | null | undefined,
  capabilities?: WorkspaceCapabilities | null,
): boolean {
  if (!feature || !capabilities) return true
  return capabilities[feature] !== false
}

export function resolveTaskExecutionActionLabel(mode: 'run' | 'guarded'): string {
  return mode === 'guarded' ? '\u9884\u6f14\u5e76\u786e\u8ba4' : '\u8fd0\u884c'
}

export function errorMessage(err: unknown): string {
  if (err instanceof Error && err.message.trim()) return err.message
  return DEFAULT_ERROR_MESSAGE
}

export function classifyFailure(message: string): string {
  const raw = message.trim()
  if (!raw) return ''

  const lower = raw.toLowerCase()
  if (
    lower.includes('access denied') ||
    lower.includes('permission denied') ||
    lower.includes('administrator') ||
    lower.includes('admin') ||
    lower.includes('elevated') ||
    raw.includes('\u6743\u9650') ||
    raw.includes('\u7ba1\u7406\u5458')
  ) {
    return PERMISSION_FAILURE_MESSAGE
  }
  if (lower.includes('no matching window')) {
    return WINDOW_FAILURE_MESSAGE
  }
  if (
    lower.includes('invalid hotkey') ||
    lower.includes('hotkey not allowed') ||
    lower.includes('remap source')
  ) {
    return HOTKEY_FAILURE_MESSAGE
  }
  if (lower.includes('hosts file')) {
    return HOSTS_FILE_FAILURE_MESSAGE
  }

  return ''
}

export function resolveTaskExecutionFailureHint(
  state: TaskExecutionState,
  requestError: string,
  processOutput: TaskProcessOutput | null,
): string {
  if (state !== 'failed') return ''
  const raw = requestError || processOutput?.stderr || processOutput?.stdout || ''
  return classifyFailure(raw)
}

export function resolveTaskExecutionStateLabel(state: TaskExecutionState): string {
  switch (state) {
    case 'previewing':
      return '\u9884\u6f14\u4e2d'
    case 'awaiting_confirm':
      return '\u5f85\u786e\u8ba4'
    case 'running':
      return '\u6267\u884c\u4e2d'
    case 'succeeded':
      return '\u6210\u529f'
    case 'failed':
      return '\u5931\u8d25'
    default:
      return '\u5f85\u6267\u884c'
  }
}

export function resolveTaskExecutionStateTone(
  state: TaskExecutionState,
): '' | 'is-ok' | 'is-error' {
  if (state === 'succeeded') return 'is-ok'
  if (state === 'failed') return 'is-error'
  return ''
}

function isFieldEmpty(field: TaskFieldDefinition, form: TaskFormState): boolean {
  const value = form[field.key] as TaskFieldValue
  if (field.type === 'checkbox') return value !== true
  return typeof value !== 'string' || !value.trim()
}

export function validateTaskForm(fields: TaskFieldDefinition[], form: TaskFormState): string {
  const missing = fields.filter((field) => field.required && isFieldEmpty(field, form))
  return missing.length
    ? `\u7f3a\u5c11\u5fc5\u586b\u9879\uff1a${missing.map((field) => field.label).join('\u3001')}`
    : ''
}
