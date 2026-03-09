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
    expect(wrapper.get('[data-testid="probe-acl-diff"]').text()).toContain('先在工作台里设置 ACL 参考路径')
  })

  it('refreshes structured acl diff when acl reference path is provided', async () => {
    apiMocks.runWorkspaceTask
      .mockResolvedValueOnce({
        workspace: 'files-security',
        action: 'lock:who',
        target: 'D:/repo/src/b.rs',
        process: {
          command_line: 'xun lock who -f json D:/repo/src/b.rs',
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
        target: 'D:/repo/src/b.rs',
        process: {
          command_line: 'xun protect status -f json D:/repo/src/b.rs',
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
        target: 'D:/repo/src/b.rs',
        process: {
          command_line: 'xun acl view -p D:/repo/src/b.rs',
          exit_code: 0,
          success: true,
          stdout: 'OWNER: BUILTIN\\Administrators',
          stderr: '',
          duration_ms: 11,
        },
      })
      .mockResolvedValueOnce({
        workspace: 'files-security',
        action: 'acl:diff',
        target: 'D:/repo/src/b.rs',
        process: {
          command_line: 'xun acl diff -p D:/repo/src/b.rs -r D:/repo/src/a.rs',
          exit_code: 0,
          success: true,
          stdout: 'Only in A: 1\nOnly in B: 1\nCommon: 2',
          stderr: '',
          duration_ms: 15,
        },
        details: {
          kind: 'acl_diff',
          diff: {
            target: 'D:/repo/src/b.rs',
            reference: 'D:/repo/src/a.rs',
            common_count: 2,
            has_diff: true,
            owner_diff: {
              target: 'BUILTIN\\Administrators',
              reference: 'NT AUTHORITY\\SYSTEM',
            },
            inheritance_diff: {
              target_protected: false,
              reference_protected: true,
            },
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

    const wrapper = mount(FileGovernancePanel, {
      props: {
        path: 'D:/repo/src/b.rs',
        aclReferencePath: 'D:/repo/src/a.rs',
        capabilities: { lock: true, protect: true } as any,
      },
    })

    await wrapper.get('[data-testid="refresh-governance"]').trigger('click')
    await flushPromises()

    expect(apiMocks.runWorkspaceTask).toHaveBeenCalledTimes(4)
    expect(apiMocks.runWorkspaceTask).toHaveBeenNthCalledWith(4, {
      workspace: 'files-security',
      action: 'acl:diff',
      target: 'D:/repo/src/b.rs',
      args: ['acl', 'diff', '-p', 'D:/repo/src/b.rs', '-r', 'D:/repo/src/a.rs'],
    })
    expect(wrapper.get('[data-testid="acl-reference-path"]').text()).toContain('D:/repo/src/a.rs')
    expect(wrapper.get('[data-testid="probe-acl-diff"]').text()).toContain('ACL 差异明细')
    expect(wrapper.get('[data-testid="probe-acl-diff"]').text()).toContain('S-1-5-32-545')
    expect(wrapper.get('[data-testid="probe-acl-diff"]').text()).toContain('仍有差异')
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
