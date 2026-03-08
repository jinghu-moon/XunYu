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
  })

  it('runs non-guarded tasks directly and renders output', async () => {
    const task: WorkspaceTaskDefinition = {
      id: 'recent',
      workspace: 'paths-context',
      title: '最近访问',
      description: '查看最近书签',
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

    const wrapper = mount(TaskToolCard, { props: { task } })
    await wrapper.get('button').trigger('click')
    await flushPromises()

    expect(apiMocks.runWorkspaceTask).toHaveBeenCalledTimes(1)
    expect(wrapper.text()).toContain('[1,2,3]')
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

    const wrapper = mount(TaskToolCard, { props: { task } })
    await wrapper.get('button').trigger('click')
    await flushPromises()

    expect(apiMocks.previewGuardedTask).toHaveBeenCalledTimes(1)
    expect(apiMocks.executeGuardedTask).not.toHaveBeenCalled()
    expect(wrapper.text()).toContain('preview ok')

    const confirmButton = wrapper.findAll('button').find((button) => button.text().includes('确认执行'))
    expect(confirmButton).toBeTruthy()
    await confirmButton!.trigger('click')
    await flushPromises()

    expect(apiMocks.executeGuardedTask).toHaveBeenCalledWith({ token: 'token-1', confirm: true })
    expect(wrapper.text()).toContain('执行回执')
    expect(wrapper.text()).toContain('deleted')
  })
})
