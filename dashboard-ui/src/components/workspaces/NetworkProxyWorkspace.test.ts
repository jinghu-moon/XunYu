import { mount } from '@vue/test-utils'
import { defineComponent } from 'vue'
import { describe, expect, it } from 'vitest'
import NetworkProxyWorkspace from './NetworkProxyWorkspace.vue'

const PortsPanelStub = defineComponent({
  template: '<div data-testid="ports-stub">ports</div>',
})

const ProxyPanelStub = defineComponent({
  template: '<div data-testid="proxy-stub">proxy</div>',
})

const RecentTasksPanelStub = defineComponent({
  emits: ['link-panel'],
  template: `
    <button
      data-testid="emit-network-recent-diagnostics"
      @click="$emit('link-panel', { panel: 'diagnostics-center', request: { panel: 'guarded', target: '7890' } })"
    >
      recent
    </button>
  `,
})

const RecipePanelStub = defineComponent({
  emits: ['link-panel'],
  template: `
    <button
      data-testid="emit-network-recipe-audit"
      @click="$emit('link-panel', { panel: 'audit', request: { action: 'pst', result: 'success', search: 'proxy' } })"
    >
      recipe
    </button>
  `,
})

const TaskToolboxStub = defineComponent({
  emits: ['link-panel'],
  template: `
    <button
      data-testid="emit-network-toolbox-recent"
      @click="$emit('link-panel', { panel: 'recent-tasks', request: { status: 'succeeded', dry_run: 'executed', action: 'px', search: 'curl' } })"
    >
      toolbox
    </button>
  `,
})

const WorkspaceFrameStub = defineComponent({
  template: '<section><slot /></section>',
})

describe('NetworkProxyWorkspace', () => {
  it('re-emits task, recipe, and recent-task links upward', async () => {
    const wrapper = mount(NetworkProxyWorkspace, {
      global: {
        stubs: {
          PortsPanel: PortsPanelStub,
          ProxyPanel: ProxyPanelStub,
          RecentTasksPanel: RecentTasksPanelStub,
          RecipePanel: RecipePanelStub,
          TaskToolbox: TaskToolboxStub,
          WorkspaceFrame: WorkspaceFrameStub,
        },
      },
    })

    await wrapper.get('[data-testid="emit-network-recent-diagnostics"]').trigger('click')
    await wrapper.get('[data-testid="emit-network-recipe-audit"]').trigger('click')
    await wrapper.get('[data-testid="emit-network-toolbox-recent"]').trigger('click')

    expect(wrapper.emitted('link-panel')).toEqual([
      [{ panel: 'diagnostics-center', request: { panel: 'guarded', target: '7890' } }],
      [{ panel: 'audit', request: { action: 'pst', result: 'success', search: 'proxy' } }],
      [{ panel: 'recent-tasks', request: { status: 'succeeded', dry_run: 'executed', action: 'px', search: 'curl' } }],
    ])
  })
})
