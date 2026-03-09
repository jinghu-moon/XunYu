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
    expect(wrapper.text()).toContain('D:/tmp/demo.txt')
    expect(wrapper.text()).not.toContain('cstat .')

    await wrapper.get('[data-testid="dryrun-filter"]').setValue('all')
    await wrapper.get('[data-testid="recent-search-filter"]').setValue('D:/tmp/demo.txt')
    expect(wrapper.text()).toContain('D:/tmp/demo.txt')
    expect(wrapper.text()).not.toContain('cstat .')

    await wrapper.get('[data-testid="recent-search-filter"]').setValue('')
    await wrapper.get('[data-testid="recent-action-filter"]').setValue('recent')
    expect(wrapper.text()).toContain('recent')
    expect(wrapper.text()).not.toContain('cstat .')
    expect(wrapper.text()).not.toContain('D:/tmp/demo.txt')
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

  it('renders structured acl diff details for selected recent task', async () => {
    apiMocks.fetchRecentWorkspaceTasks.mockResolvedValue({
      stats: { total: 1, succeeded: 1, failed: 0, dry_run: 0 },
      entries: [
        {
          id: 'task-1',
          workspace: 'files-security',
          action: 'acl:copy',
          target: 'D:/repo/a.txt',
          mode: 'guarded',
          phase: 'execute',
          status: 'succeeded',
          guarded: true,
          dry_run: false,
          summary: 'copy acl',
          created_at: 1700000000,
          audit_action: 'dashboard.task.execute.acl:copy',
          process: {
            command_line: 'xun acl copy -p D:/repo/a.txt -r D:/repo/template.txt -y',
            exit_code: 0,
            success: true,
            stdout: 'copied',
            stderr: '',
            duration_ms: 10,
          },
          details: {
            kind: 'acl_diff_transition',
            before: {
              target: 'D:/repo/a.txt',
              reference: 'D:/repo/template.txt',
              common_count: 2,
              has_diff: true,
              owner_diff: null,
              inheritance_diff: null,
              only_in_target: [
                {
                  principal: 'BUILTIN\\Users',
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
              common_count: 3,
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
        },
      ],
    })

    const wrapper = mount(RecentTasksPanel)
    await flushPromises()

    expect(wrapper.find('[data-testid="acl-diff-details"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="acl-diff-panel-before"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="acl-diff-panel-after"]').exists()).toBe(true)
    expect(wrapper.text()).toContain('S-1-5-32-545')
  })


  it('applies focus requests from the diagnostics workspace', async () => {
    const response = {
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
          summary: 'D:/tmp/demo.txt',
          created_at: 1700000000,
          audit_action: 'dashboard.task.preview',
          process: { command_line: 'xun rm --dry-run D:/tmp/demo.txt', exit_code: 0, success: true, stdout: 'preview ok', stderr: '', duration_ms: 10 },
          replay: { kind: 'guarded_preview', request: { workspace: 'files-security', action: 'rm', target: 'D:/tmp/demo.txt', preview_args: ['rm', '--dry-run', 'D:/tmp/demo.txt'], execute_args: ['rm', '-y', 'D:/tmp/demo.txt'], preview_summary: 'D:/tmp/demo.txt' } },
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
    }
    apiMocks.fetchRecentWorkspaceTasks.mockResolvedValue(response)

    const wrapper = mount(RecentTasksPanel)
    await flushPromises()

    await wrapper.setProps({
      focusRequest: {
        key: 1,
        selected_task_id: 'task-2',
        status: 'failed',
        dry_run: 'executed',
        search: 'cstat',
        action: 'cstat',
      },
    })
    await flushPromises()

    expect(apiMocks.fetchRecentWorkspaceTasks).toHaveBeenCalledTimes(2)
    expect((wrapper.get('[data-testid="status-filter"]').element as HTMLSelectElement).value).toBe('failed')
    expect((wrapper.get('[data-testid="dryrun-filter"]').element as HTMLSelectElement).value).toBe('executed')
    expect((wrapper.get('[data-testid="recent-search-filter"]').element as HTMLInputElement).value).toBe('cstat')
    expect((wrapper.get('[data-testid="recent-action-filter"]').element as HTMLSelectElement).value).toBe('cstat')
    expect(wrapper.get('[data-testid="recent-active-filters"]').text()).toContain('failed')
    expect(wrapper.get('[data-testid="recent-active-filters"]').text()).toContain('cstat')
    expect(wrapper.get('[data-testid="recent-active-filters"]').text()).toContain('cstat')
    expect(wrapper.get('[data-testid="recent-active-filters"]').text()).toContain('Dry Run')
    expect(wrapper.get('[data-testid="task-item-task-2"]').classes()).toContain('is-active')
    expect(wrapper.text()).toContain('cstat .')
    expect(wrapper.text()).not.toContain('D:/tmp/demo.txt')

    await wrapper.get('[data-testid="clear-recent-filters"]').trigger('click')
    await flushPromises()

    expect((wrapper.get('[data-testid="status-filter"]').element as HTMLSelectElement).value).toBe('all')
    expect((wrapper.get('[data-testid="dryrun-filter"]').element as HTMLSelectElement).value).toBe('all')
    expect((wrapper.get('[data-testid="recent-search-filter"]').element as HTMLInputElement).value).toBe('')
    expect((wrapper.get('[data-testid="recent-action-filter"]').element as HTMLSelectElement).value).toBe('')
    expect(wrapper.find('[data-testid="recent-active-filters"]').exists()).toBe(false)
  })

  it('emits diagnostics-center focus requests from selected records', async () => {
    apiMocks.fetchRecentWorkspaceTasks.mockResolvedValue({
      stats: { total: 1, succeeded: 0, failed: 1, dry_run: 0 },
      entries: [
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
      ],
    })

    const wrapper = mount(RecentTasksPanel)
    await flushPromises()

    await wrapper.get('[data-testid="recent-link-diagnostics"]').trigger('click')

    expect(wrapper.emitted('link-panel')).toHaveLength(1)
    expect(wrapper.emitted('link-panel')?.[0]?.[0]).toEqual({
      panel: 'diagnostics-center',
      request: {
        panel: 'failed',
        task_id: 'task-2',
        target: '.',
      },
    })
  })

})
