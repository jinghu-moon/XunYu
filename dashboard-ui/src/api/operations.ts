/**
 * Operation 协议封装
 *
 * 封装 Preview → Confirm 的操作流程。
 * 调用方只需调用 previewOperation / confirmOperation / cancelOperation。
 */

import { useWsStore } from '../stores/ws'
import type { WsPreview, WsOperationResult } from './ws-types'

/**
 * 发送 PreviewOp 请求，返回 Preview 描述。
 */
export async function previewOperation(
  operation: string,
  args?: string[],
): Promise<WsPreview> {
  const ws = useWsStore()
  const resp = await ws.sendPreview(operation, args)
  if (resp.type !== 'PreviewResult') {
    throw new Error(`Unexpected response type: ${resp.type}`)
  }
  return resp.payload.preview
}

/**
 * 发送 ConfirmOp 请求，返回 OperationResult。
 * 必须在 previewOperation 之后调用。
 */
export async function confirmOperation(
  operation: string,
  args?: string[],
): Promise<WsOperationResult> {
  const ws = useWsStore()
  const resp = await ws.sendConfirm(operation, args)
  if (resp.type !== 'OpResult') {
    throw new Error(`Unexpected response type: ${resp.type}`)
  }
  return resp.payload.result
}

/**
 * 取消当前操作。
 */
export function cancelOperation(): void {
  const ws = useWsStore()
  ws.sendCancel()
}
