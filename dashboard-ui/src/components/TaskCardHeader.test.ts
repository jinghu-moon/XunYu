import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import TaskCardHeader from './TaskCardHeader.vue'

describe('TaskCardHeader', () => {
  it('renders task metadata, feature and notices', () => {
    const wrapper = mount(TaskCardHeader, {
      props: {
        title: 'ACL Diff',
        description: 'Compare ACL entries.',
        stateLabel: 'Done',
        stateTone: 'is-ok',
        feature: 'fs',
        notices: [
          { text: 'admin required', tone: 'warning' },
          { text: 'supports recent link', tone: 'info' },
        ],
      },
    })

    expect(wrapper.text()).toContain('ACL Diff')
    expect(wrapper.text()).toContain('Compare ACL entries.')
    expect(wrapper.text()).toContain('Done')
    expect(wrapper.text()).toContain('fs')
    expect(wrapper.text()).toContain('admin required')
    expect(wrapper.get('.task-card-header__badge').classes()).toContain('is-ok')
    expect(wrapper.findAll('.task-card-header__notice')).toHaveLength(2)
  })

  it('hides optional feature and notices when absent', () => {
    const wrapper = mount(TaskCardHeader, {
      props: {
        title: 'Recent',
        description: 'desc',
        stateLabel: 'Idle',
      },
    })

    expect(wrapper.find('.task-card-header__feature').exists()).toBe(false)
    expect(wrapper.find('.task-card-header__notices').exists()).toBe(false)
  })
})
