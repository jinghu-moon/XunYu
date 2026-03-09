import { flushPromises, mount } from '@vue/test-utils'
import { defineComponent } from 'vue'
import { describe, expect, it } from 'vitest'
import IntegrationAutomationWorkspace from './IntegrationAutomationWorkspace.vue'

const TaskToolboxStub = defineComponent({
  emits: ['link-panel'],
  template: `
    <div>
      <button
        data-testid="emit-toolbox-recent"
        @click="$emit('link-panel', { panel: 'recent-tasks', request: { status: 'failed', dry_run: 'executed', action: 'brn', search: 'D:/repo' } })"
      >
        recent
      </button>
      <button
        data-testid="emit-toolbox-audit"
        @click="$emit('link-panel', { panel: 'audit', request: { action: 'alias:sync', result: 'failed', search: 'alias' } })"
      >
        audit
      </button>
    </div>
  `,
})

const RecentTasksPanelStub = defineComponent({
  props: {
    focusRequest: { type: Object, default: null },
  },
  emits: ['link-panel'],
  template: '<div data-testid="integration-recent-focus">{{ JSON.stringify(focusRequest ?? null) }}</div>',
})

const RecipePanelStub = defineComponent({
  emits: ['link-panel'],
  template: `
    <button
      data-testid="emit-recipe-audit"
      @click="$emit('link-panel', { panel: 'audit', request: { action: 'completion', result: 'success', search: 'powershell' } })"
    >
      recipe
    </button>
  `,
})

const WorkspaceFrameStub = defineComponent({
  template: '<section><slot /></section>',
})

describe('IntegrationAutomationWorkspace', () => {
  it('focuses local recent tasks and re-emits audit links', async () => {
    const wrapper = mount(IntegrationAutomationWorkspace, {
      global: {
        stubs: {
          TaskToolbox: TaskToolboxStub,
          RecentTasksPanel: RecentTasksPanelStub,
          RecipePanel: RecipePanelStub,
          WorkspaceFrame: WorkspaceFrameStub,
        },
      },
    })

    await wrapper.get('[data-testid="emit-toolbox-recent"]').trigger('click')
    await flushPromises()
    expect(wrapper.get('[data-testid="integration-recent-focus"]').text()).toContain('"action":"brn"')
    expect(wrapper.get('[data-testid="integration-recent-focus"]').text()).toContain('"search":"D:/repo"')

    await wrapper.get('[data-testid="emit-toolbox-audit"]').trigger('click')
    await wrapper.get('[data-testid="emit-recipe-audit"]').trigger('click')

    expect(wrapper.emitted('link-panel')).toEqual([
      [{ panel: 'audit', request: { action: 'alias:sync', result: 'failed', search: 'alias' } }],
      [{ panel: 'audit', request: { action: 'completion', result: 'success', search: 'powershell' } }],
    ])
  })
})
