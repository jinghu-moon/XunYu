import { flushPromises, mount } from '@vue/test-utils'
import { defineComponent } from 'vue'
import { describe, expect, it } from 'vitest'
import PathsContextWorkspace from './PathsContextWorkspace.vue'

const BookmarksPanelStub = defineComponent({
  template: '<div data-testid="bookmarks-stub">bookmarks</div>',
})

const TaskToolboxStub = defineComponent({
  emits: ['link-panel'],
  template: `
    <div>
      <button
        data-testid="emit-toolbox-recent"
        @click="$emit('link-panel', { panel: 'recent-tasks', request: { status: 'succeeded', dry_run: 'executed', action: 'recent', search: 'docs' } })"
      >
        recent
      </button>
      <button
        data-testid="emit-toolbox-audit"
        @click="$emit('link-panel', { panel: 'audit', request: { action: 'ctx:use', result: 'success', search: 'work' } })"
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
  template: `
    <div>
      <div data-testid="paths-recent-focus">{{ JSON.stringify(focusRequest ?? null) }}</div>
      <button
        data-testid="emit-recent-diagnostics"
        @click="$emit('link-panel', { panel: 'diagnostics-center', request: { panel: 'failed', task_id: 'path-task-1' } })"
      >
        diagnostics
      </button>
    </div>
  `,
})

const RecipePanelStub = defineComponent({
  emits: ['link-panel'],
  template: `
    <button
      data-testid="emit-recipe-diagnostics"
      @click="$emit('link-panel', { panel: 'diagnostics-center', request: { panel: 'audit', audit_action: 'ctx:use', target: 'D:/repo' } })"
    >
      recipe
    </button>
  `,
})

const WorkspaceFrameStub = defineComponent({
  template: '<section><slot /></section>',
})

describe('PathsContextWorkspace', () => {
  it('focuses local recent tasks and re-emits diagnostics links', async () => {
    const wrapper = mount(PathsContextWorkspace, {
      global: {
        stubs: {
          BookmarksPanel: BookmarksPanelStub,
          TaskToolbox: TaskToolboxStub,
          RecentTasksPanel: RecentTasksPanelStub,
          RecipePanel: RecipePanelStub,
          WorkspaceFrame: WorkspaceFrameStub,
        },
      },
    })

    await wrapper.get('[data-testid="emit-toolbox-recent"]').trigger('click')
    await flushPromises()
    expect(wrapper.get('[data-testid="paths-recent-focus"]').text()).toContain('"action":"recent"')
    expect(wrapper.get('[data-testid="paths-recent-focus"]').text()).toContain('"search":"docs"')

    await wrapper.get('[data-testid="emit-toolbox-audit"]').trigger('click')
    await wrapper.get('[data-testid="emit-recent-diagnostics"]').trigger('click')
    await wrapper.get('[data-testid="emit-recipe-diagnostics"]').trigger('click')

    expect(wrapper.emitted('link-panel')).toEqual([
      [{ panel: 'audit', request: { action: 'ctx:use', result: 'success', search: 'work' } }],
      [{ panel: 'diagnostics-center', request: { panel: 'failed', task_id: 'path-task-1' } }],
      [{ panel: 'diagnostics-center', request: { panel: 'audit', audit_action: 'ctx:use', target: 'D:/repo' } }],
    ])
  })
})
