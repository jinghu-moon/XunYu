import { flushPromises, mount } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { DiagnosticsSummaryResponse, RecentTaskRecord } from '../types'
import DiagnosticsCenterPanel from './DiagnosticsCenterPanel.vue'

const apiMocks = vi.hoisted(() => ({
  fetchWorkspaceDiagnosticsSummary: vi.fn(),
}))

vi.mock('../api', () => ({
  fetchWorkspaceDiagnosticsSummary: apiMocks.fetchWorkspaceDiagnosticsSummary,
}))

function createGovernanceAlert(
  overrides: {
    id?: string
    action?: string
    target?: string
    status?: RecentTaskRecord['status']
    summary?: string
    auditAction?: string | null
    createdAt?: number
    process?: Partial<RecentTaskRecord['process']>
    replay?: RecentTaskRecord['replay']
    details?: RecentTaskRecord['details']
  } = {},
): RecentTaskRecord {
  const target = overrides.target ?? 'D:/repo/demo.txt'
  const action = overrides.action ?? 'acl:owner'
  const baseSummary = overrides.summary ?? '?? D:/repo/demo.txt ? Owner'

  return {
    id: overrides.id ?? 'task-gov-1',
    workspace: 'files-security',
    action,
    target,
    mode: 'guarded',
    phase: 'execute',
    status: overrides.status ?? 'succeeded',
    guarded: true,
    dry_run: false,
    summary: baseSummary,
    created_at: overrides.createdAt ?? 1700000001,
    audit_action: overrides.auditAction ?? `dashboard.task.execute.${action}`,
    process: {
      command_line: 'xun acl owner -p D:/repo/demo.txt --set BUILTIN\Administrators -y',
      exit_code: 0,
      success: true,
      stdout: 'owner updated',
      stderr: '',
      duration_ms: 12,
      ...overrides.process,
    },
    details: overrides.details,
    replay: overrides.replay ?? {
      kind: 'guarded_preview',
      request: {
        workspace: 'files-security',
        action,
        target,
        preview_args: ['acl', 'view', '-p', target],
        execute_args: ['acl', 'owner', '-p', target, '--set', 'BUILTIN\Administrators', '-y'],
        preview_summary: baseSummary,
      },
    },
  }
}

function createSummary(options: {
  scope?: 'all' | 'user' | 'system'
  governanceAlerts?: RecentTaskRecord[]
  guardedReceipts?: RecentTaskRecord[]
} = {}): DiagnosticsSummaryResponse {
  const scope = options.scope ?? 'all'
  const governanceAlerts = options.governanceAlerts ?? [createGovernanceAlert()]
  const guardedReceipts = options.guardedReceipts ?? governanceAlerts
  const failedTask: RecentTaskRecord = {
    id: 'task-1',
    workspace: 'statistics-diagnostics',
    action: 'cstat',
    target: '.',
    mode: 'run',
    phase: 'run',
    status: 'failed',
    guarded: false,
    dry_run: false,
    summary: 'cstat .',
    created_at: 1700000000,
    audit_action: null,
    process: {
      command_line: 'xun cstat .',
      exit_code: 1,
      success: false,
      stdout: '',
      stderr: 'boom',
      duration_ms: 8,
    },
    replay: null,
  }
  return {
    generated_at: 1700000000,
    overview: {
      doctor_issues: 2,
      doctor_errors: 1,
      doctor_warnings: 1,
      doctor_fixable: 1,
      recent_failed_tasks: 1,
      recent_guarded_receipts: guardedReceipts.length,
      recent_governance_alerts: governanceAlerts.length,
      audit_entries: 3,
      urgent_items: 2,
    },
    doctor: {
      scope,
      issues: [
        {
          kind: 'path_missing',
          severity: 'error',
          scope,
          name: 'PATH',
          message: 'PATH contains missing entry',
          fixable: true,
        },
      ],
      errors: 1,
      warnings: 1,
      fixable: 1,
      load_error: null,
    },
    failed_tasks: [failedTask],
    guarded_receipts: guardedReceipts,
    governance_alerts: governanceAlerts,
    audit_timeline: [
      {
        timestamp: 1700000000,
        action: 'dashboard.task.preview',
        target: 'D:/tmp/demo.txt',
        user: 'dashboard',
        params: '{}',
        result: 'dry_run',
        reason: '',
      },
      {
        timestamp: 1700000001,
        action: 'dashboard.task.execute.acl:owner',
        target: 'D:/repo/demo.txt',
        user: 'dashboard',
        params: '{}',
        result: 'success',
        reason: '',
      },
    ],
  }
}

