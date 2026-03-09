import { flushPromises, mount } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { defineComponent } from 'vue'
import TaskToolCard from '../components/TaskToolCard.vue'
import IntegrationAutomationWorkspace from '../components/workspaces/IntegrationAutomationWorkspace.vue'
import { findWorkspaceTaskDefinition } from '../workspace-tools'

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

const TaskToolboxProbe = defineComponent({
  props: {
    taskPresets: { type: Object, default: null },
    presetVersion: { type: Number, default: 0 },
  },
  template: `
    <div>
      <div data-testid="smoke-preset-version">{{ presetVersion }}</div>
      <div data-testid="smoke-preset-payload">{{ JSON.stringify(taskPresets ?? null) }}</div>
    </div>
  `,
})

const RecentTasksPanelStub = defineComponent({
  template: '<div data-testid="smoke-recent-panel"></div>',
})

const RecipePanelStub = defineComponent({
  template: '<div data-testid="smoke-recipe-panel"></div>',
})

const WorkspaceFrameStub = defineComponent({
  template: '<section><slot /></section>',
})

describe('dashboard smoke', () => {
  beforeEach(() => {
    apiMocks.runWorkspaceTask.mockReset()
    apiMocks.previewGuardedTask.mockReset()
    apiMocks.executeGuardedTask.mockReset()
    document.body.innerHTML = ''
  })

  it('wires shell guide presets into the integration toolbox', async () => {
    const wrapper = mount(IntegrationAutomationWorkspace, {
      global: {
        stubs: {
          TaskToolbox: TaskToolboxProbe,
          RecentTasksPanel: RecentTasksPanelStub,
          RecipePanel: RecipePanelStub,
          WorkspaceFrame: WorkspaceFrameStub,
        },
      },
    })

    await wrapper.get('[data-testid="shell-guide-shell-zsh"]').trigger('click')
    await wrapper.get('[data-testid="shell-guide-apply-presets"]').trigger('click')
    await flushPromises()

    expect(wrapper.get('[data-testid="smoke-preset-version"]').text()).toBe('1')
    expect(wrapper.get('[data-testid="smoke-preset-payload"]').text()).toContain('"shell":"zsh"')
    expect(wrapper.get('[data-testid="smoke-preset-payload"]').text()).toContain('"args":"alias ls --j"')
  })

  it('keeps alias rm behind preview-confirm-execute triple guard', async () => {
    const task = findWorkspaceTaskDefinition('integration-automation', 'alias:rm')
    expect(task).not.toBeNull()

    apiMocks.previewGuardedTask.mockResolvedValue({
      token: 'alias-rm-token',
      workspace: 'integration-automation',
      action: 'alias:rm',
      target: 'gst',
      summary: 'preview ok',
      ready_to_execute: true,
      process: {
        command_line: 'xun alias which gst',
        exit_code: 0,
        success: true,
        stdout: 'gst -> git status -sb',
        stderr: '',
        duration_ms: 8,
      },
      details: null,
    })
    apiMocks.executeGuardedTask.mockResolvedValue({
      token: 'alias-rm-token',
      workspace: 'integration-automation',
      action: 'alias:rm',
      target: 'gst',
      status: 'succeeded',
      audit_action: 'workspace.alias.rm.execute',
      summary: 'removed',
      process: {
        command_line: 'xun alias rm gst',
        exit_code: 0,
        success: true,
        stdout: 'removed gst',
        stderr: '',
        duration_ms: 12,
      },
      details: null,
    })

    const wrapper = mount(TaskToolCard, {
      props: { task: task! },
      attachTo: document.body,
    })

    await wrapper.get('input').setValue('gst')
    await wrapper.get('button').trigger('click')
    await flushPromises()

    expect(apiMocks.previewGuardedTask).toHaveBeenCalledWith({
      workspace: 'integration-automation',
      action: 'alias:rm',
      target: 'gst',
      preview_args: ['alias', 'which', 'gst'],
      execute_args: ['alias', 'rm', 'gst'],
      preview_summary: '删除 alias gst 前先查看解析结果',
    })
    expect(apiMocks.executeGuardedTask).not.toHaveBeenCalled()
    expect(document.body.textContent || '').toContain('确认执行')

    const confirmButton = Array.from(document.body.querySelectorAll('button')).find((button) => button.textContent?.includes('确认执行'))
    expect(confirmButton).toBeTruthy()
    ;(confirmButton as HTMLButtonElement).click()
    await flushPromises()

    expect(apiMocks.executeGuardedTask).toHaveBeenCalledWith({ token: 'alias-rm-token', confirm: true })
    expect(wrapper.text()).toContain('removed gst')
  })

  it('keeps alias app rm behind preview-confirm-execute triple guard', async () => {
    const task = findWorkspaceTaskDefinition('integration-automation', 'alias:app-rm')
    expect(task).not.toBeNull()

    apiMocks.previewGuardedTask.mockResolvedValue({
      token: 'alias-app-rm-token',
      workspace: 'integration-automation',
      action: 'alias:app-rm',
      target: 'code',
      summary: 'preview ok',
      ready_to_execute: true,
      process: {
        command_line: 'xun alias app which code',
        exit_code: 0,
        success: true,
        stdout: 'code -> Code.exe',
        stderr: '',
        duration_ms: 8,
      },
      details: null,
    })
    apiMocks.executeGuardedTask.mockResolvedValue({
      token: 'alias-app-rm-token',
      workspace: 'integration-automation',
      action: 'alias:app-rm',
      target: 'code',
      status: 'succeeded',
      audit_action: 'workspace.alias.app-rm.execute',
      summary: 'removed',
      process: {
        command_line: 'xun alias app rm code',
        exit_code: 0,
        success: true,
        stdout: 'removed code',
        stderr: '',
        duration_ms: 12,
      },
      details: null,
    })

    const wrapper = mount(TaskToolCard, {
      props: { task: task! },
      attachTo: document.body,
    })

    await wrapper.get('input').setValue('code')
    await wrapper.get('button').trigger('click')
    await flushPromises()

    expect(apiMocks.previewGuardedTask).toHaveBeenCalledWith({
      workspace: 'integration-automation',
      action: 'alias:app-rm',
      target: 'code',
      preview_args: ['alias', 'app', 'which', 'code'],
      execute_args: ['alias', 'app', 'rm', 'code'],
      preview_summary: '删除 app alias code 前先查看解析结果',
    })
    expect(apiMocks.executeGuardedTask).not.toHaveBeenCalled()

    const confirmButton = Array.from(document.body.querySelectorAll('button')).find((button) => button.textContent?.includes('确认执行'))
    expect(confirmButton).toBeTruthy()
    ;(confirmButton as HTMLButtonElement).click()
    await flushPromises()

    expect(apiMocks.executeGuardedTask).toHaveBeenCalledWith({ token: 'alias-app-rm-token', confirm: true })
    expect(wrapper.text()).toContain('removed code')
  })
})
