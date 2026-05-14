import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import WorkspaceNav from './WorkspaceNav.vue'

describe('WorkspaceNav', () => {
  const workspaces = [
    { id: 'overview', label: 'Overview' },
    { id: 'bookmarks', label: 'Bookmarks' },
    { id: 'env', label: 'Environment' },
    { id: 'config', label: 'Config' },
  ]

  it('renders workspace tabs', () => {
    const wrapper = mount(WorkspaceNav, { props: { workspaces, active: 'overview' } })

    const tabs = wrapper.findAll('[data-testid="workspace-tab"]')
    expect(tabs).toHaveLength(4)
    expect(tabs[0].text()).toBe('Overview')
    expect(tabs[1].text()).toBe('Bookmarks')
  })

  it('highlights active tab', () => {
    const wrapper = mount(WorkspaceNav, { props: { workspaces, active: 'bookmarks' } })

    const tabs = wrapper.findAll('[data-testid="workspace-tab"]')
    expect(tabs[1].classes()).toContain('active')
    expect(tabs[0].classes()).not.toContain('active')
  })

  it('emits change on tab click', async () => {
    const wrapper = mount(WorkspaceNav, { props: { workspaces, active: 'overview' } })

    await wrapper.findAll('[data-testid="workspace-tab"]')[2].trigger('click')
    expect(wrapper.emitted('change')![0]).toEqual(['env'])
  })

  it('shows overflow dropdown for many tabs', () => {
    const manyWorkspaces = Array.from({ length: 20 }, (_, i) => ({
      id: `ws-${i}`,
      label: `Workspace ${i}`,
    }))
    const wrapper = mount(WorkspaceNav, {
      props: { workspaces: manyWorkspaces, active: 'ws-0', maxVisible: 5 },
    })

    const visibleTabs = wrapper.findAll('[data-testid="workspace-tab"]')
    expect(visibleTabs.length).toBeLessThanOrEqual(5)
    expect(wrapper.find('[data-testid="overflow-menu"]').exists()).toBe(true)
  })
})
