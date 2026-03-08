import { flushPromises, mount } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import RecentTasksPanel from './RecentTasksPanel.vue'

const apiMocks = vi.hoisted(() => ({
  fetchRecentWorkspaceTasks: vi.fn(),
  runWorkspaceTask: vi.fn(),
  previewGuardedTask: vi.fn(),
  executeGuardedTask: vi.fn(),
}))

vi.mock('../api', () => ({
  fetchRecentWorkspaceTasks: apiMocks.fetchRecentWorkspaceTasks,
  runWorkspaceTask: apiMocks.runWorkspaceTask,
  previewGuardedTask: apiMocks.previewGuardedTask,
  executeGuardedTask: apiMocks.executeGuardedTask,
}))

describe('RecentTasksPanel', () => {
  beforeEach(() => {
    apiMocks.fetchRecentWorkspaceTasks.mockReset()
    apiMocks.runWorkspaceTask.mockReset()
    apiMocks.previewGuardedTask.mockReset()
    apiMocks.executeGuardedTask.mockReset()
    document.body.innerHTML = ''
  })

  it('passes workspace filter to recent-task api', async () => {
    apiMocks.fetchRecentWorkspaceTasks.mockResolvedValue({
      stats: { total: 0, succeeded: 0, failed: 0, dry_run: 0 },
      entries: [],
    })

    mount(RecentTasksPanel, { props: { workspace: 'files-security', limit: 12 } })
    await flushPromises()

    expect(apiMocks.fetchRecentWorkspaceTasks).toHaveBeenCalledWith(12, 'files-security')
  })

  it('loads and filters recent tasks', async () => {
    apiMocks.fetchRecentWorkspaceTasks.mockResolvedValue({
      stats: { total: 3, succeeded: 1, failed: 1, dry_run: 1 },
      entries: [
        {
          id: 'task-1',
          workspace: 'files-security',
          action: 'rm',
          target: 'D:/tmp/demo.txt',
          mode: 'guarded',
          phase: 'preview',
          status: 'previewed',
          guarded: true,
          dry_run: true,
          summary: '删除 demo.txt',
          created_at: 1700000000,
          audit_action: 'dashboard.task.preview',
          process: { command_line: 'xun rm --dry-run D:/tmp/demo.txt', exit_code: 0, success: true, stdout: 'preview ok', stderr: '', duration_ms: 10 },
          replay: { kind: 'guarded_preview', request: { workspace: 'files-security', action: 'rm', target: 'D:/tmp/demo.txt', preview_args: ['rm', '--dry-run', 'D:/tmp/demo.txt'], execute_args: ['rm', '-y', 'D:/tmp/demo.txt'], preview_summary: '删除 demo.txt' } },
        },
        {
          id: 'task-2',
          workspace: 'statistics-diagnostics',
          action: 'cstat',
          target: '.',
          mode: 'run',
          phase: 'run',
          status: 'failed',
          guarded: false,
          dry_run: false,
          summary: 'cstat .',
          created_at: 1700000001,
          audit_action: null,
          process: { command_line: 'xun cstat .', exit_code: 1, success: false, stdout: '', stderr: 'boom', duration_ms: 11 },
          replay: { kind: 'run', request: { workspace: 'statistics-diagnostics', action: 'cstat', target: '.', args: ['cstat', '.'] } },
        },
        {
          id: 'task-3',
          workspace: 'paths-context',
          action: 'recent',
          target: '',
          mode: 'run',
          phase: 'run',
          status: 'succeeded',
          guarded: false,
          dry_run: false,
          summary: 'recent',
          created_at: 1700000002,
          audit_action: null,
          process: { command_line: 'xun recent', exit_code: 0, success: true, stdout: '[]', stderr: '', duration_ms: 12 },
          replay: { kind: 'run', request: { workspace: 'paths-context', action: 'recent', target: '', args: ['recent'] } },
        },
      ],
    })

    const wrapper = mount(RecentTasksPanel)
    await flushPromises()

    expect(wrapper.text()).toContain('删除 demo.txt')
    expect(wrapper.text()).toContain('cstat .')
    await wrapper.get('[data-testid="status-filter"]').setValue('failed')
    expect(wrapper.text()).toContain('cstat .')
    expect(wrapper.text()).not.toContain('删除 demo.txt')

    await wrapper.get('[data-testid="status-filter"]').setValue('all')
    await wrapper.get('[data-testid="dryrun-filter"]').setValue('dry-run')
    expect(wrapper.text()).toContain('删除 demo.txt')
    expect(wrapper.text()).not.toContain('cstat .')
  })

  it('replays run tasks directly', async () => {
    apiMocks.fetchRecentWorkspaceTasks
      .mockResolvedValueOnce({
        stats: { total: 1, succeeded: 1, failed: 0, dry_run: 0 },
        entries: [
          {
            id: 'task-1', workspace: 'paths-context', action: 'recent', target: '', mode: 'run', phase: 'run', status: 'succeeded', guarded: false, dry_run: false, summary: 'recent', created_at: 1700000002, audit_action: null,
            process: { command_line: 'xun recent', exit_code: 0, success: true, stdout: '[]', stderr: '', duration_ms: 12 },
            replay: { kind: 'run', request: { workspace: 'paths-context', action: 'recent', target: '', args: ['recent'] } },
          },
        ],
      })
      .mockResolvedValueOnce({
        stats: { total: 2, succeeded: 2, failed: 0, dry_run: 0 },
        entries: [
          {
            id: 'task-2', workspace: 'paths-context', action: 'recent', target: '', mode: 'run', phase: 'run', status: 'succeeded', guarded: false, dry_run: false, summary: 'recent replay', created_at: 1700000003, audit_action: null,
            process: { command_line: 'xun recent', exit_code: 0, success: true, stdout: '[1]', stderr: '', duration_ms: 8 },
            replay: { kind: 'run', request: { workspace: 'paths-context', action: 'recent', target: '', args: ['recent'] } },
          },
        ],
      })
    apiMocks.runWorkspaceTask.mockResolvedValue({
      workspace: 'paths-context',
      action: 'recent',
      target: '',
      process: { command_line: 'xun recent', exit_code: 0, success: true, stdout: '[1]', stderr: '', duration_ms: 8 },
    })

    const wrapper = mount(RecentTasksPanel)
    await flushPromises()
    await wrapper.get('[data-testid="replay-button"]').trigger('click')
    await flushPromises()

    expect(apiMocks.runWorkspaceTask).toHaveBeenCalledTimes(1)
    expect(wrapper.text()).toContain('重放结果')
  })

  it('replays guarded tasks through preview and confirm', async () => {
    apiMocks.fetchRecentWorkspaceTasks
      .mockResolvedValueOnce({
        stats: { total: 1, succeeded: 0, failed: 0, dry_run: 1 },
        entries: [
          {
            id: 'task-1', workspace: 'files-security', action: 'rm', target: 'D:/tmp/demo.txt', mode: 'guarded', phase: 'preview', status: 'previewed', guarded: true, dry_run: true, summary: '删除 demo.txt', created_at: 1700000000, audit_action: 'dashboard.task.preview',
            process: { command_line: 'xun rm --dry-run D:/tmp/demo.txt', exit_code: 0, success: true, stdout: 'preview ok', stderr: '', duration_ms: 10 },
            replay: { kind: 'guarded_preview', request: { workspace: 'files-security', action: 'rm', target: 'D:/tmp/demo.txt', preview_args: ['rm', '--dry-run', 'D:/tmp/demo.txt'], execute_args: ['rm', '-y', 'D:/tmp/demo.txt'], preview_summary: '删除 demo.txt' } },
          },
        ],
      })
      .mockResolvedValueOnce({
        stats: { total: 2, succeeded: 1, failed: 0, dry_run: 1 },
        entries: [
          {
            id: 'task-2', workspace: 'files-security', action: 'rm', target: 'D:/tmp/demo.txt', mode: 'guarded', phase: 'execute', status: 'succeeded', guarded: true, dry_run: false, summary: '删除 demo.txt', created_at: 1700000001, audit_action: 'dashboard.task.execute.rm',
            process: { command_line: 'xun rm -y D:/tmp/demo.txt', exit_code: 0, success: true, stdout: 'deleted', stderr: '', duration_ms: 14 },
            replay: { kind: 'guarded_preview', request: { workspace: 'files-security', action: 'rm', target: 'D:/tmp/demo.txt', preview_args: ['rm', '--dry-run', 'D:/tmp/demo.txt'], execute_args: ['rm', '-y', 'D:/tmp/demo.txt'], preview_summary: '删除 demo.txt' } },
          },
        ],
      })
    apiMocks.previewGuardedTask.mockResolvedValue({
      token: 'token-1', workspace: 'files-security', action: 'rm', target: 'D:/tmp/demo.txt', phase: 'preview', status: 'previewed', guarded: true, dry_run: true, ready_to_execute: true, summary: '删除 demo.txt', preview_summary: '删除 demo.txt', expires_in_secs: 300,
      process: { command_line: 'xun rm --dry-run D:/tmp/demo.txt', exit_code: 0, success: true, stdout: 'preview ok', stderr: '', duration_ms: 10 },
    })
    apiMocks.executeGuardedTask.mockResolvedValue({
      token: 'token-1', workspace: 'files-security', action: 'rm', target: 'D:/tmp/demo.txt', phase: 'execute', status: 'succeeded', guarded: true, dry_run: false, summary: '删除 demo.txt', audit_action: 'workspace.rm.execute', audited_at: 1700000000,
      process: { command_line: 'xun rm -y D:/tmp/demo.txt', exit_code: 0, success: true, stdout: 'deleted', stderr: '', duration_ms: 14 },
    })

    const wrapper = mount(RecentTasksPanel, { attachTo: document.body })
    await flushPromises()
    await wrapper.get('[data-testid="replay-button"]').trigger('click')
    await flushPromises()

    expect(apiMocks.previewGuardedTask).toHaveBeenCalledTimes(1)
    expect(document.body.textContent || '').toContain('确认执行')

    const confirmButton = [...document.body.querySelectorAll('button')].find((button) =>
      button.textContent?.includes('确认执行'),
    )
    expect(confirmButton).toBeTruthy()
    ;(confirmButton as HTMLButtonElement).click()
    await flushPromises()

    expect(apiMocks.executeGuardedTask).toHaveBeenCalledWith({ token: 'token-1', confirm: true })
    expect(wrapper.text()).toContain('执行回执')
  })


  it('renders governance summary for selected ACL task details', async () => {
    apiMocks.fetchRecentWorkspaceTasks.mockResolvedValue({
      stats: { total: 1, succeeded: 0, failed: 0, dry_run: 1 },
      entries: [
        {
          id: 'task-1',
          workspace: 'files-security',
          action: 'acl:owner',
          target: 'D:/repo/demo.txt',
          mode: 'guarded',
          phase: 'preview',
          status: 'previewed',
          guarded: true,
          dry_run: true,
          summary: '修改 D:/repo/demo.txt 的 Owner',
          created_at: 1700000000,
          audit_action: 'dashboard.task.preview',
          process: {
            command_line: 'xun acl view -p D:/repo/demo.txt',
            exit_code: 0,
            success: true,
            stdout: ['Path: D:/repo/demo.txt', 'Owner: NT AUTHORITY\\SYSTEM | Inherit: enabled', 'Total: 4 (Allow 4 / Deny 0)  Explicit 1  Inherited 3  Orphan 0'].join('\n'),
            stderr: '',
            duration_ms: 10,
          },
          replay: {
            kind: 'guarded_preview',
            request: {
              workspace: 'files-security',
              action: 'acl:owner',
              target: 'D:/repo/demo.txt',
              preview_args: ['acl', 'view', '-p', 'D:/repo/demo.txt'],
              execute_args: ['acl', 'owner', '-p', 'D:/repo/demo.txt', '--set', 'BUILTIN\\Administrators', '-y'],
              preview_summary: '修改 D:/repo/demo.txt 的 Owner',
            },
          },
        },
      ],
    })

    const wrapper = mount(RecentTasksPanel)
    await flushPromises()

    expect(wrapper.text()).toContain('ACL Owner 预演摘要')
    expect(wrapper.text()).toContain('NT AUTHORITY\\SYSTEM')
    expect(wrapper.text()).toContain('BUILTIN\\Administrators')
  })

  it('renders governance summary for replayed run results', async () => {
    apiMocks.fetchRecentWorkspaceTasks
      .mockResolvedValueOnce({
        stats: { total: 1, succeeded: 1, failed: 0, dry_run: 0 },
        entries: [
          {
            id: 'task-1',
            workspace: 'files-security',
            action: 'acl:backup',
            target: 'D:/repo/demo.txt',
            mode: 'run',
            phase: 'run',
            status: 'succeeded',
            guarded: false,
            dry_run: false,
            summary: '备份 D:/repo/demo.txt 的 ACL',
            created_at: 1700000000,
            audit_action: null,
            process: {
              command_line: 'xun acl backup -p D:/repo/demo.txt -o D:/repo/demo.acl.json',
              exit_code: 0,
              success: true,
              stdout: 'Backed up 6 entries -> D:/repo/demo.acl.json',
              stderr: '',
              duration_ms: 9,
            },
            replay: {
              kind: 'run',
              request: {
                workspace: 'files-security',
                action: 'acl:backup',
                target: 'D:/repo/demo.txt',
                args: ['acl', 'backup', '-p', 'D:/repo/demo.txt', '-o', 'D:/repo/demo.acl.json'],
              },
            },
          },
        ],
      })
      .mockResolvedValueOnce({
        stats: { total: 2, succeeded: 2, failed: 0, dry_run: 0 },
        entries: [],
      })
    apiMocks.runWorkspaceTask.mockResolvedValue({
      workspace: 'files-security',
      action: 'acl:backup',
      target: 'D:/repo/demo.txt',
      process: {
        command_line: 'xun acl backup -p D:/repo/demo.txt -o D:/repo/demo.acl.json',
        exit_code: 0,
        success: true,
        stdout: 'Backed up 6 entries -> D:/repo/demo.acl.json',
        stderr: '',
        duration_ms: 9,
      },
    })

    const wrapper = mount(RecentTasksPanel)
    await flushPromises()
    await wrapper.get('[data-testid="replay-button"]').trigger('click')
    await flushPromises()

    expect(wrapper.text()).toContain('ACL 备份摘要')
    expect(wrapper.text()).toContain('D:/repo/demo.acl.json')
    expect(wrapper.text()).toContain('6 条')
  })

})
