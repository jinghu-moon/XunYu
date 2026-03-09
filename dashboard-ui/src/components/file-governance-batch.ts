import type {
  GuardedTaskPreviewRequest,
  GuardedTaskPreviewResponse,
  GuardedTaskReceipt,
  RecentTasksFocusRequest,
  StatisticsWorkspaceLinkPayload,
  TaskProcessOutput,
} from '../types'
import type { TaskFieldDefinition, TaskFormState, WorkspaceTaskDefinition } from '../workspace-tools'
import { filesSecurityTaskGroups } from '../workspace-tools'
import { resolveDiagnosticsGovernanceFamilyFromAction } from './statistics-diagnostics-focus'

export type BatchGovernanceActionId =
  | 'protect-set'
  | 'protect-clear'
  | 'encrypt'
  | 'decrypt'
  | 'acl-add'
  | 'acl-copy'
  | 'acl-restore'
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
  form?: TaskFormState
  preview?: GuardedTaskPreviewResponse
  error?: string
}

export interface BatchGovernanceReceiptItem {
  path: string
  form?: TaskFormState
  receipt?: GuardedTaskReceipt
  error?: string
}

export interface BatchGovernancePlanItem {
  label: string
  value: string
}

export interface BatchGovernancePlanModel {
  title: string
  note?: string
  items: BatchGovernancePlanItem[]
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
  encrypt: {
    label: '批量加密文件',
    description: '对批量队列里的路径统一执行 EFS 或 age 公钥加密。',
  },
  decrypt: {
    label: '批量解密文件',
    description: '对批量队列里的路径统一执行 EFS 或 identity 解密。',
  },
  'acl-add': {
    label: '批量新增 ACL 规则',
    description: '按同一组 principal / rights / inherit 参数批量写入显式 ACE。',
  },
  'acl-copy': {
    label: '批量复制 ACL',
    description: '用同一个参考路径的 ACL 覆盖批量队列里的目标路径。',
  },
  'acl-restore': {
    label: '批量恢复 ACL',
    description: '从同一个备份文件恢复批量队列里的目标 ACL。',
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

function formatPlanFieldValue(field: TaskFieldDefinition, values: TaskFormState): string {
  const value = values[field.key]

  if (field.type === 'checkbox') {
    return value === true ? '是' : '否'
  }

  if (typeof value !== 'string') return '-'
  const trimmed = value.trim()
  if (!trimmed) return '-'

  if (field.type === 'select' && field.options?.length) {
    return field.options.find((option) => option.value === trimmed)?.label ?? trimmed
  }

  if (field.type === 'textarea') {
    return trimmed
      .split(/[\r\n,]+/)
      .map((item) => item.trim())
      .filter(Boolean)
      .join(' / ')
  }

  return trimmed
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

export function createBatchGovernanceItemForm(
  actionId: BatchGovernanceActionId,
  path: string,
  sharedValues: TaskFormState,
): TaskFormState {
  const task = getBatchGovernanceAction(actionId).task
  return {
    ...createTaskState(task.fields),
    ...sharedValues,
    path,
  }
}

export function buildBatchGovernancePlan(
  actionId: BatchGovernanceActionId,
  paths: string[],
  sharedValues: TaskFormState,
): BatchGovernancePlanModel {
  const action = getBatchGovernanceAction(actionId)
  const normalizedPaths = normalizeBatchPaths(paths)
  const sampleForm = createBatchGovernanceItemForm(actionId, normalizedPaths[0] ?? '', sharedValues)
  const sharedItems = getBatchGovernanceSharedFields(actionId)
    .map((field) => ({ label: field.label, value: formatPlanFieldValue(field, sampleForm), type: field.type }))
    .filter((item) => item.value !== '-' || item.type === 'checkbox')
    .map(({ label, value }) => ({ label, value }))

  const scopeValue =
    normalizedPaths.length > 3
      ? `${normalizedPaths.slice(0, 3).join(' / ')} / 共 ${normalizedPaths.length} 项`
      : normalizedPaths.join(' / ') || '-'

  return {
    title: '治理计划',
    note:
      action.task.tone === 'danger'
        ? '高风险批量治理动作；先逐项 dry-run，再统一确认，并输出逐项回执。'
        : '本次治理会先执行批量预演，再统一确认并输出回执。',
    items: [
      { label: '治理动作', value: action.label },
      { label: '目标数量', value: `${normalizedPaths.length} 项` },
      { label: '执行模式', value: 'Triple-Guard：预演 → 确认 → 回执' },
      { label: '治理范围', value: scopeValue },
      ...sharedItems,
    ],
  }
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
    const values = createBatchGovernanceItemForm(actionId, path, sharedValues)

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

function resolveRecentTasksAction(
  actionId: BatchGovernanceActionId,
  action: string | null | undefined,
): string {
  const normalized = String(action ?? '').trim()
  if (normalized) return normalized
  return getBatchGovernanceAction(actionId).task.action
}

export function createRecentTasksFocusFromBatchPreview(
  actionId: BatchGovernanceActionId,
  item: BatchGovernancePreviewItem,
): Omit<RecentTasksFocusRequest, 'key'> {
  return {
    status: item.preview?.status ?? 'previewed',
    dry_run: 'dry-run',
    action: resolveRecentTasksAction(actionId, item.preview?.action),
    search: item.path,
  }
}

export function createRecentTasksFocusFromBatchReceipt(
  actionId: BatchGovernanceActionId,
  item: BatchGovernanceReceiptItem,
): Omit<RecentTasksFocusRequest, 'key'> {
  return {
    status: item.receipt?.status ?? 'failed',
    dry_run: 'executed',
    action: resolveRecentTasksAction(actionId, item.receipt?.action),
    search: item.path,
  }
}

function resolveBatchGovernanceDiagnosticsStatus(payload: {
  previewStatus?: string | null
  receiptStatus?: string | null
  hasPreview?: boolean
}): 'previewed' | 'succeeded' | 'failed' {
  if (payload.receiptStatus === 'succeeded') return 'succeeded'
  if (payload.receiptStatus === 'failed') return 'failed'
  if (payload.previewStatus === 'previewed' || payload.hasPreview) return 'previewed'
  return 'failed'
}

export function createDiagnosticsLinkFromBatchPreview(
  actionId: BatchGovernanceActionId,
  item: BatchGovernancePreviewItem,
): StatisticsWorkspaceLinkPayload {
  const action = resolveRecentTasksAction(actionId, item.preview?.action)
  const governanceFamily = resolveDiagnosticsGovernanceFamilyFromAction(action)

  return {
    panel: 'diagnostics-center',
    request: governanceFamily
      ? {
          panel: 'governance',
          governance_family: governanceFamily,
          governance_status: resolveBatchGovernanceDiagnosticsStatus({
            previewStatus: item.preview?.status,
            hasPreview: Boolean(item.preview),
          }),
          target: item.preview?.target || item.path,
        }
      : {
          panel: 'failed',
          target: item.preview?.target || item.path,
        },
  }
}

export function createAuditLinkFromBatchReceipt(
  actionId: BatchGovernanceActionId,
  item: BatchGovernanceReceiptItem,
): StatisticsWorkspaceLinkPayload {
  const receipt = item.receipt
  const action = String(receipt?.audit_action || resolveRecentTasksAction(actionId, receipt?.action)).trim()
  const result = receipt?.status === 'succeeded' ? 'success' : 'failed'

  return {
    panel: 'audit',
    request: {
      search: item.path,
      action,
      result,
    },
  }
}

export function createDiagnosticsLinkFromBatchReceipt(
  actionId: BatchGovernanceActionId,
  item: BatchGovernanceReceiptItem,
): StatisticsWorkspaceLinkPayload {
  const receipt = item.receipt
  const action = resolveRecentTasksAction(actionId, receipt?.action)
  const governanceFamily = resolveDiagnosticsGovernanceFamilyFromAction(action)

  return {
    panel: 'diagnostics-center',
    request: governanceFamily
      ? {
          panel: 'governance',
          governance_family: governanceFamily,
          governance_status: resolveBatchGovernanceDiagnosticsStatus({
            receiptStatus: receipt?.status,
          }),
          target: receipt?.target || item.path,
          audit_action: receipt?.audit_action || undefined,
          audit_result: receipt?.status === 'succeeded' ? 'success' : 'failed',
          audit_timestamp: receipt?.audited_at,
        }
      : {
          panel: 'audit',
          target: receipt?.target || item.path,
          audit_action: receipt?.audit_action || undefined,
          audit_result: receipt?.status === 'succeeded' ? 'success' : 'failed',
          audit_timestamp: receipt?.audited_at,
        },
  }
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
