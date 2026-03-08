import type {
  GuardedTaskPreviewRequest,
  GuardedTaskPreviewResponse,
  GuardedTaskReceipt,
  TaskProcessOutput,
} from '../types'
import type { TaskFieldDefinition, TaskFormState, WorkspaceTaskDefinition } from '../workspace-tools'
import { filesSecurityTaskGroups } from '../workspace-tools'

export type BatchGovernanceActionId =
  | 'protect-set'
  | 'protect-clear'
  | 'acl-purge'
  | 'acl-inherit'
  | 'acl-owner'
  | 'acl-repair'

export interface BatchGovernanceActionDefinition {
  id: BatchGovernanceActionId
  label: string
  description: string
  task: WorkspaceTaskDefinition
}

export interface BatchGovernancePreviewItem {
  path: string
  preview?: GuardedTaskPreviewResponse
  error?: string
}

export interface BatchGovernanceReceiptItem {
  path: string
  receipt?: GuardedTaskReceipt
  error?: string
}

const batchGovernanceActionMeta: Record<BatchGovernanceActionId, { label: string; description: string }> = {
  'protect-set': {
    label: '批量设置保护',
    description: '对批量队列里的路径统一写入保护规则。',
  },
  'protect-clear': {
    label: '批量清除保护',
    description: '对批量队列里的路径统一移除保护规则。',
  },
  'acl-purge': {
    label: '批量清理 ACL 主体',
    description: '按同一主体批量清理多个路径上的显式 ACL 规则。',
  },
  'acl-inherit': {
    label: '批量切换 ACL 继承',
    description: '对批量队列统一启用或禁用 ACL 继承。',
  },
  'acl-owner': {
    label: '批量修改 ACL Owner',
    description: '把批量队列里的路径 Owner 统一切换到指定主体。',
  },
  'acl-repair': {
    label: '批量 ACL 强制修复',
    description: '逐项执行 take ownership + grant FullControl。',
  },
}

function createTaskState(fields: TaskFieldDefinition[]): TaskFormState {
  return fields.reduce<TaskFormState>((state, field) => {
    state[field.key] = field.defaultValue ?? (field.type === 'checkbox' ? false : '')
    return state
  }, {})
}

function findFilesSecurityTask(taskId: BatchGovernanceActionId): WorkspaceTaskDefinition {
  for (const group of filesSecurityTaskGroups) {
    const matched = group.tasks.find((task) => task.id === taskId)
    if (matched) return matched
  }
  throw new Error(`未找到文件治理任务定义：${taskId}`)
}

function buildAggregatedPreviewProcess(items: BatchGovernancePreviewItem[]): TaskProcessOutput {
  const previews = items
    .map((item) => item.preview)
    .filter((item): item is GuardedTaskPreviewResponse => Boolean(item))

  const readyCount = previews.filter((item) => item.ready_to_execute).length
  const errorLines = items.filter((item) => item.error).map((item) => `[阻塞] ${item.path}: ${item.error}`)
  const stdoutLines = [
    `批量预演共 ${items.length} 项`,
    `通过 ${readyCount} 项`,
    `阻塞 ${items.length - readyCount} 项`,
    ...items.map((item) => {
      if (item.preview) {
        return `[${item.preview.ready_to_execute ? '就绪' : '阻塞'}] ${item.path} · ${item.preview.summary}`
      }
      return `[阻塞] ${item.path}`
    }),
  ]

  return {
    command_line: previews.map((item) => item.process.command_line).join('\n'),
    exit_code: errorLines.length === 0 && readyCount === items.length ? 0 : 1,
    success: errorLines.length === 0 && readyCount === items.length,
    stdout: stdoutLines.join('\n'),
    stderr: errorLines.join('\n'),
    duration_ms: previews.reduce((total, item) => total + item.process.duration_ms, 0),
  }
}

export function normalizeBatchPaths(paths: string[]): string[] {
  const seen = new Set<string>()
  const normalized: string[] = []

  for (const raw of paths) {
    const path = raw.trim()
    if (!path || seen.has(path)) continue
    seen.add(path)
    normalized.push(path)
  }

  return normalized
}

