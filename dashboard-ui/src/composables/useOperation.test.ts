import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { useOperation } from './useOperation'
import { useWsStore } from '../stores/ws'

// Mock WebSocket
class MockWebSocket {
  static instances: MockWebSocket[] = []
  url: string
  readyState = 1
  onmessage: ((ev: { data: string }) => void) | null = null
  onclose: ((ev: { code: number; reason: string }) => void) | null = null
  onerror: (() => void) | null = null
  onopen: (() => void) | null = null
  private sent: string[] = []

  constructor(url: string) {
    this.url = url
    MockWebSocket.instances.push(this)
    setTimeout(() => {
      this.readyState = 1
      this.onopen?.()
    }, 0)
  }

  send(data: string) {
    this.sent.push(data)
  }

  close() {
    this.readyState = 3
    this.onclose?.({ code: 1000, reason: '' })
  }

  getLastSent(): string | undefined {
    return this.sent[this.sent.length - 1]
  }

  simulateMessage(data: unknown) {
    this.onmessage?.({ data: JSON.stringify(data) })
  }
}

describe('useOperation', () => {
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

  async function setupConnectedWs() {
    const store = useWsStore()
    store.connect()
    const ws = MockWebSocket.instances[0]
    await vi.waitFor(() => expect(store.isConnected).toBe(true))
    await new Promise((r) => setTimeout(r, 0))
    return { store, ws }
  }

  it('sends preview request', async () => {
    const { ws } = await setupConnectedWs()
    const op = useOperation()

    const promise = op.requestPreview('backup.create', ['--path', 'C:\\temp'])

    // Simulate preview response
    ws.simulateMessage({
      type: 'PreviewResult',
      payload: {
        preview: {
          description: 'Create backup of C:\\temp',
          changes: [{ action: 'create', target: 'backup.zip' }],
          risk_level: 'Low',
        },
      },
    })

    await promise
    expect(op.state.value).toBe('confirming')
    expect(op.preview.value).not.toBeNull()
    expect(op.preview.value!.description).toBe('Create backup of C:\\temp')
    expect(op.preview.value!.risk_level).toBe('Low')
  })

  it('shows dialog on preview (state transitions to confirming)', async () => {
    const { ws } = await setupConnectedWs()
    const op = useOperation()

    expect(op.state.value).toBe('idle')

    const promise = op.requestPreview('bookmark.delete', ['--name', 'test'])

    expect(op.state.value).toBe('previewing')

    ws.simulateMessage({
      type: 'PreviewResult',
      payload: {
        preview: {
          description: 'Delete bookmark "test"',
          changes: [{ action: 'delete', target: 'test' }],
          risk_level: 'Medium',
        },
      },
    })

    await promise
    expect(op.state.value).toBe('confirming')
  })

  it('sends confirm on user accept', async () => {
    const { ws } = await setupConnectedWs()
    const op = useOperation()

    // First preview
    const previewPromise = op.requestPreview('backup.create')
    ws.simulateMessage({
      type: 'PreviewResult',
      payload: {
        preview: {
          description: 'Create backup',
          changes: [],
          risk_level: 'Low',
        },
      },
    })
    await previewPromise
    expect(op.state.value).toBe('confirming')

    // Then confirm
    const confirmPromise = op.confirm()
    expect(op.state.value).toBe('executing')

    ws.simulateMessage({
      type: 'OpResult',
      payload: {
        result: { changes_applied: 1, duration_ms: 200, rollback_available: true },
      },
    })

    await confirmPromise
    expect(op.state.value).toBe('done')
    expect(op.result.value).not.toBeNull()
    expect(op.result.value!.changes_applied).toBe(1)
    expect(op.result.value!.rollback_available).toBe(true)
  })

  it('handles cancel', async () => {
    const { ws } = await setupConnectedWs()
    const op = useOperation()

    const promise = op.requestPreview('backup.create')
    ws.simulateMessage({
      type: 'PreviewResult',
      payload: {
        preview: {
          description: 'Create backup',
          changes: [],
          risk_level: 'Low',
        },
      },
    })
    await promise

    op.cancel()
    expect(op.state.value).toBe('cancelled')
    expect(op.preview.value).toBeNull()
  })

  it('handles preview error', async () => {
    const { ws } = await setupConnectedWs()
    const op = useOperation()

    const promise = op.requestPreview('unknown.op')
    ws.simulateMessage({
      type: 'Error',
      payload: { message: 'operation not found', code: 'NOT_FOUND' },
    })

    await promise
    expect(op.state.value).toBe('error')
    expect(op.error.value).toBe('operation not found')
  })

  it('handles confirm error', async () => {
    const { ws } = await setupConnectedWs()
    const op = useOperation()

    // Preview first
    const previewPromise = op.requestPreview('backup.create')
    ws.simulateMessage({
      type: 'PreviewResult',
      payload: {
        preview: { description: 'Create backup', changes: [], risk_level: 'Low' },
      },
    })
    await previewPromise

    // Confirm with error
    const confirmPromise = op.confirm()
    ws.simulateMessage({
      type: 'Error',
      payload: { message: 'execution failed', code: 'EXECUTION_FAILED' },
    })

    await confirmPromise
    expect(op.state.value).toBe('error')
    expect(op.error.value).toBe('execution failed')
  })

  it('resets state', async () => {
    const { ws } = await setupConnectedWs()
    const op = useOperation()

    const promise = op.requestPreview('backup.create')
    ws.simulateMessage({
      type: 'PreviewResult',
      payload: {
        preview: { description: 'Create backup', changes: [], risk_level: 'Low' },
      },
    })
    await promise

    op.reset()
    expect(op.state.value).toBe('idle')
    expect(op.preview.value).toBeNull()
    expect(op.result.value).toBeNull()
    expect(op.error.value).toBeNull()
  })

  it('rejects confirm without preview', async () => {
    await setupConnectedWs()
    const op = useOperation()

    await op.confirm()
    expect(op.state.value).toBe('error')
    expect(op.error.value).toBe('No preview to confirm')
  })
})
