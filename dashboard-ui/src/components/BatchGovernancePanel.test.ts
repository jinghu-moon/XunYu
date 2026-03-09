import { flushPromises, mount } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import BatchGovernancePanel from './BatchGovernancePanel.vue'

const apiMocks = vi.hoisted(() => ({
  previewGuardedTask: vi.fn(),
  executeGuardedTask: vi.fn(),
}))

vi.mock('../api', () => ({
  previewGuardedTask: apiMocks.previewGuardedTask,
  executeGuardedTask: apiMocks.executeGuardedTask,
}))

describe('BatchGovernancePanel', () => {
  beforeEach(() => {
    apiMocks.previewGuardedTask.mockReset()
    apiMocks.executeGuardedTask.mockReset()
    document.body.innerHTML = ''
  })

  it('shows placeholder when batch queue is empty', () => {
    const wrapper = mount(BatchGovernancePanel)

    expect(wrapper.text()).toContain('先把文件加入批量队列')
    expect(wrapper.text()).toContain('0 项')
  })

  it('previews all paths and executes receipts after confirm', async () => {
    apiMocks.previewGuardedTask
      .mockResolvedValueOnce({
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
          stdout: 'ok-a',
          stderr: '',
          duration_ms: 7,
        },
      })
      .mockResolvedValueOnce({
        token: 'token-b',
        workspace: 'files-security',
        action: 'protect:set',
        target: 'D:/repo/b.txt',
        phase: 'preview',
        status: 'previewed',
        guarded: true,
        dry_run: true,
        ready_to_execute: true,
        summary: '设置保护 D:/repo/b.txt',
        preview_summary: '设置保护 D:/repo/b.txt',
        expires_in_secs: 180,
        process: {
          command_line: 'xun protect status -f json D:/repo/b.txt',
          exit_code: 0,
          success: true,
          stdout: 'ok-b',
          stderr: '',
          duration_ms: 8,
        },
      })

    apiMocks.executeGuardedTask
      .mockResolvedValueOnce({
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
          stdout: 'done-a',
          stderr: '',
          duration_ms: 11,
        },
      })
      .mockResolvedValueOnce({
        token: 'token-b',
        workspace: 'files-security',
        action: 'protect:set',
        target: 'D:/repo/b.txt',
        phase: 'execute',
        status: 'succeeded',
        guarded: true,
        dry_run: false,
        summary: '设置保护 D:/repo/b.txt',
        audit_action: 'workspace.protect.execute',
        audited_at: 1700000000,
        process: {
          command_line: 'xun protect set D:/repo/b.txt',
          exit_code: 0,
          success: true,
          stdout: 'done-b',
          stderr: '',
          duration_ms: 12,
        },
      })

    const wrapper = mount(BatchGovernancePanel, {
      attachTo: document.body,
      props: {
        paths: ['D:/repo/a.txt', 'D:/repo/b.txt'],
        capabilities: { protect: true } as any,
      },
    })

    await wrapper.get('[data-testid="batch-governance-preview"]').trigger('click')
    await flushPromises()

    expect(apiMocks.previewGuardedTask).toHaveBeenCalledTimes(2)
    expect(wrapper.find('[data-testid="batch-governance-plan"]').exists()).toBe(true)
    expect(wrapper.text()).toContain('治理计划')
    expect(wrapper.text()).toContain('已通过 2 / 2 项')
    expect(wrapper.text()).toContain('保护变更预演摘要')

    const confirmButton = [...document.body.querySelectorAll('button')].find((button) =>
      button.textContent?.includes('确认执行'),
    ) as HTMLButtonElement | undefined

    expect(confirmButton).toBeTruthy()
    expect(confirmButton?.disabled).toBe(false)
    confirmButton?.click()
    await flushPromises()

    expect(apiMocks.executeGuardedTask).toHaveBeenCalledTimes(2)
    expect(wrapper.text()).toContain('批量执行回执')
    expect(wrapper.text()).toContain('保护变更执行摘要')
    expect(wrapper.text()).toContain('workspace.protect.execute')
    expect(wrapper.text()).toContain('D:/repo/a.txt')
    expect(wrapper.text()).toContain('D:/repo/b.txt')
  })

  it('switches to acl:purge and builds preview requests from shared fields', async () => {
    apiMocks.previewGuardedTask
      .mockResolvedValueOnce({
        token: 'token-a',
        workspace: 'files-security',
        action: 'acl:purge',
        target: 'D:/repo/a.txt',
        phase: 'preview',
        status: 'previewed',
        guarded: true,
        dry_run: true,
        ready_to_execute: true,
        summary: '清理 D:/repo/a.txt 上 BUILTIN\\Users 的显式 ACL',
        preview_summary: '清理 D:/repo/a.txt 上 BUILTIN\\Users 的显式 ACL',
        expires_in_secs: 180,
        process: {
          command_line: 'xun acl view -p D:/repo/a.txt --detail',
          exit_code: 0,
          success: true,
          stdout: 'ok-a',
          stderr: '',
          duration_ms: 7,
        },
      })
      .mockResolvedValueOnce({
        token: 'token-b',
        workspace: 'files-security',
        action: 'acl:purge',
        target: 'D:/repo/b.txt',
        phase: 'preview',
        status: 'previewed',
        guarded: true,
        dry_run: true,
        ready_to_execute: true,
        summary: '清理 D:/repo/b.txt 上 BUILTIN\\Users 的显式 ACL',
        preview_summary: '清理 D:/repo/b.txt 上 BUILTIN\\Users 的显式 ACL',
        expires_in_secs: 180,
        process: {
          command_line: 'xun acl view -p D:/repo/b.txt --detail',
          exit_code: 0,
          success: true,
          stdout: 'ok-b',
          stderr: '',
          duration_ms: 8,
        },
      })

    const wrapper = mount(BatchGovernancePanel, {
      attachTo: document.body,
      props: {
        paths: ['D:/repo/a.txt', 'D:/repo/b.txt'],
      },
    })

    await wrapper.get('[data-testid="batch-governance-action"]').setValue('acl-purge')
    const principalInput = wrapper.find('input[type="text"]')
    expect(principalInput.exists()).toBe(true)
    await principalInput.setValue('BUILTIN\\Users')
    await wrapper.get('[data-testid="batch-governance-preview"]').trigger('click')
    await flushPromises()

    expect(apiMocks.previewGuardedTask).toHaveBeenNthCalledWith(1, {
      workspace: 'files-security',
      action: 'acl:purge',
      target: 'D:/repo/a.txt',
      preview_args: ['acl', 'view', '-p', 'D:/repo/a.txt', '--detail'],
      execute_args: ['acl', 'purge', '-p', 'D:/repo/a.txt', '--principal', 'BUILTIN\\Users', '-y'],
      preview_summary: '清理 D:/repo/a.txt 上 BUILTIN\\Users 的显式 ACL',
    })
    expect(wrapper.find('[data-testid="batch-governance-plan"]').exists()).toBe(true)
    expect(wrapper.text()).toContain('批量清理 ACL 主体')
    expect(wrapper.text()).toContain('主体')
    expect(wrapper.text()).toContain('BUILTIN\\Users')
  })



  it('switches to encrypt and builds preview requests from shared fields', async () => {
    apiMocks.previewGuardedTask.mockResolvedValueOnce({
      token: 'token-encrypt',
      workspace: 'files-security',
      action: 'encrypt',
      target: 'D:/repo/a.txt',
      phase: 'preview',
      status: 'previewed',
      guarded: true,
      dry_run: true,
      ready_to_execute: true,
      summary: '加密 D:/repo/a.txt',
      preview_summary: '加密 D:/repo/a.txt',
      expires_in_secs: 180,
      process: {
        command_line: 'xun find --dry-run -f json --test-path D:/repo/a.txt',
        exit_code: 0,
        success: true,
        stdout: 'path: "D:/repo/a.txt"  (is_dir=false)\n  -> Decision: INCLUDE (source: inherited)',
        stderr: '',
        duration_ms: 9,
      },
    })

    const wrapper = mount(BatchGovernancePanel, {
      attachTo: document.body,
      props: {
        paths: ['D:/repo/a.txt'],
        capabilities: { crypt: true } as any,
      },
    })

    await wrapper.get('[data-testid="batch-governance-action"]').setValue('encrypt')
    const textarea = wrapper.find('textarea')
    expect(textarea.exists()).toBe(true)
    await textarea.setValue('age1abc\nage1def')
    const textInputs = wrapper.findAll('input[type="text"]')
    expect(textInputs).toHaveLength(1)
    await textInputs[0]!.setValue('D:/repo/a.txt.age')
    await wrapper.get('[data-testid="batch-governance-preview"]').trigger('click')
    await flushPromises()

    expect(apiMocks.previewGuardedTask).toHaveBeenNthCalledWith(1, {
      workspace: 'files-security',
      action: 'encrypt',
      target: 'D:/repo/a.txt',
      preview_args: ['find', '--dry-run', '-f', 'json', '--test-path', 'D:/repo/a.txt'],
      execute_args: ['encrypt', '--to', 'age1abc', '--to', 'age1def', '-o', 'D:/repo/a.txt.age', 'D:/repo/a.txt'],
      preview_summary: '加密 D:/repo/a.txt',
    })
    expect(wrapper.find('[data-testid="batch-governance-plan"]').exists()).toBe(true)
    expect(wrapper.text()).toContain('批量加密文件')
    expect(wrapper.text()).toContain('age1abc / age1def')
    expect(wrapper.text()).toContain('加密预演摘要')
  })

  it('emits recent task focus from preview items', async () => {
    apiMocks.previewGuardedTask.mockResolvedValueOnce({
      token: 'token-preview',
      workspace: 'files-security',
      action: 'protect:set',
      target: 'D:/repo/a.txt',
      phase: 'preview',
      status: 'previewed',
      guarded: true,
      dry_run: true,
      ready_to_execute: true,
      summary: 'Protect D:/repo/a.txt',
      preview_summary: 'Protect D:/repo/a.txt',
      expires_in_secs: 180,
      process: {
        command_line: 'xun protect status -f json D:/repo/a.txt',
        exit_code: 0,
        success: true,
        stdout: 'ok-a',
        stderr: '',
        duration_ms: 7,
      },
    })

    const wrapper = mount(BatchGovernancePanel, {
      attachTo: document.body,
      props: {
        paths: ['D:/repo/a.txt'],
        capabilities: { protect: true } as any,
      },
    })

    await wrapper.get('[data-testid="batch-governance-preview"]').trigger('click')
    await flushPromises()
    await wrapper.get('[data-testid="batch-preview-link-recent"]').trigger('click')

    expect(wrapper.emitted('focus-recent-tasks')).toEqual([[{
      status: 'previewed',
      dry_run: 'dry-run',
      action: 'protect:set',
      search: 'D:/repo/a.txt',
    }]])
  })

  it('emits recent task focus from execute receipts', async () => {
    apiMocks.previewGuardedTask.mockResolvedValueOnce({
      token: 'token-receipt',
      workspace: 'files-security',
      action: 'protect:set',
      target: 'D:/repo/a.txt',
      phase: 'preview',
      status: 'previewed',
      guarded: true,
      dry_run: true,
      ready_to_execute: true,
      summary: 'Protect D:/repo/a.txt',
      preview_summary: 'Protect D:/repo/a.txt',
      expires_in_secs: 180,
      process: {
        command_line: 'xun protect status -f json D:/repo/a.txt',
        exit_code: 0,
        success: true,
        stdout: 'ok-a',
        stderr: '',
        duration_ms: 7,
      },
    })
    apiMocks.executeGuardedTask.mockResolvedValueOnce({
      token: 'token-receipt',
      workspace: 'files-security',
      action: 'protect:set',
      target: 'D:/repo/a.txt',
      phase: 'execute',
      status: 'succeeded',
      guarded: true,
      dry_run: false,
      summary: 'Protect D:/repo/a.txt',
      audit_action: 'workspace.protect.execute',
      audited_at: 1700000000,
      process: {
        command_line: 'xun protect set D:/repo/a.txt',
        exit_code: 0,
        success: true,
        stdout: 'done-a',
        stderr: '',
        duration_ms: 11,
      },
    })

    const wrapper = mount(BatchGovernancePanel, {
      attachTo: document.body,
      props: {
        paths: ['D:/repo/a.txt'],
        capabilities: { protect: true } as any,
      },
    })

    await wrapper.get('[data-testid="batch-governance-preview"]').trigger('click')
    await flushPromises()

    const confirmButton = [...document.body.querySelectorAll('button')].find((button) =>
      button.textContent?.includes('确认执行'),
    ) as HTMLButtonElement | undefined

    expect(confirmButton).toBeTruthy()
    confirmButton?.click()
    await flushPromises()

    await wrapper.get('[data-testid="batch-receipt-link-recent"]').trigger('click')

    expect(wrapper.emitted('focus-recent-tasks')).toEqual([[{
      status: 'succeeded',
      dry_run: 'executed',
      action: 'protect:set',
      search: 'D:/repo/a.txt',
    }]])
  })

  it('keeps confirm disabled when any preview is blocked', async () => {
    apiMocks.previewGuardedTask
      .mockResolvedValueOnce({
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
          stdout: 'ok-a',
          stderr: '',
          duration_ms: 7,
        },
      })
      .mockRejectedValueOnce(new Error('400 Bad Request: preview failed'))

    const wrapper = mount(BatchGovernancePanel, {
      attachTo: document.body,
      props: {
        paths: ['D:/repo/a.txt', 'D:/repo/b.txt'],
        capabilities: { protect: true } as any,
      },
    })

    await wrapper.get('[data-testid="batch-governance-preview"]').trigger('click')
    await flushPromises()

    expect(apiMocks.previewGuardedTask).toHaveBeenCalledTimes(2)
    expect(wrapper.text()).toContain('未通过 1 项')
    expect(wrapper.text()).toContain('400 Bad Request: preview failed')

    const confirmButton = [...document.body.querySelectorAll('button')].find((button) =>
      button.textContent?.includes('确认执行'),
    ) as HTMLButtonElement | undefined

    expect(confirmButton).toBeTruthy()
    expect(confirmButton?.disabled).toBe(true)
    confirmButton?.click()
    await flushPromises()

    expect(apiMocks.executeGuardedTask).not.toHaveBeenCalled()
  })
})
