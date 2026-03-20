import { describe, expect, it } from 'vitest'

import type { TaskProcessOutput, WorkspaceCapabilities } from '../../types'
import type { TaskFieldDefinition, TaskFormState } from './catalog'
import {
  applyTaskInitialValues,
  classifyFailure,
  createInitialTaskForm,
  errorMessage,
  isTaskSupported,
  resolveTaskExecutionActionLabel,
  resolveTaskExecutionFailureHint,
  resolveTaskExecutionStateLabel,
  resolveTaskExecutionStateTone,
  validateTaskForm,
} from './task-execution-core'

const fields: TaskFieldDefinition[] = [
  { key: 'path', label: '\u8def\u5f84', type: 'text', required: true },
  { key: 'confirm', label: '\u786e\u8ba4', type: 'checkbox', required: true },
  { key: 'name', label: '\u540d\u79f0', type: 'text', defaultValue: 'demo' },
]

const capabilities: WorkspaceCapabilities = {
  alias: true,
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
}

const failedProcess: TaskProcessOutput = {
  command_line: 'xun demo',
  exit_code: 1,
  success: false,
  stdout: '',
  stderr: 'Access denied',
  duration_ms: 10,
}

describe('task-execution-core', () => {
  it('builds initial form with defaults and checkbox fallback', () => {
    expect(createInitialTaskForm(fields)).toEqual({
      path: '',
      confirm: false,
      name: 'demo',
    })
  })

  it('applies preset values only to known fields with defined values', () => {
    const form = createInitialTaskForm(fields)

    applyTaskInitialValues(fields, form, {
      path: 'D:/repo/demo.txt',
      name: undefined,
      extra: 'ignored',
    } as Partial<TaskFormState>)

    expect(form).toEqual({
      path: 'D:/repo/demo.txt',
      confirm: false,
      name: 'demo',
    })
  })

  it('resolves feature support from capabilities', () => {
    expect(isTaskSupported(undefined, null)).toBe(true)
    expect(isTaskSupported('alias', null)).toBe(true)
    expect(isTaskSupported('alias', capabilities)).toBe(true)
    expect(isTaskSupported('alias', { ...capabilities, alias: false })).toBe(false)
  })

  it('returns readable validation error for missing required fields', () => {
    const form: TaskFormState = {
      path: '   ',
      confirm: false,
      name: 'demo',
    }

    expect(validateTaskForm(fields, form)).toBe('\u7f3a\u5c11\u5fc5\u586b\u9879\uff1a\u8def\u5f84\u3001\u786e\u8ba4')
  })

  it('accepts filled string fields and checked checkbox', () => {
    const form: TaskFormState = {
      path: 'D:/repo/demo.txt',
      confirm: true,
      name: 'demo',
    }

    expect(validateTaskForm(fields, form)).toBe('')
  })

  it('resolves action label and state presentation', () => {
    expect(resolveTaskExecutionActionLabel('guarded')).toBe('\u9884\u6f14\u5e76\u786e\u8ba4')
    expect(resolveTaskExecutionActionLabel('run')).toBe('\u8fd0\u884c')
    expect(resolveTaskExecutionStateLabel('idle')).toBe('\u5f85\u6267\u884c')
    expect(resolveTaskExecutionStateLabel('previewing')).toBe('\u9884\u6f14\u4e2d')
    expect(resolveTaskExecutionStateLabel('awaiting_confirm')).toBe('\u5f85\u786e\u8ba4')
    expect(resolveTaskExecutionStateLabel('running')).toBe('\u6267\u884c\u4e2d')
    expect(resolveTaskExecutionStateLabel('succeeded')).toBe('\u6210\u529f')
    expect(resolveTaskExecutionStateLabel('failed')).toBe('\u5931\u8d25')
    expect(resolveTaskExecutionStateTone('idle')).toBe('')
    expect(resolveTaskExecutionStateTone('succeeded')).toBe('is-ok')
    expect(resolveTaskExecutionStateTone('failed')).toBe('is-error')
  })

  it('normalizes common failure messages to operator hints', () => {
    expect(classifyFailure('Access denied while updating hosts file')).toBe(
      '\u53ef\u80fd\u7f3a\u5c11\u7ba1\u7406\u5458\u6743\u9650\uff0c\u8bf7\u4ee5\u7ba1\u7406\u5458\u65b9\u5f0f\u8fd0\u884c\u6216\u4f7f\u7528\u63d0\u6743\u53c2\u6570\u3002',
    )
    expect(classifyFailure('no matching window found')).toBe(
      '\u672a\u627e\u5230\u5339\u914d\u7a97\u53e3\uff0c\u8bf7\u786e\u8ba4\u76ee\u6807\u7a97\u53e3\u5df2\u6253\u5f00\u4e14\u6807\u9898\u5339\u914d\u3002',
    )
    expect(classifyFailure('invalid hotkey ctrl+alt+space')).toBe(
      '\u5feb\u6377\u952e\u683c\u5f0f\u9519\u8bef\u6216\u88ab\u7cfb\u7edf\u4fdd\u7559\uff0c\u68c0\u67e5\u8f93\u5165\u683c\u5f0f\u3002',
    )
  })

  it('derives failure hint only for failed state and honors request error priority', () => {
    expect(resolveTaskExecutionFailureHint('running', 'Access denied', failedProcess)).toBe('')
    expect(resolveTaskExecutionFailureHint('failed', 'permission denied', failedProcess)).toBe(
      '\u53ef\u80fd\u7f3a\u5c11\u7ba1\u7406\u5458\u6743\u9650\uff0c\u8bf7\u4ee5\u7ba1\u7406\u5458\u65b9\u5f0f\u8fd0\u884c\u6216\u4f7f\u7528\u63d0\u6743\u53c2\u6570\u3002',
    )
    expect(resolveTaskExecutionFailureHint('failed', '', failedProcess)).toBe(
      '\u53ef\u80fd\u7f3a\u5c11\u7ba1\u7406\u5458\u6743\u9650\uff0c\u8bf7\u4ee5\u7ba1\u7406\u5458\u65b9\u5f0f\u8fd0\u884c\u6216\u4f7f\u7528\u63d0\u6743\u53c2\u6570\u3002',
    )
  })

  it('falls back to raw error or default error text', () => {
    expect(errorMessage(new Error('boom'))).toBe('boom')
    expect(errorMessage(new Error('   '))).toBe('\u8bf7\u6c42\u5931\u8d25\uff0c\u8bf7\u68c0\u67e5\u5168\u5c40\u9519\u8bef\u63d0\u793a\u3002')
    expect(errorMessage('plain text')).toBe('\u8bf7\u6c42\u5931\u8d25\uff0c\u8bf7\u68c0\u67e5\u5168\u5c40\u9519\u8bef\u63d0\u793a\u3002')
  })
})
