import { flushPromises, mount } from '@vue/test-utils'
import { defineComponent } from 'vue'
import { describe, expect, it } from 'vitest'
import MediaConversionWorkspace from './MediaConversionWorkspace.vue'

const TaskToolboxStub = defineComponent({
  emits: ['link-panel'],
  template: `
    <div>
      <button
        data-testid="emit-toolbox-recent"
        @click="$emit('link-panel', { panel: 'recent-tasks', request: { status: 'succeeded', dry_run: 'executed', action: 'video:compress', search: 'demo.mp4' } })"
      >
        recent
      </button>
      <button
        data-testid="emit-toolbox-audit"
        @click="$emit('link-panel', { panel: 'audit', request: { action: 'video:compress', result: 'success', search: 'demo.small.mp4' } })"
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
  template: '<div data-testid="media-recent-focus">{{ JSON.stringify(focusRequest ?? null) }}</div>',
})

const RecipePanelStub = defineComponent({
  emits: ['link-panel'],
  template: `
    <button
      data-testid="emit-recipe-diagnostics"
      @click="$emit('link-panel', { panel: 'diagnostics-center', request: { panel: 'audit', audit_action: 'video:compress', target: 'demo.small.mp4' } })"
    >
      recipe
    </button>
  `,
})

const WorkspaceFrameStub = defineComponent({
  template: '<section><slot /></section>',
})

describe('MediaConversionWorkspace', () => {
  it('focuses local recent tasks and re-emits diagnostics links', async () => {
    const wrapper = mount(MediaConversionWorkspace, {
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
    expect(wrapper.get('[data-testid="media-recent-focus"]').text()).toContain('"action":"video:compress"')
    expect(wrapper.get('[data-testid="media-recent-focus"]').text()).toContain('"search":"demo.mp4"')

    await wrapper.get('[data-testid="emit-toolbox-audit"]').trigger('click')
    await wrapper.get('[data-testid="emit-recipe-diagnostics"]').trigger('click')

    expect(wrapper.emitted('link-panel')).toEqual([
      [{ panel: 'audit', request: { action: 'video:compress', result: 'success', search: 'demo.small.mp4' } }],
      [{ panel: 'diagnostics-center', request: { panel: 'audit', audit_action: 'video:compress', target: 'demo.small.mp4' } }],
    ])
  })
})
