import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import CollapsiblePanel from './CollapsiblePanel.vue'

describe('CollapsiblePanel', () => {
  it('renders title', () => {
    const wrapper = mount(CollapsiblePanel, {
      props: { title: 'Test Panel' },
      slots: { default: '<p>content</p>' },
    })

    expect(wrapper.find('[data-testid="panel-toggle"]').text()).toContain('Test Panel')
  })

  it('shows content by default', () => {
    const wrapper = mount(CollapsiblePanel, {
      props: { title: 'Panel' },
      slots: { default: '<p data-testid="inner">content</p>' },
    })

    expect(wrapper.find('[data-testid="panel-body"]').isVisible()).toBe(true)
    expect(wrapper.find('[data-testid="inner"]').exists()).toBe(true)
  })

  it('collapses on toggle click', async () => {
    const wrapper = mount(CollapsiblePanel, {
      props: { title: 'Panel' },
      slots: { default: '<p>content</p>' },
    })

    await wrapper.find('[data-testid="panel-toggle"]').trigger('click')

    expect(wrapper.find('[data-testid="panel-body"]').isVisible()).toBe(false)
    expect(wrapper.find('[data-testid="panel-toggle"]').attributes('aria-expanded')).toBe('false')
  })

  it('expands on second toggle click', async () => {
    const wrapper = mount(CollapsiblePanel, {
      props: { title: 'Panel' },
      slots: { default: '<p>content</p>' },
    })

    await wrapper.find('[data-testid="panel-toggle"]').trigger('click')
    await wrapper.find('[data-testid="panel-toggle"]').trigger('click')

    expect(wrapper.find('[data-testid="panel-body"]').isVisible()).toBe(true)
    expect(wrapper.find('[data-testid="panel-toggle"]').attributes('aria-expanded')).toBe('true')
  })

  it('starts collapsed when collapsed prop is true', () => {
    const wrapper = mount(CollapsiblePanel, {
      props: { title: 'Panel', collapsed: true },
      slots: { default: '<p>content</p>' },
    })

    expect(wrapper.find('[data-testid="panel-body"]').isVisible()).toBe(false)
  })
})
