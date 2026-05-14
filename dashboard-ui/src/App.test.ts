import { flushPromises, mount } from '@vue/test-utils'
import { defineComponent, h, nextTick } from 'vue'
import { beforeEach, describe, expect, it, vi } from 'vitest'

const apiMocks = vi.hoisted(() => ({
  fetchWorkspaceCapabilities: vi.fn(),
  fetchBookmarks: vi.fn(),
  fetchProxyStatus: vi.fn(),
  fetchProxyConfig: vi.fn(),
  fetchPorts: vi.fn(),
  fetchWorkspaceOverviewSummary: vi.fn(),
  queryCommand: vi.fn(),
}))

const operationMocks = vi.hoisted(() => ({
  useOperation: vi.fn(() => ({
    state: { value: 'idle' },
    preview: { value: null },
    result: { value: null },
    error: { value: null },
    requestPreview: vi.fn(),
    confirm: vi.fn(),
    cancel: vi.fn(),
    reset: vi.fn(),
  })),
}))

vi.mock('./api', () => ({
  fetchWorkspaceCapabilities: apiMocks.fetchWorkspaceCapabilities,
  fetchBookmarks: apiMocks.fetchBookmarks,
  fetchProxyStatus: apiMocks.fetchProxyStatus,
  fetchProxyConfig: apiMocks.fetchProxyConfig,
  fetchPorts: apiMocks.fetchPorts,
  fetchWorkspaceOverviewSummary: apiMocks.fetchWorkspaceOverviewSummary,
}))

vi.mock('./api/commands', () => ({
  queryCommand: apiMocks.queryCommand,
}))

vi.mock('./composables/useOperation', () => ({
  useOperation: operationMocks.useOperation,
}))

vi.mock('./components/workspaces/OverviewWorkspace.vue', async () => {
  const component = defineComponent({
    name: 'OverviewWorkspaceMock',
    emits: ['link-panel'],
    template: `
      <button
        data-testid="emit-overview-link"
        @click="$emit('link-panel', { panel: 'audit', request: { action: 'recent', result: 'success', search: 'demo.log' } })"
      >
        overview-link
      </button>
    `,
  })

  return {
    __esModule: true,
    __isTeleport: false,
    __isKeepAlive: false,
    __isSuspense: false,
    __name: 'OverviewWorkspaceMock',
    name: 'OverviewWorkspaceMock',
    ...component,
    default: component,
  }
})

vi.mock('./components/workspaces/StatisticsDiagnosticsWorkspace.vue', async () => {
  const component = defineComponent({
    name: 'StatisticsDiagnosticsWorkspaceMock',
    props: {
      externalLink: { type: Object, default: null },
    },
    template: '<div data-testid="stats-external-link">{{ JSON.stringify(externalLink ?? null) }}</div>',
  })

  return {
    __esModule: true,
    __isTeleport: false,
    __isKeepAlive: false,
    __isSuspense: false,
    __name: 'StatisticsDiagnosticsWorkspaceMock',
    name: 'StatisticsDiagnosticsWorkspaceMock',
    ...component,
    default: component,
  }
})

