/**
 * 统一命令调用层
 *
 * 通过 WS store 发送 Query 命令，解析 Table 响应。
 * 对于已有 HTTP API 的命令，保留 HTTP 通道；新增命令统一走 WS。
 */

import { useWsStore } from '../stores/ws'
import type { WsTable, WsResponse } from './ws-types'

/**
 * 发送 Query 命令并返回 Table 数据。
 */
export async function queryCommand(command: string, args?: string[]): Promise<WsTable> {
  const ws = useWsStore()
  const resp = await ws.sendCommand(command, args)
  if (resp.type !== 'QueryResult') {
    throw new Error(`Unexpected response type: ${resp.type}`)
  }
  return resp.payload.table
}

/**
 * 发送 Query 命令并返回原始响应（用于需要完整响应的场景）。
 */
export async function queryCommandRaw(command: string, args?: string[]): Promise<WsResponse> {
  const ws = useWsStore()
  return ws.sendCommand(command, args)
}
