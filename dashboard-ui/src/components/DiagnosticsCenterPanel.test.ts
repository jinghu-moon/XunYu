import { flushPromises, mount } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import DiagnosticsCenterPanel from './DiagnosticsCenterPanel.vue'

const apiMocks = vi.hoisted(() => ({
  fetchWorkspaceDiagnosticsSummary: vi.fn(),
}))

vi.mock('../api', () => ({
  fetchWorkspaceDiagnosticsSummary: apiMocks.fetchWorkspaceDiagnosticsSummary,
}))

function createSummary(scope: 'all' | 'user' | 'system' = 'all') {
  return {
    generated_at: 1700000000,
    overview: {
      doctor_issues: 2,
      doctor_errors: 1,
      doctor_warnings: 1,
      doctor_fixable: 1,
      recent_failed_tasks: 1,
      recent_guarded_receipts: 1,
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
    failed_tasks: [
      {
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
      },
    ],
    guarded_receipts: [
      {
        id: 'task-2',
        workspace: 'files-security',
        action: 'rm',
        target: 'D:/tmp/demo.txt',
        mode: 'guarded',
        phase: 'execute',
        status: 'succeeded',
        guarded: true,
        dry_run: false,
        summary: '删除 D:/tmp/demo.txt',
        created_at: 1700000001,
        audit_action: 'dashboard.task.execute.rm',
        process: {
          command_line: 'xun rm -y D:/tmp/demo.txt',
          exit_code: 0,
          success: true,
          stdout: 'deleted',
          stderr: '',
          duration_ms: 12,
        },
        replay: null,
      },
    ],
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
        action: 'dashboard.task.execute.rm',
        target: 'D:/tmp/demo.txt',
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

  it('loads and renders diagnosis aggregates', async () => {
    apiMocks.fetchWorkspaceDiagnosticsSummary.mockResolvedValue(createSummary())

    const wrapper = mount(DiagnosticsCenterPanel)
    await flushPromises()

    expect(apiMocks.fetchWorkspaceDiagnosticsSummary).toHaveBeenCalledWith('all')
    expect(wrapper.text()).toContain('紧急项')
    expect(wrapper.text()).toContain('PATH contains missing entry')
    expect(wrapper.text()).toContain('cstat .')
    expect(wrapper.text()).toContain('删除 D:/tmp/demo.txt')
    expect(wrapper.text()).toContain('dashboard.task.execute.rm')
  })

  it('reloads when scope changes', async () => {
    apiMocks.fetchWorkspaceDiagnosticsSummary
      .mockResolvedValueOnce(createSummary('all'))
      .mockResolvedValueOnce(createSummary('user'))

    const wrapper = mount(DiagnosticsCenterPanel)
    await flushPromises()

    await wrapper.get('[data-testid="diagnostics-scope"]').setValue('user')
    await flushPromises()

    expect(apiMocks.fetchWorkspaceDiagnosticsSummary).toHaveBeenNthCalledWith(1, 'all')
    expect(apiMocks.fetchWorkspaceDiagnosticsSummary).toHaveBeenNthCalledWith(2, 'user')
  })
})
