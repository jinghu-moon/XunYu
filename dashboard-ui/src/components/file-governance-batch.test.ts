import { describe, expect, it } from 'vitest'
import {
  createBatchGovernanceDialogPreview,
  createBatchGovernancePreviewRequests,
  createBatchGovernanceSharedState,
  isBatchPreviewReady,
  normalizeBatchPaths,
  summarizeBatchPreviews,
  summarizeBatchReceipts,
  type BatchGovernancePreviewItem,
  type BatchGovernanceReceiptItem,
} from './file-governance-batch'

describe('file-governance-batch', () => {
  it('normalizes batch paths and removes blanks / duplicates', () => {
    expect(normalizeBatchPaths([' D:/repo/a ', '', 'D:/repo/a', 'D:/repo/b '])).toEqual([
      'D:/repo/a',
      'D:/repo/b',
    ])
  })

  it('builds guarded preview requests for protect:set', () => {
    const requests = createBatchGovernancePreviewRequests(
      'protect-set',
      ['D:/repo/a.txt', 'D:/repo/b.txt'],
      {
        ...createBatchGovernanceSharedState('protect-set'),
        deny: 'delete',
        require: 'force',
        systemAcl: true,
      },
    )

    expect(requests).toHaveLength(2)
    expect(requests[0]).toEqual({
      workspace: 'files-security',
      action: 'protect:set',
      target: 'D:/repo/a.txt',
      preview_args: ['protect', 'status', '-f', 'json', 'D:/repo/a.txt'],
      execute_args: ['protect', 'set', 'D:/repo/a.txt', '--deny', 'delete', '--require', 'force', '--system-acl'],
      preview_summary: '设置保护 D:/repo/a.txt',
    })
    expect(requests[1]?.execute_args).toEqual([
      'protect',
      'set',
      'D:/repo/b.txt',
      '--deny',
      'delete',
      '--require',
      'force',
      '--system-acl',
    ])
  })

  it('builds guarded preview requests for protect:clear', () => {
    const requests = createBatchGovernancePreviewRequests(
      'protect-clear',
      ['D:/repo/a.txt'],
      {
        ...createBatchGovernanceSharedState('protect-clear'),
        systemAcl: true,
      },
    )

    expect(requests[0]).toEqual({
      workspace: 'files-security',
      action: 'protect:clear',
      target: 'D:/repo/a.txt',
      preview_args: ['protect', 'status', '-f', 'json', 'D:/repo/a.txt'],
      execute_args: ['protect', 'clear', 'D:/repo/a.txt', '--system-acl'],
      preview_summary: '清除保护 D:/repo/a.txt',
    })
  })

  it('builds guarded preview requests for acl:purge', () => {
    const requests = createBatchGovernancePreviewRequests(
      'acl-purge',
      ['D:/repo/a.txt'],
      {
        ...createBatchGovernanceSharedState('acl-purge'),
        principal: 'BUILTIN\\Users',
      },
    )

    expect(requests[0]).toEqual({
      workspace: 'files-security',
      action: 'acl:purge',
      target: 'D:/repo/a.txt',
      preview_args: ['acl', 'view', '-p', 'D:/repo/a.txt', '--detail'],
      execute_args: ['acl', 'purge', '-p', 'D:/repo/a.txt', '--principal', 'BUILTIN\\Users', '-y'],
      preview_summary: '清理 D:/repo/a.txt 上 BUILTIN\\Users 的显式 ACL',
    })
  })

  it('builds guarded preview requests for acl:inherit', () => {
    const requests = createBatchGovernancePreviewRequests(
      'acl-inherit',
      ['D:/repo/a.txt'],
      {
        ...createBatchGovernanceSharedState('acl-inherit'),
        mode: 'disable',
        preserve: false,
      },
    )

    expect(requests[0]).toEqual({
      workspace: 'files-security',
      action: 'acl:inherit',
      target: 'D:/repo/a.txt',
      preview_args: ['acl', 'view', '-p', 'D:/repo/a.txt'],
      execute_args: ['acl', 'inherit', '-p', 'D:/repo/a.txt', '--disable', '--preserve', 'false'],
      preview_summary: '将 D:/repo/a.txt 的 ACL 继承切换为 禁用',
    })
  })

  it('creates an aggregated dialog preview and blocks confirm when any item is not ready', () => {
    const items: BatchGovernancePreviewItem[] = [
      {
        path: 'D:/repo/a.txt',
        preview: {
          token: 'token-a',
          workspace: 'files-security',
          action: 'protect:set',
          target: 'D:/repo/a.txt',
          phase: 'preview',
          status: 'previewed',
          guarded: true,
          dry_run: true,
          ready_to_execute: true,
          summary: '设置保护 D:/repo/a.txt',
          preview_summary: '设置保护 D:/repo/a.txt',
          expires_in_secs: 180,
          process: {
            command_line: 'xun protect status -f json D:/repo/a.txt',
            exit_code: 0,
            success: true,
            stdout: 'ok',
            stderr: '',
            duration_ms: 10,
          },
        },
      },
      {
        path: 'D:/repo/b.txt',
        error: '403 Forbidden: preview failed',
      },
    ]

    const dialogPreview = createBatchGovernanceDialogPreview('protect-set', items)

    expect(isBatchPreviewReady(items)).toBe(false)
    expect(summarizeBatchPreviews(items)).toEqual({ total: 2, ready: 1, blocked: 1 })
    expect(dialogPreview.ready_to_execute).toBe(false)
    expect(dialogPreview.summary).toBe('批量设置保护：1/2 项预演通过')
    expect(dialogPreview.process.stderr).toContain('403 Forbidden: preview failed')
  })

  it('summarizes batch receipts', () => {
    const items: BatchGovernanceReceiptItem[] = [
      {
        path: 'D:/repo/a.txt',
        receipt: {
          token: 'token-a',
          workspace: 'files-security',
          action: 'protect:set',
          target: 'D:/repo/a.txt',
          phase: 'execute',
          status: 'succeeded',
          guarded: true,
          dry_run: false,
          summary: '设置保护 D:/repo/a.txt',
          audit_action: 'workspace.protect.execute',
          audited_at: 1700000000,
          process: {
            command_line: 'xun protect set D:/repo/a.txt',
            exit_code: 0,
            success: true,
            stdout: 'ok',
            stderr: '',
            duration_ms: 11,
          },
        },
      },
      {
        path: 'D:/repo/b.txt',
        error: '500 Internal Server Error',
      },
    ]

    expect(summarizeBatchReceipts(items)).toEqual({ total: 2, succeeded: 1, failed: 1 })
  })
})
