import { effectScope, nextTick, reactive } from 'vue'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import type { WorkspaceTaskDefinition } from './catalog'
import { useTaskExecution } from './use-task-execution'

const apiMocks = vi.hoisted(() => ({
  runWorkspaceTask: vi.fn(),
  previewGuardedTask: vi.fn(),
  executeGuardedTask: vi.fn(),
}))

vi.mock('../../api', () => ({
  runWorkspaceTask: apiMocks.runWorkspaceTask,
  previewGuardedTask: apiMocks.previewGuardedTask,
  executeGuardedTask: apiMocks.executeGuardedTask,
}))

type UseTaskExecutionProps = Parameters<typeof useTaskExecution>[0]

function createRunTask(): WorkspaceTaskDefinition {
  return {
    id: 'alias-query',
    workspace: 'paths-context',
    title: 'Alias Query',
    description: 'query alias',
    action: 'alias:query',
    mode: 'run',
    feature: 'alias',
    fields: [{ key: 'name', label: 'Name', type: 'text', required: true, defaultValue: 'demo' }],
    target: (values) => String(values.name ?? ''),
    buildRunArgs: (values) => ['alias', 'query', String(values.name ?? '')],
  }
}

function createGuardedTask(): WorkspaceTaskDefinition {
  return {
    id: 'alias-remove',
    workspace: 'paths-context',
    title: 'Alias Remove',
    description: 'remove alias',
    action: 'alias:remove',
    mode: 'guarded',
    feature: 'alias',
    fields: [{ key: 'name', label: 'Name', type: 'text', required: true, defaultValue: 'demo' }],
    target: (values) => String(values.name ?? ''),
    buildPreviewArgs: (values) => ['alias', 'remove', '--dry-run', String(values.name ?? '')],
    buildExecuteArgs: (values) => ['alias', 'remove', String(values.name ?? '')],
    previewSummary: (values) => `remove ${String(values.name ?? '')}`,
  }
}

function setupUseTaskExecution(propsOverride: Partial<UseTaskExecutionProps> = {}) {
  const props = reactive<UseTaskExecutionProps>({
    task: createRunTask(),
    capabilities: null,
    initialValues: null,
    presetVersion: 0,
    ...propsOverride,
  })
  const scope = effectScope()
  const execution = scope.run(() => useTaskExecution(props))

  if (!execution) {
    throw new Error('useTaskExecution setup failed')
  }

  return {
    props,
    execution,
    stop: () => scope.stop(),
  }
}

