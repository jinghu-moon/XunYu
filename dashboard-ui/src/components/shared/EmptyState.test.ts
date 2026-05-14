import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import EmptyState from './EmptyState.vue'

describe('EmptyState', () => {
  it('renders default message', () => {
    const wrapper = mount(EmptyState)
    expect(wrapper.find('[data-testid="empty-state"]').exists()).toBe(true)
    expect(wrapper.text()).toContain('No data')
  })

  it('renders custom message', () => {
    const wrapper = mount(EmptyState, { props: { message: 'No bookmarks found' } })
    expect(wrapper.text()).toContain('No bookmarks found')
  })

  it('renders icon slot', () => {
    const wrapper = mount(EmptyState, {
      slots: { icon: '<span class="custom-icon">📭</span>' },
    })
    expect(wrapper.find('.custom-icon').exists()).toBe(true)
  })

  it('renders action slot', () => {
    const wrapper = mount(EmptyState, {
      slots: { action: '<button>Add item</button>' },
    })
    expect(wrapper.find('button').text()).toBe('Add item')
  })
})
