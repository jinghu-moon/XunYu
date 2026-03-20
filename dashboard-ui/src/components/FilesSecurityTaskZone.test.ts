import { mount } from '@vue/test-utils'
import { defineComponent } from 'vue'
import { describe, expect, it } from 'vitest'

import { filesSecurityTaskGroups } from '../features/tasks'
import FilesSecurityTaskZone from './FilesSecurityTaskZone.vue'

const TaskToolboxStub = defineComponent({
  props: {
    title: { type: String, default: '' },
    description: { type: String, default: '' },
    tasks: { type: Array, default: () => [] },
    taskPresets: { type: Object, default: null },
    presetVersion: { type: Number, default: 0 },
  },
  emits: ['link-panel'],
  template: `
    <div class="task-toolbox-stub" :data-title="title" :data-tasks="tasks.length" :data-version="presetVersion">
      {{ JSON.stringify(taskPresets ?? {}) }}
      <button
        data-testid="emit-link-panel"
        @click="$emit('link-panel', { panel: 'audit', request: { result: 'success', action: 'workspace.files.test', search: 'D:/repo/demo.txt' } })"
      >
        emit-link-panel
      </button>
    </div>
  `,
})

describe('FilesSecurityTaskZone', () => {
  it('renders every files-security task group with shared presets', () => {
    const wrapper = mount(FilesSecurityTaskZone, {
      props: {
        capabilities: null,
        taskPresets: { find: { paths: 'D:/repo/demo.txt' } },
        presetVersion: 3,
      },
      global: {
        stubs: {
          TaskToolbox: TaskToolboxStub,
        },
      },
    })

    const toolboxes = wrapper.findAll('.task-toolbox-stub')
    expect(toolboxes).toHaveLength(filesSecurityTaskGroups.length)
    expect(toolboxes[0].attributes('data-version')).toBe('3')
    expect(toolboxes[0].text()).toContain('D:/repo/demo.txt')
  })

  it('re-emits toolbox workspace links upward', async () => {
    const wrapper = mount(FilesSecurityTaskZone, {
      props: {
        capabilities: null,
        taskPresets: {},
        presetVersion: 0,
      },
      global: {
        stubs: {
          TaskToolbox: TaskToolboxStub,
        },
      },
    })

    await wrapper.get('[data-testid="emit-link-panel"]').trigger('click')

    expect(wrapper.emitted('link-panel')).toEqual([[
      {
        panel: 'audit',
        request: {
          result: 'success',
          action: 'workspace.files.test',
          search: 'D:/repo/demo.txt',
        },
      },
    ]])
  })
})
