import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { useWsStore } from './ws'

// Mock WebSocket
class MockWebSocket {
  static instances: MockWebSocket[] = []
  url: string
  readyState = 1 // OPEN
  onmessage: ((ev: { data: string }) => void) | null = null
  onclose: ((ev: { code: number; reason: string }) => void) | null = null
  onerror: (() => void) | null = null
  onopen: (() => void) | null = null
  private sent: string[] = []

  constructor(url: string) {
    this.url = url
    MockWebSocket.instances.push(this)
    // Simulate async open
    setTimeout(() => {
      this.readyState = 1
      this.onopen?.()
    }, 0)
  }

  send(data: string) {
    this.sent.push(data)
  }

  close() {
    this.readyState = 3 // CLOSED
    this.onclose?.({ code: 1000, reason: '' })
  }

  getLastSent(): string | undefined {
    return this.sent[this.sent.length - 1]
  }

  getAllSent(): string[] {
    return [...this.sent]
  }

  simulateMessage(data: unknown) {
    this.onmessage?.({ data: JSON.stringify(data) })
  }

  simulateError() {
    this.onerror?.()
  }

  simulateClose(code = 1000, reason = '') {
    this.readyState = 3
    this.onclose?.({ code, reason })
  }
}

describe('useWsStore', () => {
  let originalWebSocket: typeof globalThis.WebSocket

  beforeEach(() => {
    setActivePinia(createPinia())
    originalWebSocket = globalThis.WebSocket
    MockWebSocket.instances = []
    ;(globalThis as any).WebSocket = MockWebSocket
  })

  afterEach(() => {
    globalThis.WebSocket = originalWebSocket
  })

  it('connects to WebSocket on connect()', () => {
    const store = useWsStore()
    store.connect()
    expect(MockWebSocket.instances.length).toBe(1)
    expect(MockWebSocket.instances[0].url).toContain('/ws')
  })

  it('sends command and receives response', async () => {
    const store = useWsStore()
    store.connect()

    const ws = MockWebSocket.instances[0]
    // Wait for open
    await vi.waitFor(() => expect(store.isConnected).toBe(true))
    await new Promise((r) => setTimeout(r, 0))

    const promise = store.sendCommand('bookmark.list', ['--tag', 'shell'])

    // Verify sent message
    const sent = JSON.parse(ws.getLastSent()!)
    expect(sent.type).toBe('Query')
    expect(sent.payload.command).toBe('bookmark.list')
    expect(sent.payload.args).toEqual(['--tag', 'shell'])

    // Simulate response
    ws.simulateMessage({
      type: 'QueryResult',
      payload: {
        table: {
          columns: [{ name: 'name', kind: 'String' }],
          rows: [{ name: 'test' }],
        },
      },
    })

    const result = await promise
    expect(result.type).toBe('QueryResult')
    if (result.type === 'QueryResult') {
      expect(result.payload.table.rows).toEqual([{ name: 'test' }])
    }
  })

  it('handles preview response', async () => {
    const store = useWsStore()
    store.connect()
    const ws = MockWebSocket.instances[0]
    await vi.waitFor(() => expect(store.isConnected).toBe(true))
    await new Promise((r) => setTimeout(r, 0))

    const promise = store.sendPreview('backup.create', ['--path', 'C:\\temp'])

    const sent = JSON.parse(ws.getLastSent()!)
    expect(sent.type).toBe('PreviewOp')
    expect(sent.payload.operation).toBe('backup.create')

    ws.simulateMessage({
      type: 'PreviewResult',
      payload: {
        preview: {
          description: 'Create backup',
          changes: [{ action: 'create', target: 'backup.zip' }],
          risk_level: 'Low',
        },
      },
    })

    const result = await promise
    expect(result.type).toBe('PreviewResult')
    if (result.type === 'PreviewResult') {
      expect(result.payload.preview.description).toBe('Create backup')
    }
  })

  it('rejects on error response', async () => {
    const store = useWsStore()
    store.connect()
    const ws = MockWebSocket.instances[0]
    await vi.waitFor(() => expect(store.isConnected).toBe(true))
    await new Promise((r) => setTimeout(r, 0))

    const promise = store.sendCommand('unknown.cmd')

    ws.simulateMessage({
      type: 'Error',
      payload: { message: 'command not found', code: 'NOT_FOUND' },
    })

    await expect(promise).rejects.toThrow('command not found')
  })

  it('reconnects on disconnect', async () => {
    const store = useWsStore()
    store.connect()
    const ws = MockWebSocket.instances[0]
    await vi.waitFor(() => expect(store.isConnected).toBe(true))
    await new Promise((r) => setTimeout(r, 0))

    // Simulate disconnect
    ws.simulateClose(1006, 'abnormal')
    expect(store.isConnected).toBe(false)

    // Wait for reconnect (RECONNECT_DELAY_MS = 1000)
    await new Promise((r) => setTimeout(r, 1100))
    expect(MockWebSocket.instances.length).toBe(2)
  })

  it('rejects pending promises on disconnect', async () => {
    const store = useWsStore()
    store.connect()
    const ws = MockWebSocket.instances[0]
    await vi.waitFor(() => expect(store.isConnected).toBe(true))
    await new Promise((r) => setTimeout(r, 0))

    const promise = store.sendCommand('bookmark.list')

    // Simulate disconnect before response
    ws.simulateClose(1006, 'abnormal')

    await expect(promise).rejects.toThrow()
  })

  it('sends confirm command', async () => {
    const store = useWsStore()
    store.connect()
    const ws = MockWebSocket.instances[0]
    await vi.waitFor(() => expect(store.isConnected).toBe(true))
    // Ensure microtask queue is flushed so ws reference is stable
    await new Promise((r) => setTimeout(r, 0))

    const promise = store.sendConfirm('backup.create', ['--path', 'C:\\temp'])

    const sent = JSON.parse(ws.getLastSent()!)
    expect(sent.type).toBe('ConfirmOp')
    expect(sent.payload.operation).toBe('backup.create')

    ws.simulateMessage({
      type: 'OpResult',
      payload: { result: { changes_applied: 1, duration_ms: 100, rollback_available: true } },
    })

    const result = await promise
    expect(result.type).toBe('OpResult')
  })

  it('sends cancel command', async () => {
    const store = useWsStore()
    store.connect()
    const ws = MockWebSocket.instances[0]
    await vi.waitFor(() => expect(store.isConnected).toBe(true))
    await new Promise((r) => setTimeout(r, 0))

    store.sendCancel()

    const sent = JSON.parse(ws.getLastSent()!)
    expect(sent.type).toBe('CancelOp')
  })

  it('disconnects cleanly', async () => {
    const store = useWsStore()
    store.connect()
    const ws = MockWebSocket.instances[0]
    await vi.waitFor(() => expect(store.isConnected).toBe(true))
    await new Promise((r) => setTimeout(r, 0))

    store.disconnect()
    expect(store.isConnected).toBe(false)
    expect(ws.readyState).toBe(3)
  })
})
