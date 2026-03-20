import { describe, expect, it } from 'vitest'

import type { GuardedTaskReceipt, WorkspaceTaskRunResponse } from '../../types'
import {
  buildAuditLinkPayloadFromReceipt,
  buildAuditLinkPayloadFromRunResult,
  buildRecentTasksLinkPayloadFromReceipt,
  buildRecentTasksLinkPayloadFromRunResult,
} from './task-link-payloads'

const runResult: WorkspaceTaskRunResponse = {
  workspace: 'paths-context',
  action: 'recent',
  target: 'D:/repo/demo.txt',
  process: {
    command_line: 'xun recent -n 10',
    exit_code: 0,
    success: true,
    stdout: 'ok',
    stderr: '',
    duration_ms: 10,
  },
  details: null,
}

const failedReceipt: GuardedTaskReceipt = {
  token: 'token-1',
  workspace: 'files-security',
  action: 'rm',
  target: 'D:/repo/demo.txt',
  phase: 'execute',
  status: 'failed',
  guarded: true,
  dry_run: false,
  summary: 'rm demo',
  audit_action: 'workspace.rm.execute',
  audited_at: 1700000000,
  process: {
    command_line: 'xun rm -y D:/repo/demo.txt',
    exit_code: 1,
    success: false,
    stdout: '',
    stderr: 'failed',
    duration_ms: 20,
  },
  details: null,
}

describe('task-link-payloads', () => {
  it('builds link payloads from run results', () => {
    expect(buildRecentTasksLinkPayloadFromRunResult(runResult, 'fallback')).toEqual({
      panel: 'recent-tasks',
      request: {
        status: 'succeeded',
        dry_run: 'executed',
        search: 'D:/repo/demo.txt',
        action: 'recent',
      },
    })
    expect(buildAuditLinkPayloadFromRunResult(runResult)).toEqual({
      panel: 'audit',
      request: {
        search: 'D:/repo/demo.txt',
        result: 'success',
      },
    })
  })

  it('builds link payloads from guarded receipts', () => {
    expect(buildRecentTasksLinkPayloadFromReceipt(failedReceipt)).toEqual({
      panel: 'recent-tasks',
      request: {
        status: 'failed',
        dry_run: 'executed',
        search: 'D:/repo/demo.txt',
        action: 'rm',
      },
    })
    expect(buildAuditLinkPayloadFromReceipt(failedReceipt)).toEqual({
      panel: 'audit',
      request: {
        search: 'D:/repo/demo.txt',
        action: 'workspace.rm.execute',
        result: 'failed',
      },
    })
  })

  it('returns null when source payload is absent', () => {
    expect(buildRecentTasksLinkPayloadFromRunResult(null, 'fallback')).toBeNull()
    expect(buildAuditLinkPayloadFromRunResult(null)).toBeNull()
    expect(buildRecentTasksLinkPayloadFromReceipt(null)).toBeNull()
    expect(buildAuditLinkPayloadFromReceipt(null)).toBeNull()
  })
})
