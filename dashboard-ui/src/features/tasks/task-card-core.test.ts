import { describe, expect, it, vi } from 'vitest'

import {
  executeTaskCardAction,
  resolveTaskCardActionHint,
  resolveTaskCardBusy,
} from './task-card-core'

describe('task-card-core', () => {
  it('resolves action hint priority in supported order', () => {
    const unsupportedHint = resolveTaskCardActionHint(false, '', '')
    expect(unsupportedHint?.tone).toBe('default')
    expect(unsupportedHint?.text).toContain('feature')

    expect(resolveTaskCardActionHint(true, 'missing field', '')).toEqual({
      text: 'missing field',
      tone: 'error',
    })
    expect(resolveTaskCardActionHint(true, '', 'request failed')).toEqual({
      text: 'request failed',
      tone: 'error',
    })
    expect(resolveTaskCardActionHint(true, '', '')).toBeNull()
  })

  it('resolves busy state by task mode', () => {
    expect(resolveTaskCardBusy('guarded', true, false)).toBe(true)
    expect(resolveTaskCardBusy('guarded', false, true)).toBe(false)
    expect(resolveTaskCardBusy('run', true, false)).toBe(false)
    expect(resolveTaskCardBusy('run', false, true)).toBe(true)
  })

  it('dispatches preview or run action by mode', () => {
    const previewTask = vi.fn()
    const runTask = vi.fn()

    executeTaskCardAction('guarded', { previewTask, runTask })
    expect(previewTask).toHaveBeenCalledTimes(1)
    expect(runTask).not.toHaveBeenCalled()

    executeTaskCardAction('run', { previewTask, runTask })
    expect(runTask).toHaveBeenCalledTimes(1)
  })
})