describe('DiagnosticsCenterPanel', () => {
  beforeEach(() => {
    apiMocks.fetchWorkspaceDiagnosticsSummary.mockReset()
  })

  it('loads and renders diagnosis aggregates with governance alerts', async () => {
    apiMocks.fetchWorkspaceDiagnosticsSummary.mockResolvedValue(createSummary())

    const wrapper = mount(DiagnosticsCenterPanel)
    await flushPromises()

    expect(apiMocks.fetchWorkspaceDiagnosticsSummary).toHaveBeenCalledWith('all')
    expect(wrapper.text()).toContain('???')
    expect(wrapper.text()).toContain('PATH contains missing entry')
    expect(wrapper.text()).toContain('cstat .')
    expect(wrapper.text()).toContain('??????')
    expect(wrapper.text()).toContain('ACL Owner')
    expect(wrapper.text()).toContain('Administrators')
    expect(wrapper.text()).toContain('dashboard.task.execute.acl:owner')
  })

  it('reloads when scope changes', async () => {
    apiMocks.fetchWorkspaceDiagnosticsSummary
      .mockResolvedValueOnce(createSummary({ scope: 'all' }))
      .mockResolvedValueOnce(createSummary({ scope: 'user' }))

    const wrapper = mount(DiagnosticsCenterPanel)
    await flushPromises()

    await wrapper.get('[data-testid="diagnostics-scope"]').setValue('user')
    await flushPromises()

    expect(apiMocks.fetchWorkspaceDiagnosticsSummary).toHaveBeenNthCalledWith(1, 'all')
    expect(apiMocks.fetchWorkspaceDiagnosticsSummary).toHaveBeenNthCalledWith(2, 'user')
  })

  it('renders structured acl diff details for governance alerts when available', async () => {
    const summary = createSummary({
      governanceAlerts: [
        createGovernanceAlert({
          id: 'task-gov-diff',
          action: 'acl:copy',
          target: 'D:/repo/a.txt',
          summary: 'copy acl',
          auditAction: 'dashboard.task.execute.acl:copy',
          process: {
            command_line: 'xun acl copy -p D:/repo/a.txt -r D:/repo/template.txt -y',
            stdout: 'copied',
          },
          details: {
            kind: 'acl_diff_transition',
            before: {
              target: 'D:/repo/a.txt',
              reference: 'D:/repo/template.txt',
              common_count: 1,
              has_diff: true,
              owner_diff: null,
              inheritance_diff: null,
              only_in_target: [
                {
                  principal: 'BUILTIN\Users',
                  sid: 'S-1-5-32-545',
                  rights: 'Read',
                  ace_type: 'Allow',
                  source: 'explicit',
                  inheritance: 'BothInherit',
                  propagation: 'None',
                  orphan: false,
                },
              ],
              only_in_reference: [],
            },
            after: {
              target: 'D:/repo/a.txt',
              reference: 'D:/repo/template.txt',
              common_count: 2,
              has_diff: false,
              owner_diff: null,
              inheritance_diff: null,
              only_in_target: [],
              only_in_reference: [],
            },
          },
          replay: {
            kind: 'guarded_preview',
            request: {
              workspace: 'files-security',
              action: 'acl:copy',
              target: 'D:/repo/a.txt',
              preview_args: ['acl', 'diff', '-p', 'D:/repo/a.txt', '-r', 'D:/repo/template.txt'],
              execute_args: ['acl', 'copy', '-p', 'D:/repo/a.txt', '-r', 'D:/repo/template.txt', '-y'],
              preview_summary: 'copy acl',
            },
          },
        }),
      ],
    })
    apiMocks.fetchWorkspaceDiagnosticsSummary.mockResolvedValue(summary)

    const wrapper = mount(DiagnosticsCenterPanel)
    await flushPromises()

    expect(wrapper.find('[data-testid="acl-diff-details"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="acl-diff-panel-before"]').exists()).toBe(true)
    expect(wrapper.text()).toContain('S-1-5-32-545')
  })

  it('filters governance alerts, groups them, and jumps to guarded receipts', async () => {
    const aclAlert = createGovernanceAlert()
    const protectAlert = createGovernanceAlert({
      id: 'task-gov-protect',
      action: 'protect:set',
      target: 'D:/repo/protect.txt',
      summary: '???? D:/repo/protect.txt',
      auditAction: 'dashboard.task.execute.protect:set',
      process: {
        command_line: 'xun protect set D:/repo/protect.txt --deny delete -y',
        stdout: 'protected',
      },
      replay: {
        kind: 'guarded_preview',
        request: {
          workspace: 'files-security',
          action: 'protect:set',
          target: 'D:/repo/protect.txt',
          preview_args: ['protect', 'status', '-f', 'json', 'D:/repo/protect.txt'],
          execute_args: ['protect', 'set', 'D:/repo/protect.txt', '--deny', 'delete'],
          preview_summary: '???? D:/repo/protect.txt',
        },
      },
    })
    const cryptAlert = createGovernanceAlert({
      id: 'task-gov-encrypt',
      action: 'encrypt',
      target: 'D:/repo/secret.txt',
      status: 'failed',
      summary: '?? D:/repo/secret.txt',
      auditAction: 'dashboard.task.execute.encrypt',
      process: {
        command_line: 'xun encrypt --to age1abc D:/repo/secret.txt',
        exit_code: 1,
        success: false,
        stdout: '',
        stderr: 'encrypt failed',
      },
      replay: {
        kind: 'guarded_preview',
        request: {
          workspace: 'files-security',
          action: 'encrypt',
          target: 'D:/repo/secret.txt',
          preview_args: ['find', '--dry-run', '-f', 'json', '--test-path', 'D:/repo/secret.txt'],
          execute_args: ['encrypt', '--to', 'age1abc', 'D:/repo/secret.txt'],
          preview_summary: '?? D:/repo/secret.txt',
        },
      },
    })
    const summary = createSummary({
      governanceAlerts: [aclAlert, protectAlert, cryptAlert],
      guardedReceipts: [aclAlert, protectAlert, cryptAlert],
    })
    apiMocks.fetchWorkspaceDiagnosticsSummary.mockResolvedValue(summary)

    const wrapper = mount(DiagnosticsCenterPanel)
    await flushPromises()

    const governancePanel = wrapper.get('[data-panel-id="governance"]')
    expect(governancePanel.text()).toContain('ACL')
    expect(governancePanel.text()).toContain('Protect')
    expect(governancePanel.text()).toContain('???')
    expect(governancePanel.findAll('[data-testid="diagnostics-governance-group"]')).toHaveLength(3)

    await wrapper.get('[data-testid="diagnostics-governance-family"]').setValue('crypt')
    await flushPromises()

    expect(governancePanel.text()).toContain('?? D:/repo/secret.txt')
    expect(governancePanel.text()).not.toContain('???? D:/repo/protect.txt')

    await wrapper.get('[data-testid="diagnostics-governance-status"]').setValue('failed')
    await flushPromises()

    expect(governancePanel.text()).toContain('?? 1 / 3 ?????')

    await wrapper.get('[data-testid="diagnostics-jump-guarded"]').trigger('click')

    expect(wrapper.get('[data-panel-id="guarded"]').classes()).toContain('is-active')
  })

  it('emits linked recent-task and audit focus requests from governance alerts', async () => {
    apiMocks.fetchWorkspaceDiagnosticsSummary.mockResolvedValue(createSummary())

    const wrapper = mount(DiagnosticsCenterPanel)
    await flushPromises()

    await wrapper.get('[data-testid="diagnostics-link-recent-governance-task-gov-1"]').trigger('click')
    await wrapper.get('[data-testid="diagnostics-link-audit-governance-task-gov-1"]').trigger('click')

    expect(wrapper.emitted('link-panel')).toHaveLength(2)
    expect(wrapper.emitted('link-panel')?.[0]?.[0]).toEqual({
      panel: 'recent-tasks',
      request: {
        selected_task_id: 'task-gov-1',
        status: 'succeeded',
        dry_run: 'executed',
      },
    })
    expect(wrapper.emitted('link-panel')?.[1]?.[0]).toEqual({
      panel: 'audit',
      request: {
        search: 'D:/repo/demo.txt',
        action: 'dashboard.task.execute.acl:owner',
        result: 'success',
      },
    })
  })

  it('applies reverse focus requests from consumer panels', async () => {
    apiMocks.fetchWorkspaceDiagnosticsSummary.mockResolvedValue(createSummary())

    const wrapper = mount(DiagnosticsCenterPanel)
    await flushPromises()

    await wrapper.setProps({
      focusRequest: {
        key: 1,
        panel: 'governance',
        governance_family: 'acl',
        governance_status: 'succeeded',
        task_id: 'task-gov-1',
        target: 'D:/repo/demo.txt',
        audit_action: 'dashboard.task.execute.acl:owner',
      },
    })
    await flushPromises()

    expect(wrapper.get('[data-panel-id="governance"]').classes()).toContain('is-active')
    expect((wrapper.get('[data-testid="diagnostics-governance-family"]').element as HTMLSelectElement).value).toBe('acl')
    expect((wrapper.get('[data-testid="diagnostics-governance-status"]').element as HTMLSelectElement).value).toBe('succeeded')
    expect(wrapper.get('[data-testid="diagnostics-link-recent-governance-task-gov-1"]').element.closest('article')?.className).toContain('is-active')

    await wrapper.setProps({
      focusRequest: {
        key: 2,
        panel: 'audit',
        target: 'D:/repo/demo.txt',
        audit_action: 'dashboard.task.execute.acl:owner',
        audit_result: 'success',
        audit_timestamp: 1700000001,
      },
    })
    await flushPromises()

    expect(wrapper.get('[data-panel-id="audit"]').classes()).toContain('is-active')
    const auditItem = wrapper
      .findAll('[data-panel-id="audit"] .diagnostics-center__item')
      .find((item) => item.text().includes('dashboard.task.execute.acl:owner'))
    expect(auditItem?.classes()).toContain('is-active')
  })

})
