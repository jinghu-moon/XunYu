import type {
  AuditEntry,
  DiagnosticsCenterFocusRequest,
  DiagnosticsGovernanceFamilyFilter,
  DiagnosticsGovernanceStatusFilter,
  RecentTaskRecord,
} from '../types'

type DiagnosticsGovernanceGroupKey = Exclude<DiagnosticsGovernanceFamilyFilter, 'all'>

export function resolveDiagnosticsGovernanceFamilyFromAction(
  action: string | null | undefined,
): DiagnosticsGovernanceGroupKey | null {
  const normalized = String(action ?? '').trim().toLowerCase()
  if (!normalized) return null
  if (normalized.includes('acl:')) return 'acl'
  if (normalized.includes('protect:')) return 'protect'
  if (normalized.includes('encrypt') || normalized.includes('decrypt')) return 'crypt'
  return null
}

export function resolveDiagnosticsGovernanceStatusFromAuditResult(
  result: string | null | undefined,
): DiagnosticsGovernanceStatusFilter {
  const normalized = String(result ?? '').trim().toLowerCase()
  if (normalized === 'failed') return 'failed'
  if (normalized === 'dry_run') return 'previewed'
  if (normalized === 'success') return 'succeeded'
  return 'all'
}

export function buildDiagnosticsAuditEntryKey(payload: {
  timestamp: number
  action: string
  target?: string | null
  result?: string | null
}): string {
  return [payload.timestamp, payload.action || '', payload.target || '', payload.result || ''].join('::')
}

export function resolveDiagnosticsCenterFocusFromRecentTask(
  record: RecentTaskRecord,
): Omit<DiagnosticsCenterFocusRequest, 'key'> {
  const governanceFamily = resolveDiagnosticsGovernanceFamilyFromAction(record.action)
  if (governanceFamily) {
    return {
      panel: 'governance',
      governance_family: governanceFamily,
      governance_status: record.status,
      task_id: record.id,
      target: record.target || undefined,
      audit_action: record.audit_action || undefined,
    }
  }

  if (record.status === 'failed') {
    return {
      panel: 'failed',
      task_id: record.id,
      target: record.target || undefined,
      audit_action: record.audit_action || undefined,
    }
  }

  if (record.guarded) {
    return {
      panel: 'guarded',
      task_id: record.id,
      target: record.target || undefined,
      audit_action: record.audit_action || undefined,
    }
  }

  return {
    panel: 'audit',
    target: record.target || undefined,
    audit_action: record.audit_action || undefined,
    audit_result: record.status === 'previewed' ? 'dry_run' : 'success',
  }
}

export function resolveDiagnosticsCenterFocusFromAuditEntry(
  entry: AuditEntry,
): Omit<DiagnosticsCenterFocusRequest, 'key'> {
  const governanceFamily = resolveDiagnosticsGovernanceFamilyFromAction(entry.action)
  if (governanceFamily) {
    return {
      panel: 'governance',
      governance_family: governanceFamily,
      governance_status: resolveDiagnosticsGovernanceStatusFromAuditResult(entry.result),
      target: entry.target || undefined,
      audit_action: entry.action || undefined,
      audit_result: entry.result || undefined,
      audit_timestamp: entry.timestamp,
    }
  }

  return {
    panel: 'audit',
    target: entry.target || undefined,
    audit_action: entry.action || undefined,
    audit_result: entry.result || undefined,
    audit_timestamp: entry.timestamp,
  }
}
