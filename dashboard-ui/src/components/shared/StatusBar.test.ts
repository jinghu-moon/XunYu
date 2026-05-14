import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { setActivePinia, createPinia } from 'pinia'
import StatusBar from './StatusBar.vue'
import { useWsStore } from '../../stores/ws'

class MockWebSocket {
  static instances: MockWebSocket[] = []
  url: string
  readyState = 1
  onmessage: ((ev: { data: string }) => void) | null = null
  onclose: ((ev: { code: number; reason: string }) => void) | null = null
  onerror: (() => void) | null = null
  onopen: (() => void) | null = null

  constructor(url: string) {
    this.url = url
    MockWebSocket.instances.push(this)
    setTimeout(() => {
      this.readyState = 1
      this.onopen?.()
    }, 0)
  }

  send() {}
  close() {}
}

describe('StatusBar', () => {
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

  it('shows disconnected state by default', () => {
    const wrapper = mount(StatusBar)
    expect(wrapper.find('[data-testid="ws-status"]').text()).toContain('Disconnected')
  })

  it('shows connected state after WS connect', async () => {
    const store = useWsStore()
    store.connect()
    await vi.waitFor(() => expect(store.isConnected).toBe(true))

    const wrapper = mount(StatusBar)
    expect(wrapper.find('[data-testid="ws-status"]').text()).toContain('Connected')
  })

  it('shows last operation result', async () => {
    const wrapper = mount(StatusBar, {
      props: { lastOperation: 'bookmark.delete' },
    })
    expect(wrapper.find('[data-testid="last-op"]').text()).toContain('bookmark.delete')
  })
})
