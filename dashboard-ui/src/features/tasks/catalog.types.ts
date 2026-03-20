import type { WorkspaceCapabilities, WorkspaceKey } from '../../types'

export type TaskFieldType = 'text' | 'textarea' | 'number' | 'select' | 'checkbox'
export type TaskFieldValue = string | boolean
export type TaskFormState = Record<string, TaskFieldValue>

export interface TaskFieldOption {
  label: string
  value: string
}

export interface TaskFieldDefinition {
  key: string
  label: string
  type: TaskFieldType
  placeholder?: string
  help?: string
  defaultValue?: TaskFieldValue
  required?: boolean
  options?: TaskFieldOption[]
  min?: number
  max?: number
}

export type TaskNoticeTone = 'info' | 'warning'

export interface TaskNotice {
  text: string
  tone?: TaskNoticeTone
}

export interface WorkspaceTaskDefinition {
  id: string
  workspace: WorkspaceKey
  title: string
  description: string
  action: string
  mode: 'run' | 'guarded'
  tone?: 'default' | 'danger'
  feature?: keyof WorkspaceCapabilities
  notices?: TaskNotice[]
  fields: TaskFieldDefinition[]
  target?: (values: TaskFormState) => string
  buildRunArgs?: (values: TaskFormState) => string[]
  buildPreviewArgs?: (values: TaskFormState) => string[]
  buildExecuteArgs?: (values: TaskFormState) => string[]
  previewSummary?: (values: TaskFormState) => string
}

export interface WorkspaceTaskGroup {
  id: string
  title: string
  description: string
  tasks: WorkspaceTaskDefinition[]
}

export interface WorkspaceTabDefinition {
  value: WorkspaceKey
  label: string
  description: string
}
