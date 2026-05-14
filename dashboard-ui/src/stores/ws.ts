import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { WsCommand, WsResponse } from '../api/ws-types'

interface PendingRequest {
  resolve: (value: WsResponse) => void
  reject: (reason: Error) => void
  timer: ReturnType<typeof setTimeout>
}

const RECONNECT_DELAY_MS = 1000
const REQUEST_TIMEOUT_MS = 30_000

// WebSocket readyState constants (not available as static props in jsdom)
const WS_CONNECTING = 0
const WS_OPEN = 1

export const useWsStore = defineStore('ws', () => {
  const isConnected = ref(false)
  let ws: WebSocket | null = null
  let reconnectTimer: ReturnType<typeof setTimeout> | null = null
  let requestIdCounter = 0
  const pending = new Map<string, PendingRequest>()

  function getWsUrl(): string {
    const proto = window.location.protocol === 'https:' ? 'wss' : 'ws'
    return `${proto}://${window.location.host}/ws`
  }

  function connect() {
    if (ws && (ws.readyState === WS_OPEN || ws.readyState === WS_CONNECTING)) {
      return
    }

    ws = new WebSocket(getWsUrl())

    ws.onopen = () => {
      isConnected.value = true
      if (reconnectTimer) {
        clearTimeout(reconnectTimer)
        reconnectTimer = null
      }
    }

    ws.onmessage = (ev) => {
      let parsed: WsResponse
      try {
        parsed = JSON.parse(ev.data)
      } catch {
        return
      }

      // Match by request_id if present (future extension),
      // for now resolve the oldest pending request
      const firstKey = pending.keys().next().value
      if (firstKey !== undefined) {
        const req = pending.get(firstKey)!
        pending.delete(firstKey)
        clearTimeout(req.timer)

        if (parsed.type === 'Error') {
          req.reject(new Error(parsed.payload.message))
        } else {
          req.resolve(parsed)
        }
      }
    }

    ws.onclose = (ev) => {
      isConnected.value = false
      ws = null

      // Reject all pending requests
      for (const [key, req] of pending) {
        clearTimeout(req.timer)
        req.reject(new Error('WebSocket closed'))
        pending.delete(key)
      }

      // Auto-reconnect on abnormal close
      if (ev.code !== 1000) {
        scheduleReconnect()
      }
    }

    ws.onerror = () => {
      // onerror is always followed by onclose, so cleanup happens there
    }
  }

  function scheduleReconnect() {
    if (reconnectTimer) return
    reconnectTimer = setTimeout(() => {
      reconnectTimer = null
      connect()
    }, RECONNECT_DELAY_MS)
  }

  function disconnect() {
    if (reconnectTimer) {
      clearTimeout(reconnectTimer)
      reconnectTimer = null
    }
    if (ws) {
      ws.close(1000, 'client disconnect')
      ws = null
    }
    isConnected.value = false
  }

  function sendRaw(command: WsCommand): Promise<WsResponse> {
    return new Promise((resolve, reject) => {
      if (!ws || ws.readyState !== WS_OPEN) {
        reject(new Error('WebSocket not connected'))
        return
      }

      const id = String(++requestIdCounter)
      const timer = setTimeout(() => {
        pending.delete(id)
        reject(new Error('Request timeout'))
      }, REQUEST_TIMEOUT_MS)

      pending.set(id, { resolve, reject, timer })
      ws.send(JSON.stringify(command))
    })
  }

  function sendCommand(command: string, args?: string[]): Promise<WsResponse> {
    return sendRaw({ type: 'Query', payload: { command, args: args ?? [] } })
  }

  function sendPreview(operation: string, args?: string[]): Promise<WsResponse> {
    return sendRaw({ type: 'PreviewOp', payload: { operation, args: args ?? [] } })
  }

  function sendConfirm(operation: string, args?: string[]): Promise<WsResponse> {
    return sendRaw({ type: 'ConfirmOp', payload: { operation, args: args ?? [] } })
  }

  function sendCancel(): void {
    if (!ws || ws.readyState !== WS_OPEN) return
    ws.send(JSON.stringify({ type: 'CancelOp' } satisfies WsCommand))
  }

  return {
    isConnected,
    connect,
    disconnect,
    sendCommand,
    sendPreview,
    sendConfirm,
    sendCancel,
  }
})