export function getBatchGovernanceActions(): BatchGovernanceActionDefinition[] {
  return (Object.keys(batchGovernanceActionMeta) as BatchGovernanceActionId[]).map((id) => ({
    id,
    ...batchGovernanceActionMeta[id],
    task: findFilesSecurityTask(id),
  }))
}

export function getBatchGovernanceAction(actionId: BatchGovernanceActionId): BatchGovernanceActionDefinition {
  const matched = getBatchGovernanceActions().find((action) => action.id === actionId)
  if (!matched) throw new Error(`未找到批量治理动作：${actionId}`)
  return matched
}

export function getBatchGovernanceSharedFields(actionId: BatchGovernanceActionId): TaskFieldDefinition[] {
  return getBatchGovernanceAction(actionId).task.fields.filter((field) => field.key !== 'path')
}

export function createBatchGovernanceSharedState(actionId: BatchGovernanceActionId): TaskFormState {
  return createTaskState(getBatchGovernanceSharedFields(actionId))
}

export function createBatchGovernancePreviewRequests(
  actionId: BatchGovernanceActionId,
  paths: string[],
  sharedValues: TaskFormState,
): GuardedTaskPreviewRequest[] {
  const action = getBatchGovernanceAction(actionId)
  const { task } = action
  const buildPreviewArgs = task.buildPreviewArgs
  const buildExecuteArgs = task.buildExecuteArgs

  if (task.mode !== 'guarded' || !buildPreviewArgs || !buildExecuteArgs) {
    throw new Error(`批量治理动作未启用 guarded 协议：${actionId}`)
  }

  return normalizeBatchPaths(paths).map((path) => {
    const values: TaskFormState = {
      ...createTaskState(task.fields),
      ...sharedValues,
      path,
    }

    return {
      workspace: task.workspace,
      action: task.action,
      target: task.target?.(values) ?? path,
      preview_args: buildPreviewArgs(values),
      execute_args: buildExecuteArgs(values),
      preview_summary: task.previewSummary?.(values) ?? `${action.label} ${path}`,
    }
  })
}

export function summarizeBatchPreviews(items: BatchGovernancePreviewItem[]): {
  total: number
  ready: number
  blocked: number
} {
  const ready = items.filter((item) => item.preview?.ready_to_execute).length
  return {
    total: items.length,
    ready,
    blocked: Math.max(items.length - ready, 0),
  }
}

export function summarizeBatchReceipts(items: BatchGovernanceReceiptItem[]): {
  total: number
  succeeded: number
  failed: number
} {
  const succeeded = items.filter((item) => item.receipt?.process.success).length
  return {
    total: items.length,
    succeeded,
    failed: Math.max(items.length - succeeded, 0),
  }
}

export function isBatchPreviewReady(items: BatchGovernancePreviewItem[]): boolean {
  return items.length > 0 && items.every((item) => item.preview?.ready_to_execute === true)
}

export function createBatchGovernanceDialogPreview(
  actionId: BatchGovernanceActionId,
  items: BatchGovernancePreviewItem[],
): GuardedTaskPreviewResponse {
  const action = getBatchGovernanceAction(actionId)
  const stats = summarizeBatchPreviews(items)
  const previews = items
    .map((item) => item.preview)
    .filter((item): item is GuardedTaskPreviewResponse => Boolean(item))

  const summary = `${action.label}：${stats.ready}/${stats.total} 项预演通过`

  return {
    token: `batch-${actionId}`,
    workspace: action.task.workspace,
    action: action.task.action,
    target: `${stats.total} 项路径`,
    phase: 'preview',
    status: 'previewed',
    guarded: true,
    dry_run: true,
    ready_to_execute: isBatchPreviewReady(items),
    summary,
    preview_summary: summary,
    process: buildAggregatedPreviewProcess(items),
    expires_in_secs:
      previews.length > 0
        ? previews.reduce(
            (minValue, item) => Math.min(minValue, item.expires_in_secs),
            previews[0].expires_in_secs,
          )
        : 0,
  }
}
