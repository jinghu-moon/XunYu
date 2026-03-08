import { afterEach, vi } from 'vitest'
import { config } from '@vue/test-utils'

config.global.stubs = {
  teleport: true,
}

if (!globalThis.ResizeObserver) {
  class ResizeObserverStub {
    observe() {}
    unobserve() {}
    disconnect() {}
  }
  ;(globalThis as typeof globalThis & { ResizeObserver: typeof ResizeObserverStub }).ResizeObserver = ResizeObserverStub
}

if (!globalThis.requestAnimationFrame) {
  globalThis.requestAnimationFrame = (callback: FrameRequestCallback) => {
    return window.setTimeout(() => callback(performance.now()), 0)
  }
}

if (!globalThis.cancelAnimationFrame) {
  globalThis.cancelAnimationFrame = (handle: number) => {
    window.clearTimeout(handle)
  }
}

afterEach(() => {
  vi.restoreAllMocks()
})
