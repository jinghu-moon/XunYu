import { describe, expect, it } from 'vitest'
import {
  buildBatchGovernancePlan,
  createBatchGovernanceDialogPreview,
  createBatchGovernanceItemForm,
  createBatchGovernancePreviewRequests,
  createBatchGovernanceSharedState,
  createRecentTasksFocusFromBatchPreview,
  createRecentTasksFocusFromBatchReceipt,
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

  it('builds guarded preview requests for encrypt', () => {
    const requests = createBatchGovernancePreviewRequests(
      'encrypt',
      ['D:/repo/a.txt'],
      {
        ...createBatchGovernanceSharedState('encrypt'),
        efs: false,
        to: 'age1abc\nage1def',
        out: 'D:/repo/a.txt.age',
      },
    )

    expect(requests[0]).toEqual({
      workspace: 'files-security',
      action: 'encrypt',
      target: 'D:/repo/a.txt',
      preview_args: ['find', '--dry-run', '-f', 'json', '--test-path', 'D:/repo/a.txt'],
      execute_args: ['encrypt', '--to', 'age1abc', '--to', 'age1def', '-o', 'D:/repo/a.txt.age', 'D:/repo/a.txt'],
      preview_summary: '加密 D:/repo/a.txt',
    })
  })

  it('builds guarded preview requests for decrypt', () => {
    const requests = createBatchGovernancePreviewRequests(
      'decrypt',
      ['D:/repo/a.txt'],
      {
        ...createBatchGovernanceSharedState('decrypt'),
        efs: false,
        identity: 'D:/keys/a.txt\nD:/keys/b.txt',
        out: 'D:/repo/a.clear.txt',
      },
    )

    expect(requests[0]).toEqual({
      workspace: 'files-security',
      action: 'decrypt',
      target: 'D:/repo/a.txt',
      preview_args: ['find', '--dry-run', '-f', 'json', '--test-path', 'D:/repo/a.txt'],
      execute_args: ['decrypt', '-i', 'D:/keys/a.txt', '-i', 'D:/keys/b.txt', '-o', 'D:/repo/a.clear.txt', 'D:/repo/a.txt'],
      preview_summary: '解密 D:/repo/a.txt',
    })
  })

  it('builds guarded preview requests for acl:add', () => {
    const requests = createBatchGovernancePreviewRequests(
      'acl-add',
      ['D:/repo/a.txt'],
      {
        ...createBatchGovernanceSharedState('acl-add'),
        principal: 'BUILTIN\Users',
        rights: 'Modify',
        aceType: 'Allow',
        inherit: 'ObjectOnly',
      },
    )

    expect(requests[0]).toEqual({
      workspace: 'files-security',
      action: 'acl:add',
      target: 'D:/repo/a.txt',
      preview_args: ['acl', 'view', '-p', 'D:/repo/a.txt', '--detail'],
      execute_args: [
        'acl',
        'add',
        '-p',
        'D:/repo/a.txt',
        '--principal',
        'BUILTIN\Users',
        '--rights',
        'Modify',
        '--ace-type',
        'Allow',
        '--inherit',
        'ObjectOnly',
        '-y',
      ],
      preview_summary: '为 D:/repo/a.txt 添加 ACL',
    })
  })

  it('builds guarded preview requests for acl:copy', () => {
    const requests = createBatchGovernancePreviewRequests(
      'acl-copy',
      ['D:/repo/a.txt'],
      {
        ...createBatchGovernanceSharedState('acl-copy'),
        reference: 'D:/repo/reference.txt',
      },
    )

    expect(requests[0]).toEqual({
      workspace: 'files-security',
      action: 'acl:copy',
      target: 'D:/repo/a.txt',
      preview_args: ['acl', 'diff', '-p', 'D:/repo/a.txt', '-r', 'D:/repo/reference.txt'],
      execute_args: ['acl', 'copy', '-p', 'D:/repo/a.txt', '-r', 'D:/repo/reference.txt', '-y'],
      preview_summary: '用 D:/repo/reference.txt 覆盖 D:/repo/a.txt 的 ACL',
    })
  })

  it('builds guarded preview requests for acl:restore', () => {
    const requests = createBatchGovernancePreviewRequests(
      'acl-restore',
      ['D:/repo/a.txt'],
      {
        ...createBatchGovernanceSharedState('acl-restore'),
        from: 'D:/repo/demo.acl.json',
      },
    )

    expect(requests[0]).toEqual({
      workspace: 'files-security',
      action: 'acl:restore',
      target: 'D:/repo/a.txt',
      preview_args: ['find', '--dry-run', '-f', 'json', '--test-path', 'D:/repo/demo.acl.json'],
      execute_args: ['acl', 'restore', '-p', 'D:/repo/a.txt', '--from', 'D:/repo/demo.acl.json', '-y'],
      preview_summary: '从 D:/repo/demo.acl.json 恢复 D:/repo/a.txt 的 ACL',
    })
  })

  it('builds recent tasks focus from batch preview items', () => {
    const focus = createRecentTasksFocusFromBatchPreview('encrypt', {
      path: 'D:/repo/a.txt',
      preview: { action: 'encrypt', status: 'previewed' } as any,
    })

    expect(focus).toEqual({
      status: 'previewed',
      dry_run: 'dry-run',
      action: 'encrypt',
      search: 'D:/repo/a.txt',
    })
  })

  it('builds recent tasks focus from batch receipt items', () => {
    const focus = createRecentTasksFocusFromBatchReceipt('protect-set', {
      path: 'D:/repo/a.txt',
      receipt: { action: 'protect:set', status: 'succeeded' } as any,
    })

    expect(focus).toEqual({
      status: 'succeeded',
      dry_run: 'executed',
      action: 'protect:set',
      search: 'D:/repo/a.txt',
    })
  })

  it('creates per-item form with path and shared values', () => {
    const form = createBatchGovernanceItemForm('protect-set', 'D:/repo/a.txt', {
      ...createBatchGovernanceSharedState('protect-set'),
      deny: 'delete',
      require: 'force',
      systemAcl: true,
    })

    expect(form).toMatchObject({
      path: 'D:/repo/a.txt',
      deny: 'delete',
      require: 'force',
      systemAcl: true,
    })
  })

  it('builds governance plan from shared fields', () => {
    const plan = buildBatchGovernancePlan('acl-inherit', ['D:/repo/a.txt', 'D:/repo/b.txt'], {
      ...createBatchGovernanceSharedState('acl-inherit'),
      mode: 'disable',
      preserve: false,
    })

    expect(plan.title).toBe('治理计划')
    expect(plan.note).toContain('逐项 dry-run')
    expect(plan.items).toEqual(
      expect.arrayContaining([
        { label: '治理动作', value: '批量切换 ACL 继承' },
        { label: '目标数量', value: '2 项' },
        { label: '执行模式', value: 'Triple-Guard：预演 → 确认 → 回执' },
        { label: '目标状态', value: '禁用继承' },
        { label: '禁用时保留继承 ACE', value: '否' },
      ]),
    )
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