describe('useTaskExecution', () => {
  const stops: Array<() => void> = []

  beforeEach(() => {
    apiMocks.runWorkspaceTask.mockReset()
    apiMocks.previewGuardedTask.mockReset()
    apiMocks.executeGuardedTask.mockReset()
  })

  afterEach(() => {
    while (stops.length) {
      stops.pop()?.()
    }
  })

  it('applies initial values on mount and when preset version changes', async () => {
    const setup = setupUseTaskExecution({
      initialValues: { name: 'preset-a' },
    })
    stops.push(setup.stop)

    expect(setup.execution.form.name).toBe('preset-a')

    setup.props.initialValues = { name: 'preset-b' }
    setup.props.presetVersion = 1
    await nextTick()

    expect(setup.execution.form.name).toBe('preset-b')
  })

  it('runs non-guarded tasks and updates execution state', async () => {
    apiMocks.runWorkspaceTask.mockResolvedValue({
      workspace: 'paths-context',
      action: 'alias:query',
      target: 'demo',
      process: {
        command_line: 'xun alias query demo',
        exit_code: 0,
        success: true,
        stdout: 'demo => D:/repo/demo',
        stderr: '',
        duration_ms: 8,
      },
      details: null,
    })

    const setup = setupUseTaskExecution()
    stops.push(setup.stop)

    await setup.execution.runTask()

    expect(apiMocks.runWorkspaceTask).toHaveBeenCalledWith({
      workspace: 'paths-context',
      action: 'alias:query',
      target: 'demo',
      args: ['alias', 'query', 'demo'],
    })
    expect(setup.execution.actionLabel.value).toBe('\u8fd0\u884c')
    expect(setup.execution.state.value).toBe('succeeded')
    expect(setup.execution.stateLabel.value).toBe('\u6210\u529f')
    expect(setup.execution.processOutput.value?.stdout).toContain('demo =>')
  })

  it('previews guarded tasks and confirms execution', async () => {
    apiMocks.previewGuardedTask.mockResolvedValue({
      token: 'token-1',
      workspace: 'paths-context',
      action: 'alias:remove',
      target: 'demo',
      phase: 'preview',
      status: 'previewed',
      guarded: true,
      dry_run: true,
      ready_to_execute: true,
      summary: 'remove demo',
      preview_summary: 'remove demo',
      expires_in_secs: 300,
      process: {
        command_line: 'xun alias remove --dry-run demo',
        exit_code: 0,
        success: true,
        stdout: 'preview ok',
        stderr: '',
        duration_ms: 9,
      },
      details: null,
    })
    apiMocks.executeGuardedTask.mockResolvedValue({
      token: 'token-1',
      workspace: 'paths-context',
      action: 'alias:remove',
      target: 'demo',
      phase: 'execute',
      status: 'succeeded',
      guarded: true,
      dry_run: false,
      summary: 'remove demo',
      audit_action: 'dashboard.task.execute.alias:remove',
      audited_at: 1700000000,
      process: {
        command_line: 'xun alias remove demo',
        exit_code: 0,
        success: true,
        stdout: 'removed',
        stderr: '',
        duration_ms: 7,
      },
      details: null,
    })

    const setup = setupUseTaskExecution({ task: createGuardedTask() })
    stops.push(setup.stop)

    await setup.execution.previewTask()

    expect(setup.execution.actionLabel.value).toBe('\u9884\u6f14\u5e76\u786e\u8ba4')
    expect(setup.execution.state.value).toBe('awaiting_confirm')
    expect(setup.execution.dialogOpen.value).toBe(true)
    expect(setup.execution.preview.value?.token).toBe('token-1')

    await setup.execution.confirmTask()

    expect(apiMocks.executeGuardedTask).toHaveBeenCalledWith({ token: 'token-1', confirm: true })
    expect(setup.execution.dialogOpen.value).toBe(false)
    expect(setup.execution.preview.value).toBeNull()
    expect(setup.execution.receipt.value?.process.stdout).toBe('removed')
    expect(setup.execution.state.value).toBe('succeeded')
  })

  it('exposes feature gating and request failure hint', async () => {
    apiMocks.runWorkspaceTask.mockRejectedValue(new Error('permission denied'))

    const setup = setupUseTaskExecution({
      capabilities: {
        alias: false,
        batch_rename: true,
        crypt: true,
        cstat: true,
        diff: true,
        fs: true,
        img: true,
        lock: true,
        protect: true,
        redirect: true,
        desktop: true,
        tui: true,
      },
    })
    stops.push(setup.stop)

    expect(setup.execution.isSupported.value).toBe(false)

    setup.props.capabilities = null
    await nextTick()
    expect(setup.execution.isSupported.value).toBe(true)

    await setup.execution.runTask()

    expect(setup.execution.state.value).toBe('failed')
    expect(setup.execution.requestError.value).toBe('permission denied')
    expect(setup.execution.failureHint.value).toBe(
      '\u53ef\u80fd\u7f3a\u5c11\u7ba1\u7406\u5458\u6743\u9650\uff0c\u8bf7\u4ee5\u7ba1\u7406\u5458\u65b9\u5f0f\u8fd0\u884c\u6216\u4f7f\u7528\u63d0\u6743\u53c2\u6570\u3002',
    )
  })
})
