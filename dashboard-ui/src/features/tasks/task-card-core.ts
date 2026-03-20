export type TaskCardActionHintTone = 'default' | 'error'

export interface TaskCardActionHint {
  text: string
  tone: TaskCardActionHintTone
}

export function resolveTaskCardActionHint(
  isSupported: boolean,
  validationError: string,
  requestError: string,
): TaskCardActionHint | null {
  if (!isSupported) {
    return { text: '\u5f53\u524d\u6784\u5efa\u672a\u542f\u7528\u8be5 feature\u3002', tone: 'default' }
  }
  if (validationError) {
    return { text: validationError, tone: 'error' }
  }
  if (requestError) {
    return { text: requestError, tone: 'error' }
  }
  return null
}

export function resolveTaskCardBusy(
  mode: 'run' | 'guarded',
  previewBusy: boolean,
  runBusy: boolean,
): boolean {
  return mode === 'guarded' ? previewBusy : runBusy
}

export function executeTaskCardAction(
  mode: 'run' | 'guarded',
  actions: {
    previewTask: () => void | Promise<void>
    runTask: () => void | Promise<void>
  },
) {
  if (mode === 'guarded') {
    void actions.previewTask()
    return
  }
  void actions.runTask()
}
