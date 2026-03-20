import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import TaskCardActions from './TaskCardActions.vue'

describe('TaskCardActions', () => {
  it('emits trigger when action button is clicked', async () => {
    const wrapper = mount(TaskCardActions, {
      props: {
        tone: 'danger',
        actionLabel: 'Preview',
        disabled: false,
        loading: false,
      },
    })

    await wrapper.get('[data-testid="task-card-action-trigger"]').trigger('click')

    expect(wrapper.emitted('trigger')).toHaveLength(1)
    expect(wrapper.text()).toContain('Preview')
  })

  it('renders hint precedence result and failure hint', () => {
    const wrapper = mount(TaskCardActions, {
      props: {
        actionLabel: 'Run',
        disabled: true,
        loading: false,
        hintText: 'feature unavailable',
        hintTone: 'default',
        failureHint: 'check permissions first',
      },
    })

    expect(wrapper.get('button').attributes('disabled')).toBeDefined()
    expect(wrapper.text()).toContain('feature unavailable')
    expect(wrapper.text()).toContain('check permissions first')
    expect(wrapper.find('.task-card-actions-panel__hint--warn').exists()).toBe(true)
  })
})
