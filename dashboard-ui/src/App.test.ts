import { flushPromises, mount } from '@vue/test-utils'
import { defineComponent } from 'vue'
import { beforeEach, describe, expect, it, vi } from 'vitest'

const apiMocks = vi.hoisted(() => ({
  fetchWorkspaceCapabilities: vi.fn(),
}))

vi.mock('./api', () => ({
  fetchWorkspaceCapabilities: apiMocks.fetchWorkspaceCapabilities,
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
})
