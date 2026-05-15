/**
 * WS 协议类型定义
 *
 * 与后端 src/xun_core/ws_protocol.rs 保持一致。
 * 所有消息均为 JSON，通过 serde tag="type" content="payload" 格式。
 */

// ── 前端 → 后端命令 ────────────────────────────────────────

export type WsCommand =
  | WsQueryCommand
  | WsPreviewOpCommand
  | WsConfirmOpCommand
  | WsCancelOpCommand

export interface WsQueryCommand {
  type: 'Query'
  payload: {
    command: string
    args?: string[]
  }
}

export interface WsPreviewOpCommand {
  type: 'PreviewOp'
  payload: {
    operation: string
    args?: string[]
  }
}

export interface WsConfirmOpCommand {
  type: 'ConfirmOp'
  payload: {
    operation: string
    args?: string[]
  }
}

export interface WsCancelOpCommand {
  type: 'CancelOp'
}

// ── 后端 → 前端响应 ────────────────────────────────────────

export type WsResponse =
  | WsQueryResultResponse
  | WsPreviewResultResponse
  | WsOpResultResponse
  | WsErrorResponse
  | WsConnectedResponse

export interface WsQueryResultResponse {
  type: 'QueryResult'
  payload: {
    table: WsTable
  }
}

export interface WsPreviewResultResponse {
  type: 'PreviewResult'
  payload: {
    preview: WsPreview
  }
}

export interface WsOpResultResponse {
  type: 'OpResult'
  payload: {
    result: WsOperationResult
  }
}

export interface WsErrorResponse {
  type: 'Error'
  payload: {
    message: string
    code: WsErrorCode
  }
}

export interface WsConnectedResponse {
  type: 'Connected'
}

// ── 值类型 ──────────────────────────────────────────────────

export interface WsTable {
  columns: WsColumnDef[]
  rows: WsTableRow[]
}

export interface WsColumnDef {
  name: string
  kind: string
}

export type WsTableRow = Record<string, unknown>

export interface WsPreview {
  description: string
  changes: WsChange[]
  risk_level: WsRiskLevel
}

export interface WsChange {
  action: string
  target: string
  detail?: string
}

export type WsRiskLevel = 'Low' | 'Medium' | 'High' | 'Critical'

export interface WsOperationResult {
  changes_applied: number
  duration_ms: number
  rollback_available: boolean
}

// ── 错误码 ──────────────────────────────────────────────────

export type WsErrorCode =
  | 'NOT_FOUND'
  | 'INVALID_ARGS'
  | 'EXECUTION_FAILED'
  | 'PREVIEW_REQUIRED'
  | 'UNKNOWN'
