import { flushPromises, mount } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { WorkspaceTaskDefinition } from '../workspace-tools'
import TaskToolCard from './TaskToolCard.vue'

const apiMocks = vi.hoisted(() => ({
  runWorkspaceTask: vi.fn(),
  previewGuardedTask: vi.fn(),
  executeGuardedTask: vi.fn(),
}))

vi.mock('../api', () => ({
  runWorkspaceTask: apiMocks.runWorkspaceTask,
  previewGuardedTask: apiMocks.previewGuardedTask,
  executeGuardedTask: apiMocks.executeGuardedTask,
}))

describe('TaskToolCard', () => {
  beforeEach(() => {
    apiMocks.runWorkspaceTask.mockReset()
    apiMocks.previewGuardedTask.mockReset()
    apiMocks.executeGuardedTask.mockReset()
    document.body.innerHTML = ''
  })

  it('runs non-guarded tasks directly and renders output', async () => {
    const task: WorkspaceTaskDefinition = {
      id: 'recent',
      workspace: 'paths-context',
      title: '最近访问',
      description: '查看最近书签',
      action: 'recent',
      mode: 'run',
      fields: [{ key: 'limit', label: '鏁伴噺', type: 'number', defaultValue: '10' }],
      buildRunArgs: () => ['recent', '-n', '10', '-f', 'json'],
    }

    apiMocks.runWorkspaceTask.mockResolvedValue({
      workspace: 'paths-context',
      action: 'recent',
      target: '',
      process: {
        command_line: 'xun recent -n 10 -f json',
        exit_code: 0,
        success: true,
        stdout: '[1,2,3]',
        stderr: '',
        duration_ms: 10,
      },
    })

    const wrapper = mount(TaskToolCard, { props: { task }, attachTo: document.body })
    await wrapper.get('button').trigger('click')
    await flushPromises()

    expect(apiMocks.runWorkspaceTask).toHaveBeenCalledTimes(1)
    expect(wrapper.text()).toContain('[1,2,3]')
    expect(wrapper.text()).toContain('成功')
  })

  it('renders governance execute summary for acl:diff run tasks', async () => {
    const task: WorkspaceTaskDefinition = {
      id: 'acl-diff',
      workspace: 'files-security',
      title: 'ACL 差异摘要',
      description: '比较 ACL 差异并输出摘要。',
      action: 'acl:diff',
      mode: 'run',
      fields: [
        { key: 'path', label: '路径', type: 'text', required: true, defaultValue: 'D:/tmp/a.txt' },
        { key: 'reference', label: '参考路径', type: 'text', required: true, defaultValue: 'D:/tmp/b.txt' },
        { key: 'output', label: '输出 CSV', type: 'text', defaultValue: 'D:/tmp/acl-diff.csv' },
      ],
      target: () => 'D:/tmp/a.txt',
      buildRunArgs: () => ['acl', 'diff', '-p', 'D:/tmp/a.txt', '-r', 'D:/tmp/b.txt', '-o', 'D:/tmp/acl-diff.csv'],
    }

    apiMocks.runWorkspaceTask.mockResolvedValue({
      workspace: 'files-security',
      action: 'acl:diff',
      target: 'D:/tmp/a.txt',
      process: {
        command_line: 'xun acl diff -p D:/tmp/a.txt -r D:/tmp/b.txt -o D:/tmp/acl-diff.csv',
        exit_code: 0,
        success: true,
        stdout: [
          'Path: D:/tmp/a.txt',
          'Reference: D:/tmp/b.txt',
          'Owner differs',
          'Only in A: 2',
          'Only in B: 1',
          'Common: 5',
          'Exported 3 rows to D:/tmp/acl-diff.csv',
        ].join('\n'),
        stderr: '',
        duration_ms: 12,
      },
    })

    const wrapper = mount(TaskToolCard, { props: { task }, attachTo: document.body })
    await wrapper.get('button').trigger('click')
    await flushPromises()

    expect(wrapper.text()).toContain('ACL 差异摘要')
    expect(wrapper.text()).toContain('D:/tmp/b.txt')
    expect(wrapper.text()).toContain('2 条')
    expect(wrapper.text()).toContain('D:/tmp/acl-diff.csv')
  })

  it('enforces preview before guarded execution and shows receipt after confirm', async () => {
    const task: WorkspaceTaskDefinition = {
      id: 'rm',
      workspace: 'files-security',
      title: '删除文件',
      description: '危险动作',
      action: 'rm',
      mode: 'guarded',
      tone: 'danger',
      fields: [{ key: 'path', label: '路径', type: 'text', required: true, defaultValue: 'D:/tmp/demo.txt' }],
      target: () => 'D:/tmp/demo.txt',
      buildPreviewArgs: () => ['rm', '--dry-run', 'D:/tmp/demo.txt'],
      buildExecuteArgs: () => ['rm', '-y', 'D:/tmp/demo.txt'],
      previewSummary: () => '删除 D:/tmp/demo.txt',
    }

    apiMocks.previewGuardedTask.mockResolvedValue({
      token: 'token-1',
      workspace: 'files-security',
      action: 'rm',
      target: 'D:/tmp/demo.txt',
      phase: 'preview',
      status: 'previewed',
      guarded: true,
      dry_run: true,
      ready_to_execute: true,
      summary: '删除 D:/tmp/demo.txt',
      preview_summary: '删除 D:/tmp/demo.txt',
      expires_in_secs: 300,
      process: {
        command_line: 'xun rm --dry-run D:/tmp/demo.txt',
        exit_code: 0,
        success: true,
        stdout: 'preview ok',
        stderr: '',
        duration_ms: 10,
      },
    })
    apiMocks.executeGuardedTask.mockResolvedValue({
      token: 'token-1',
      workspace: 'files-security',
      action: 'rm',
      target: 'D:/tmp/demo.txt',
      phase: 'execute',
      status: 'succeeded',
      guarded: true,
      dry_run: false,
      summary: '删除 D:/tmp/demo.txt',
      audit_action: 'workspace.rm.execute',
      audited_at: 1700000000,
      process: {
        command_line: 'xun rm -y D:/tmp/demo.txt',
        exit_code: 0,
        success: true,
        stdout: 'deleted',
        stderr: '',
        duration_ms: 15,
      },
    })

    const wrapper = mount(TaskToolCard, { props: { task }, attachTo: document.body })
    await wrapper.get('button').trigger('click')
    await flushPromises()

    expect(apiMocks.previewGuardedTask).toHaveBeenCalledTimes(1)
    expect(apiMocks.executeGuardedTask).not.toHaveBeenCalled()
    expect(wrapper.text()).toContain('preview ok')
    expect(wrapper.text()).toContain('待确认')
    expect(document.body.textContent || '').toContain('确认执行')

    const confirmButton = [...document.body.querySelectorAll('button')].find((button) =>
      button.textContent?.includes('确认执行'),
    )
    expect(confirmButton).toBeTruthy()
    ;(confirmButton as HTMLButtonElement).click()
    await flushPromises()

    expect(apiMocks.executeGuardedTask).toHaveBeenCalledWith({ token: 'token-1', confirm: true })
    expect(wrapper.text()).toContain('执行回执')
    expect(wrapper.text()).toContain('deleted')
    expect(wrapper.text()).toContain('成功')
  })

  it('renders governance preview summary for protect:set tasks', async () => {
    const task: WorkspaceTaskDefinition = {
      id: 'protect-set',
      workspace: 'files-security',
      title: '设置保护规则',
      description: '预演后确认写入保护规则。',
      action: 'protect:set',
      mode: 'guarded',
      tone: 'danger',
      feature: 'protect',
      fields: [
        { key: 'path', label: '路径', type: 'text', required: true, defaultValue: 'D:/tmp/demo.txt' },
        { key: 'deny', label: '拒绝权限', type: 'text', defaultValue: 'delete,move,rename' },
        { key: 'require', label: '必需条件', type: 'text', defaultValue: 'force,reason' },
        { key: 'systemAcl', label: '写入系统 ACL', type: 'checkbox', defaultValue: true },
      ],
      target: () => 'D:/tmp/demo.txt',
      buildPreviewArgs: () => ['protect', 'status', '-f', 'json', 'D:/tmp/demo.txt'],
      buildExecuteArgs: () => ['protect', 'set', 'D:/tmp/demo.txt', '--deny', 'delete,move,rename', '--require', 'force,reason', '--system-acl'],
      previewSummary: () => '保护 D:/tmp/demo.txt',
    }

    apiMocks.previewGuardedTask.mockResolvedValue({
      token: 'token-2',
      workspace: 'files-security',
      action: 'protect:set',
      target: 'D:/tmp/demo.txt',
      phase: 'preview',
      status: 'previewed',
      guarded: true,
      dry_run: true,
      ready_to_execute: true,
      summary: '保护 D:/tmp/demo.txt',
      preview_summary: '保护 D:/tmp/demo.txt',
      expires_in_secs: 300,
      process: {
        command_line: 'xun protect status -f json D:/tmp/demo.txt',
        exit_code: 0,
        success: true,
        stdout: '[{"path":"D:/tmp/demo.txt","deny":["delete"],"require":["force"]}]',
        stderr: '',
        duration_ms: 9,
      },
    })

    const wrapper = mount(TaskToolCard, { props: { task }, attachTo: document.body })
    await wrapper.get('button').trigger('click')
    await flushPromises()

    expect(wrapper.get('[data-testid="governance-summary-preview"]').text()).toContain('保护变更预演摘要')
    expect(wrapper.text()).toContain('更新现有保护规则')
    expect(wrapper.text()).toContain('delete / move / rename')
    expect(document.body.querySelector('[data-testid="confirm-dialog-extra"]')?.textContent || '').toContain('保护变更预演摘要')
  })

  it('stays out of confirm state when preview fails', async () => {
    const task: WorkspaceTaskDefinition = {
      id: 'rm',
      workspace: 'files-security',
      title: '删除文件',
      description: '危险动作',
      action: 'rm',
      mode: 'guarded',
      tone: 'danger',
      fields: [{ key: 'path', label: '路径', type: 'text', required: true, defaultValue: 'D:/tmp/demo.txt' }],
      target: () => 'D:/tmp/demo.txt',
      buildPreviewArgs: () => ['rm', '--dry-run', 'D:/tmp/demo.txt'],
      buildExecuteArgs: () => ['rm', '-y', 'D:/tmp/demo.txt'],
    }

    apiMocks.previewGuardedTask.mockRejectedValue(new Error('400 Bad Request: preview failed'))

    const wrapper = mount(TaskToolCard, { props: { task }, attachTo: document.body })
    await wrapper.get('button').trigger('click')
    await flushPromises()

    expect(apiMocks.previewGuardedTask).toHaveBeenCalledTimes(1)
    expect(apiMocks.executeGuardedTask).not.toHaveBeenCalled()
    expect(wrapper.text()).toContain('400 Bad Request: preview failed')
    expect(wrapper.text()).toContain('失败')
    expect(document.body.textContent || '').not.toContain('确认执行')
  })

  it('renders failed receipt when guarded execution returns failed process', async () => {
    const task: WorkspaceTaskDefinition = {
      id: 'rm',
      workspace: 'files-security',
      title: '删除文件',
      description: '危险动作',
      action: 'rm',
      mode: 'guarded',
      tone: 'danger',
      fields: [{ key: 'path', label: '路径', type: 'text', required: true, defaultValue: 'D:/tmp/demo.txt' }],
      target: () => 'D:/tmp/demo.txt',
      buildPreviewArgs: () => ['rm', '--dry-run', 'D:/tmp/demo.txt'],
      buildExecuteArgs: () => ['rm', '-y', 'D:/tmp/demo.txt'],
      previewSummary: () => '删除 D:/tmp/demo.txt',
    }

    apiMocks.previewGuardedTask.mockResolvedValue({
      token: 'token-1',
      workspace: 'files-security',
      action: 'rm',
      target: 'D:/tmp/demo.txt',
      phase: 'preview',
      status: 'previewed',
      guarded: true,
      dry_run: true,
      ready_to_execute: true,
      summary: '删除 D:/tmp/demo.txt',
      preview_summary: '删除 D:/tmp/demo.txt',
      expires_in_secs: 300,
      process: {
        command_line: 'xun rm --dry-run D:/tmp/demo.txt',
        exit_code: 0,
        success: true,
        stdout: 'preview ok',
        stderr: '',
        duration_ms: 10,
      },
    })
    apiMocks.executeGuardedTask.mockResolvedValue({
      token: 'token-1',
      workspace: 'files-security',
      action: 'rm',
      target: 'D:/tmp/demo.txt',
      phase: 'execute',
      status: 'failed',
      guarded: true,
      dry_run: false,
      summary: '删除 D:/tmp/demo.txt',
      audit_action: 'workspace.rm.execute',
      audited_at: 1700000000,
      process: {
        command_line: 'xun rm -y D:/tmp/demo.txt',
        exit_code: 1,
        success: false,
        stdout: '',
        stderr: 'access denied',
        duration_ms: 15,
      },
    })

    const wrapper = mount(TaskToolCard, { props: { task }, attachTo: document.body })
    await wrapper.get('button').trigger('click')
    await flushPromises()

    const confirmButton = [...document.body.querySelectorAll('button')].find((button) =>
      button.textContent?.includes('确认执行'),
    )
    expect(confirmButton).toBeTruthy()
    ;(confirmButton as HTMLButtonElement).click()
    await flushPromises()

    expect(wrapper.text()).toContain('执行回执')
    expect(wrapper.text()).toContain('access denied')
    expect(wrapper.text()).toContain('失败')
  })

  it('applies external presets only when preset version changes', async () => {
    const task: WorkspaceTaskDefinition = {
      id: 'rm',
      workspace: 'files-security',
      title: '删除文件',
      description: '危险动作',
      action: 'rm',
      mode: 'guarded',
      tone: 'danger',
      fields: [{ key: 'path', label: '路径', type: 'text', required: true }],
      target: () => '',
      buildPreviewArgs: () => ['rm', '--dry-run'],
      buildExecuteArgs: () => ['rm', '-y'],
    }

    const wrapper = mount(TaskToolCard, {
      props: {
        task,
        initialValues: { path: 'D:/seed/a.txt' },
        presetVersion: 1,
      },
    })

    const input = wrapper.get('input[type="text"]')
    expect((input.element as HTMLInputElement).value).toBe('D:/seed/a.txt')

    await input.setValue('D:/manual.txt')
    expect((input.element as HTMLInputElement).value).toBe('D:/manual.txt')

    await wrapper.setProps({
      initialValues: { path: 'D:/seed/b.txt' },
      presetVersion: 2,
    })

    expect((wrapper.get('input[type="text"]').element as HTMLInputElement).value).toBe('D:/seed/b.txt')
  })
  it('renders structured acl diff transition for guarded receipt', async () => {
    const task: WorkspaceTaskDefinition = {
      id: 'acl-copy',
      workspace: 'files-security',
      title: 'ACL copy',
      description: 'copy acl',
      action: 'acl:copy',
      mode: 'guarded',
      fields: [
        { key: 'path', label: 'path', type: 'text', required: true, defaultValue: 'D:/repo/a.txt' },
        { key: 'reference', label: 'reference', type: 'text', required: true, defaultValue: 'D:/repo/template.txt' },
      ],
      target: () => 'D:/repo/a.txt',
      buildPreviewArgs: () => ['acl', 'diff', '-p', 'D:/repo/a.txt', '-r', 'D:/repo/template.txt'],
      buildExecuteArgs: () => ['acl', 'copy', '-p', 'D:/repo/a.txt', '-r', 'D:/repo/template.txt', '-y'],
      previewSummary: () => 'copy acl',
    }

    apiMocks.previewGuardedTask.mockResolvedValue({
      token: 'token-acl-copy',
      workspace: 'files-security',
      action: 'acl:copy',
      target: 'D:/repo/a.txt',
      phase: 'preview',
      status: 'previewed',
      guarded: true,
      dry_run: true,
      ready_to_execute: true,
      summary: 'copy acl',
      preview_summary: 'copy acl',
      expires_in_secs: 300,
      process: {
        command_line: 'xun acl diff -p D:/repo/a.txt -r D:/repo/template.txt',
        exit_code: 0,
        success: true,
        stdout: 'preview ok',
        stderr: '',
        duration_ms: 10,
      },
      details: {
        kind: 'acl_diff',
        diff: {
          target: 'D:/repo/a.txt',
          reference: 'D:/repo/template.txt',
          common_count: 1,
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
      },
    })
    apiMocks.executeGuardedTask.mockResolvedValue({
      token: 'token-acl-copy',
      workspace: 'files-security',
      action: 'acl:copy',
      target: 'D:/repo/a.txt',
      phase: 'execute',
      status: 'succeeded',
      guarded: true,
      dry_run: false,
      summary: 'copy acl',
      audit_action: 'dashboard.task.execute.acl:copy',
      audited_at: 1700000000,
      process: {
        command_line: 'xun acl copy -p D:/repo/a.txt -r D:/repo/template.txt -y',
        exit_code: 0,
        success: true,
        stdout: 'copied',
        stderr: '',
        duration_ms: 15,
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
          common_count: 2,
          has_diff: false,
          owner_diff: null,
          inheritance_diff: null,
          only_in_target: [],
          only_in_reference: [],
        },
      },
    })

    const wrapper = mount(TaskToolCard, { props: { task }, attachTo: document.body })
    await wrapper.get('button').trigger('click')
    await flushPromises()

    const buttons = [...document.body.querySelectorAll('button')]
    const confirmButton = buttons[buttons.length - 1]
    expect(confirmButton).toBeTruthy()
    ;(confirmButton as HTMLButtonElement).click()
    await flushPromises()

    expect(wrapper.find('[data-testid="acl-diff-panel-before"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="acl-diff-panel-after"]').exists()).toBe(true)
    expect(wrapper.text()).toContain('S-1-5-32-545')
  })


  it('emits focus links for run results', async () => {
    const task: WorkspaceTaskDefinition = {
      id: 'recent-focus',
      workspace: 'paths-context',
      title: '最近访问',
      description: '查看最近访问记录。',
      action: 'recent',
      mode: 'run',
      fields: [{ key: 'limit', label: '数量', type: 'number', defaultValue: '10' }],
      buildRunArgs: () => ['recent', '-n', '10', '-f', 'json'],
    }

    apiMocks.runWorkspaceTask.mockResolvedValue({
      workspace: 'paths-context',
      action: 'recent',
      target: '',
      process: {
        command_line: 'xun recent -n 10 -f json',
        exit_code: 0,
        success: true,
        stdout: '[1,2,3]',
        stderr: '',
        duration_ms: 10,
      },
    })

    const wrapper = mount(TaskToolCard, { props: { task }, attachTo: document.body })
    await wrapper.get('button').trigger('click')
    await flushPromises()

    await wrapper.get('[data-testid="task-card-link-recent"]').trigger('click')
    await wrapper.get('[data-testid="task-card-link-audit"]').trigger('click')

    expect(wrapper.emitted('link-panel')).toHaveLength(2)
    expect(wrapper.emitted('link-panel')?.[0]?.[0]).toMatchObject({
      panel: 'recent-tasks',
      request: {
        status: 'succeeded',
        dry_run: 'executed',
        action: 'recent',
      },
    })
    expect(wrapper.emitted('link-panel')?.[1]?.[0]).toMatchObject({
      panel: 'audit',
      request: {
        result: 'success',
      },
    })
  })

})
