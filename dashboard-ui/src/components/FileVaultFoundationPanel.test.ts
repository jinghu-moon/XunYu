import { flushPromises, mount } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import FileVaultFoundationPanel from './FileVaultFoundationPanel.vue'

const apiMocks = vi.hoisted(() => ({
  runWorkspaceTask: vi.fn(),
  fetchRecentWorkspaceTasks: vi.fn(),
}))

vi.mock('../api', () => ({
  runWorkspaceTask: apiMocks.runWorkspaceTask,
  fetchRecentWorkspaceTasks: apiMocks.fetchRecentWorkspaceTasks,
}))

describe('FileVaultFoundationPanel', () => {
  beforeEach(() => {
    apiMocks.runWorkspaceTask.mockReset()
    apiMocks.fetchRecentWorkspaceTasks.mockReset()
    apiMocks.fetchRecentWorkspaceTasks.mockResolvedValue({ entries: [], stats: { total: 0, succeeded: 0, failed: 0, dry_run: 0 } })
  })

  it('uses unified form to run verify and updates diagnostics', async () => {
    apiMocks.runWorkspaceTask.mockResolvedValue({
      workspace: 'files-security',
      action: 'filevault:verify',
      target: 'D:/vault/data.fv',
      process: {
        command_line: 'xun vault verify D:/vault/data.fv --password secret --json',
        exit_code: 0,
        success: true,
        stdout: '{"status":"ok","header":{"valid":true},"payload":{"valid":true},"footer":{"present":true}}',
        stderr: '',
        duration_ms: 14,
      },
    })

    const wrapper = mount(FileVaultFoundationPanel, {
      props: {
        path: 'D:/vault/data.fv',
        capabilities: { crypt: true } as any,
      },
    })
    await flushPromises()

    await wrapper.get('[data-testid="filevault-operation"]').setValue('verify')
    await wrapper.get('[data-testid="filevault-input"]').setValue('D:/vault/data.fv')
    await wrapper.find('input[type="password"]').setValue('secret')
    await wrapper.get('[data-testid="filevault-run"]').trigger('click')
    await flushPromises()

    expect(apiMocks.runWorkspaceTask).toHaveBeenCalledWith({
      workspace: 'files-security',
      action: 'filevault:verify',
      target: 'D:/vault/data.fv',
      args: ['vault', 'verify', 'D:/vault/data.fv', '--password', 'secret', '--json'],
    })
    expect(wrapper.get('[data-testid="filevault-diagnostics"]').text()).toContain('"status": "ok"')
  })

  it('requires confirmation text before cleanup', async () => {
    apiMocks.runWorkspaceTask.mockResolvedValue({
      workspace: 'files-security',
      action: 'filevault:cleanup',
      target: 'D:/vault/data.fv',
      process: {
        command_line: 'xun vault cleanup D:/vault/data.fv --json',
        exit_code: 0,
        success: true,
        stdout: '{"status":"ok","removed":["D:/vault/data.fv.fvjournal"]}',
        stderr: '',
        duration_ms: 6,
      },
    })

    const wrapper = mount(FileVaultFoundationPanel, {
      props: {
        path: 'D:/vault/data.fv',
        capabilities: { crypt: true } as any,
      },
    })
    await flushPromises()

    await wrapper.get('[data-testid="filevault-cleanup"]').trigger('click')
    await flushPromises()
    expect(apiMocks.runWorkspaceTask).not.toHaveBeenCalled()
    expect(wrapper.text()).toContain('请输入 CLEANUP')

    const inputs = wrapper.findAll('input')
    await inputs[inputs.length - 1].setValue('CLEANUP')
    await wrapper.get('[data-testid="filevault-cleanup"]').trigger('click')
    await flushPromises()

    expect(apiMocks.runWorkspaceTask).toHaveBeenCalledWith({
      workspace: 'files-security',
      action: 'filevault:cleanup',
      target: 'D:/vault/data.fv',
      args: ['vault', 'cleanup', 'D:/vault/data.fv', '--json'],
    })
  })
})
