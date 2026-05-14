/**
 * useOperation composable
 *
 * 封装 Operation UI 流程：preview → dialog → confirm/cancel。
 * 调用方传入 operation 名称和参数，composable 管理整个流程状态。
 */

import { ref, readonly } from 'vue'
import { previewOperation, confirmOperation, cancelOperation } from '../api/operations'
import type { WsPreview, WsOperationResult, WsRiskLevel } from '../api/ws-types'

export type OperationState = 'idle' | 'previewing' | 'confirming' | 'executing' | 'done' | 'error' | 'cancelled'

export interface UseOperationReturn {
  /** 当前状态 */
  readonly state: Readonly<ReturnType<typeof ref<OperationState>>>
  /** 预览数据 */
  readonly preview: Readonly<ReturnType<typeof ref<WsPreview | null>>>
  /** 执行结果 */
  readonly result: Readonly<ReturnType<typeof ref<WsOperationResult | null>>>
  /** 错误信息 */
  readonly error: Readonly<ReturnType<typeof ref<string | null>>>
  /** 发起预览 */
  requestPreview: (operation: string, args?: string[]) => Promise<void>
  /** 用户确认执行 */
  confirm: () => Promise<void>
  /** 用户取消 */
  cancel: () => void
  /** 重置状态 */
  reset: () => void
}

export function useOperation(): UseOperationReturn {
  const state = ref<OperationState>('idle')
  const preview = ref<WsPreview | null>(null)
  const result = ref<WsOperationResult | null>(null)
  const error = ref<string | null>(null)

  // 保存当前操作参数，confirm 时需要复用
  let currentOperation = ''
  let currentArgs: string[] = []

  async function requestPreview(operation: string, args?: string[]): Promise<void> {
    currentOperation = operation
    currentArgs = args ?? []
    state.value = 'previewing'
    error.value = null
    result.value = null

    try {
      preview.value = await previewOperation(operation, currentArgs)
      state.value = 'confirming'
    } catch (err) {
      error.value = err instanceof Error ? err.message : String(err)
      state.value = 'error'
    }
  }

  async function confirm(): Promise<void> {
    if (state.value !== 'confirming') {
      error.value = 'No preview to confirm'
      state.value = 'error'
      return
    }

    state.value = 'executing'
    error.value = null

    try {
      result.value = await confirmOperation(currentOperation, currentArgs)
      state.value = 'done'
    } catch (err) {
      error.value = err instanceof Error ? err.message : String(err)
      state.value = 'error'
    }
  }

  function cancel(): void {
    cancelOperation()
    state.value = 'cancelled'
    preview.value = null
    error.value = null
  }

  function reset(): void {
    state.value = 'idle'
    preview.value = null
    result.value = null
    error.value = null
    currentOperation = ''
    currentArgs = []
  }

  return {
    state: readonly(state),
    preview: readonly(preview) as unknown as Readonly<ReturnType<typeof ref<WsPreview | null>>>,
    result: readonly(result) as unknown as Readonly<ReturnType<typeof ref<WsOperationResult | null>>>,
    error: readonly(error),
    requestPreview,
    confirm,
    cancel,
    reset,
  }
}