describe('App', () => {
  beforeEach(() => {
    apiMocks.fetchWorkspaceCapabilities.mockReset()
    apiMocks.fetchWorkspaceCapabilities.mockResolvedValue({})
    apiMocks.fetchBookmarks.mockReset()
    apiMocks.fetchBookmarks.mockResolvedValue([])
    apiMocks.fetchProxyStatus.mockReset()
    apiMocks.fetchProxyStatus.mockResolvedValue({})
    apiMocks.fetchProxyConfig.mockReset()
    apiMocks.fetchProxyConfig.mockResolvedValue({})
    apiMocks.fetchPorts.mockReset()
    apiMocks.fetchPorts.mockResolvedValue({ tcp: [], udp: [] })
    apiMocks.fetchWorkspaceOverviewSummary.mockReset()
    apiMocks.fetchWorkspaceOverviewSummary.mockResolvedValue({})
    apiMocks.queryCommand.mockReset()
    apiMocks.queryCommand.mockResolvedValue({ columns: [], rows: [] })
  })

  it('routes workspace link payloads into statistics diagnostics workspace', async () => {
    const { default: App } = await import('./App.vue')
    const wrapper = mount(App, {
      global: {
        stubs: {
          CapsuleTabs: defineComponent({
            props: { modelValue: { type: String, default: '' }, items: { type: Array, default: () => [] } },
            emits: ['update:modelValue'],
            template: '<div data-testid="tabs-stub">tabs</div>',
          }),
          CommandPalette: true,
          DensityToggle: true,
          ThemeToggle: true,
          GlobalFeedback: true,
        },
      },
    })

    await vi.dynamicImportSettled()
    await flushPromises()
    await flushPromises()

    await wrapper.get('[data-testid="emit-overview-link"]').trigger('click')
    await vi.dynamicImportSettled()
    await flushPromises()
    await flushPromises()

    const linkText = wrapper.get('[data-testid="stats-external-link"]').text()
    expect(linkText).toContain('"panel":"audit"')
    expect(linkText).toContain('"action":"recent"')
    expect(linkText).toContain('"search":"demo.log"')
  })

  describe('keyboard navigation (Tab/Shift+Tab)', () => {
    // Use a sentinel stub that exposes modelValue via data attribute
    const tabsModelValues: string[] = []
    const TabsStub = defineComponent({
      props: { modelValue: { type: String, default: '' }, items: { type: Array, default: () => [] } },
      emits: ['update:modelValue'],
      setup(props) {
        tabsModelValues.push(props.modelValue)
        return () => h('div', { 'data-testid': 'tabs-stub', 'data-model': props.modelValue }, 'tabs')
      },
    })

    async function mountApp() {
      tabsModelValues.length = 0
      const { default: App } = await import('./App.vue')
      const wrapper = mount(App, {
        global: {
          stubs: {
            CapsuleTabs: TabsStub,
            CommandPalette: true,
            DensityToggle: true,
            ThemeToggle: true,
            GlobalFeedback: true,
          },
        },
      })
      await vi.dynamicImportSettled()
      await flushPromises()
      await flushPromises()
      return wrapper
    }

    it('Tab switches to next workspace', async () => {
      const wrapper = await mountApp()

      window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Tab', bubbles: true }))
      await nextTick()

      // Initial: overview (index 0), Tab → paths-context (index 1)
      const modelValue = wrapper.find('[data-testid="tabs-stub"]').attributes('data-model')
      expect(modelValue).toBe('paths-context')
    })

    it('Shift+Tab switches to previous workspace', async () => {
      const wrapper = await mountApp()

      // First Tab to paths-context
      window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Tab', bubbles: true }))
      await nextTick()

      // Then Shift+Tab back to overview
      window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Tab', shiftKey: true, bubbles: true }))
      await nextTick()

      const modelValue = wrapper.find('[data-testid="tabs-stub"]').attributes('data-model')
      expect(modelValue).toBe('overview')
    })

    it('Tab wraps around from last to first', async () => {
      const wrapper = await mountApp()

      // Set to last workspace via emitting update:modelValue
      wrapper.findComponent(TabsStub).vm.$emit('update:modelValue', 'statistics-diagnostics')
      await nextTick()

      // Tab should wrap to overview
      window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Tab', bubbles: true }))
      await nextTick()

      const modelValue = wrapper.find('[data-testid="tabs-stub"]').attributes('data-model')
      expect(modelValue).toBe('overview')
    })

    it('Tab is ignored when focus is on input element', async () => {
      const wrapper = await mountApp()

      const input = document.createElement('input')
      document.body.appendChild(input)
      input.focus()

      // Dispatch from input so e.target is the input element (bubbles to window)
      input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Tab', bubbles: true }))
      await nextTick()

      const modelValue = wrapper.find('[data-testid="tabs-stub"]').attributes('data-model')
      expect(modelValue).toBe('overview') // unchanged

      document.body.removeChild(input)
    })
  })
})
