import type {
  GuardedTaskReceipt,
  StatisticsWorkspaceLinkPayload,
  WorkspaceTaskRunResponse,
} from '../../types'

export function buildRecentTasksLinkPayloadFromRunResult(
  result: WorkspaceTaskRunResponse | null,
  fallbackAction: string,
): StatisticsWorkspaceLinkPayload | null {
  if (!result) return null

  return {
    panel: 'recent-tasks',
    request: {
      status: result.process.success ? 'succeeded' : 'failed',
      dry_run: 'executed',
      search: result.target || undefined,
      action: result.action || fallbackAction,
    },
  }
}

export function buildAuditLinkPayloadFromRunResult(
  result: WorkspaceTaskRunResponse | null,
): StatisticsWorkspaceLinkPayload | null {
  if (!result) return null

  return {
    panel: 'audit',
    request: {
      search: result.target || undefined,
      result: result.process.success ? 'success' : 'failed',
    },
  }
}

export function buildRecentTasksLinkPayloadFromReceipt(
  receipt: GuardedTaskReceipt | null,
): StatisticsWorkspaceLinkPayload | null {
  if (!receipt) return null

  return {
    panel: 'recent-tasks',
    request: {
      status: receipt.status,
      dry_run: receipt.dry_run ? 'dry-run' : 'executed',
      search: receipt.target || undefined,
      action: receipt.action,
    },
  }
}

export function buildAuditLinkPayloadFromReceipt(
  receipt: GuardedTaskReceipt | null,
): StatisticsWorkspaceLinkPayload | null {
  if (!receipt) return null

  return {
    panel: 'audit',
    request: {
      search: receipt.target || undefined,
      action: receipt.audit_action || undefined,
      result: receipt.status === 'failed' ? 'failed' : 'success',
    },
  }
}
