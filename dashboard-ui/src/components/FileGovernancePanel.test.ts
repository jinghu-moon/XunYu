import { flushPromises, mount } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import FileGovernancePanel from './FileGovernancePanel.vue'

const apiMocks = vi.hoisted(() => ({
  runWorkspaceTask: vi.fn(),
}))

vi.mock('../api', () => ({
  runWorkspaceTask: apiMocks.runWorkspaceTask,
}))

describe('FileGovernancePanel', () => {
  beforeEach(() => {
    apiMocks.runWorkspaceTask.mockReset()
  })

  it('shows placeholder when no file is selected', () => {
    const wrapper = mount(FileGovernancePanel)

    expect(wrapper.text()).toContain('先在上方 File Manager 选中文件')
    expect(wrapper.get('[data-testid="refresh-governance"]').attributes('disabled')).toBeDefined()
  })

  it('refreshes lock / protect / acl snapshots for the selected file', async () => {
    apiMocks.runWorkspaceTask
      .mockResolvedValueOnce({
        workspace: 'files-security',
        action: 'lock:who',
        target: 'D:/repo/src/a.rs',
        process: {
          command_line: 'xun lock who -f json D:/repo/src/a.rs',
          exit_code: 0,
          success: true,
          stdout: '{"holders":[]}',
          stderr: '',
          duration_ms: 8,
        },
      })
      .mockResolvedValueOnce({
        workspace: 'files-security',
        action: 'protect:status',
        target: 'D:/repo/src/a.rs',
        process: {
          command_line: 'xun protect status -f json D:/repo/src/a.rs',
          exit_code: 0,
          success: true,
          stdout: '{"rules":[]}',
          stderr: '',
          duration_ms: 7,
        },
      })
      .mockResolvedValueOnce({
        workspace: 'files-security',
        action: 'acl:view',
        target: 'D:/repo/src/a.rs',
        process: {
          command_line: 'xun acl view -p D:/repo/src/a.rs',
          exit_code: 0,
          success: true,
          stdout: 'OWNER: BUILTIN\\Administrators',
          stderr: '',
          duration_ms: 11,
        },
      })

    const wrapper = mount(FileGovernancePanel, {
      props: {
        path: 'D:/repo/src/a.rs',
        capabilities: { lock: true, protect: true } as any,
      },
    })

    await wrapper.get('[data-testid="refresh-governance"]').trigger('click')
    await flushPromises()

    expect(apiMocks.runWorkspaceTask).toHaveBeenCalledTimes(3)
    expect(apiMocks.runWorkspaceTask).toHaveBeenNthCalledWith(1, {
      workspace: 'files-security',
      action: 'lock:who',
      target: 'D:/repo/src/a.rs',
      args: ['lock', 'who', '-f', 'json', 'D:/repo/src/a.rs'],
    })
    expect(wrapper.get('[data-testid="probe-lock"]').text()).toContain('holders')
    expect(wrapper.get('[data-testid="probe-protect"]').text()).toContain('rules')
    expect(wrapper.get('[data-testid="probe-acl"]').text()).toContain('OWNER: BUILTIN\\Administrators')
  })

  it('skips probes for unavailable capabilities', async () => {
    apiMocks.runWorkspaceTask.mockResolvedValue({
      workspace: 'files-security',
      action: 'acl:view',
      target: 'D:/repo/src/a.rs',
      process: {
        command_line: 'xun acl view -p D:/repo/src/a.rs',
        exit_code: 0,
        success: true,
        stdout: 'ok',
        stderr: '',
        duration_ms: 4,
      },
    })

    const wrapper = mount(FileGovernancePanel, {
      props: {
        path: 'D:/repo/src/a.rs',
        capabilities: { lock: false, protect: false } as any,
      },
    })

    await wrapper.get('[data-testid="refresh-governance"]').trigger('click')
    await flushPromises()

    expect(apiMocks.runWorkspaceTask).toHaveBeenCalledTimes(1)
    expect(wrapper.get('[data-testid="probe-lock"]').text()).toContain('当前构建未启用该能力')
    expect(wrapper.get('[data-testid="probe-protect"]').text()).toContain('当前构建未启用该能力')
    expect(wrapper.get('[data-testid="probe-acl"]').text()).toContain('ok')
  })
})
