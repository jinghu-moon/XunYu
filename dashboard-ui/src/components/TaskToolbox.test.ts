import { mount } from '@vue/test-utils'
import { defineComponent } from 'vue'
import { describe, expect, it } from 'vitest'
import type { WorkspaceTaskDefinition } from '../workspace-tools'
import TaskToolbox from './TaskToolbox.vue'

const TaskToolCardStub = defineComponent({
  emits: ['link-panel'],
  template: `
    <button
      data-testid="toolbox-child-link"
      type="button"
      @click="$emit('link-panel', { panel: 'recent-tasks', request: { status: 'failed', dry_run: 'executed' } })"
    >
      child
    </button>
  `,
})

describe('TaskToolbox', () => {
  it('forwards link-panel events from task cards', async () => {
    const task: WorkspaceTaskDefinition = {
      id: 'recent',
      workspace: 'statistics-diagnostics',
      title: '????',
      description: 'desc',
      action: 'cstat',
      mode: 'run',
      fields: [],
      buildRunArgs: () => ['cstat', '.'],
    }

    const wrapper = mount(TaskToolbox, {
      props: {
        title: '????',
        tasks: [task],
      },
      global: {
        stubs: {
          TaskToolCard: TaskToolCardStub,
        },
      },
    })

    await wrapper.get('[data-testid="toolbox-child-link"]').trigger('click')

    expect(wrapper.emitted('link-panel')).toHaveLength(1)
    expect(wrapper.emitted('link-panel')?.[0]?.[0]).toMatchObject({
      panel: 'recent-tasks',
      request: {
        status: 'failed',
        dry_run: 'executed',
      },
    })
  })
})
