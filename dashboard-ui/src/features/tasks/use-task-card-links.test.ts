import { ref } from 'vue'
import { describe, expect, it, vi } from 'vitest'

import type { GuardedTaskReceipt, WorkspaceTaskRunResponse } from '../../types'
import { useTaskCardLinks } from './use-task-card-links'

describe('useTaskCardLinks', () => {
  it('emits run-result link payloads', () => {
    const emit = vi.fn()
    const result = ref<WorkspaceTaskRunResponse | null>({
      workspace: 'paths-context',
      action: 'recent',
      target: 'D:/repo/demo.txt',
      process: {
        command_line: 'xun recent -n 10',
        exit_code: 0,
        success: true,
        stdout: 'ok',
        stderr: '',
        duration_ms: 12,
      },
      details: null,
    })
    const receipt = ref<GuardedTaskReceipt | null>(null)
    const links = useTaskCardLinks({ action: 'recent', result, receipt, emit })

    links.focusRecentTasksForResult()
    links.focusAuditForResult()

    expect(emit).toHaveBeenNthCalledWith(1, {
      panel: 'recent-tasks',
      request: {
        status: 'succeeded',
        dry_run: 'executed',
        search: 'D:/repo/demo.txt',
        action: 'recent',
      },
    })
    expect(emit).toHaveBeenNthCalledWith(2, {
      panel: 'audit',
      request: {
        search: 'D:/repo/demo.txt',
        result: 'success',
      },
    })
  })

  it('emits receipt link payloads and ignores null inputs', () => {
    const emit = vi.fn()
    const result = ref<WorkspaceTaskRunResponse | null>(null)
    const receipt = ref<GuardedTaskReceipt | null>({
      token: 'token-1',
      workspace: 'files-security',
      action: 'rm',
      target: 'D:/tmp/demo.txt',
      phase: 'execute',
      status: 'failed',
      guarded: true,
      dry_run: false,
      summary: 'rm failed',
      audit_action: 'dashboard.task.execute.rm',
      audited_at: 1700000000,
      process: {
        command_line: 'xun rm -y D:/tmp/demo.txt',
        exit_code: 1,
        success: false,
        stdout: '',
        stderr: 'failed',
        duration_ms: 18,
      },
      details: null,
    })
    const links = useTaskCardLinks({ action: 'rm', result, receipt, emit })

    links.focusRecentTasksForResult()
    links.focusAuditForResult()
    links.focusRecentTasksForReceipt()
    links.focusAuditForReceipt()

    expect(emit).toHaveBeenCalledTimes(2)
    expect(emit).toHaveBeenNthCalledWith(1, {
      panel: 'recent-tasks',
      request: {
        status: 'failed',
        dry_run: 'executed',
        search: 'D:/tmp/demo.txt',
        action: 'rm',
      },
    })
    expect(emit).toHaveBeenNthCalledWith(2, {
      panel: 'audit',
      request: {
        search: 'D:/tmp/demo.txt',
        action: 'dashboard.task.execute.rm',
        result: 'failed',
      },
    })
  })
})
