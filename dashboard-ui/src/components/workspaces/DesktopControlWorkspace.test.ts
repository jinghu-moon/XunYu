import { mount } from '@vue/test-utils'
import { describe, expect, it, vi } from 'vitest'
import DesktopControlWorkspace from './DesktopControlWorkspace.vue'

vi.mock('../RecentTasksPanel.vue', () => ({
  default: {
    name: 'RecentTasksPanel',
    template: '<div data-testid="recent-tasks-panel" />',
  },
}))

vi.mock('../RecipePanel.vue', () => ({
  default: {
    name: 'RecipePanel',
    template: '<div data-testid="recipe-panel" />',
  },
}))

vi.mock('../TaskToolbox.vue', () => ({
  default: {
    name: 'TaskToolbox',
    props: ['tasks'],
    template: '<div data-testid="task-toolbox" />',
  },
}))

vi.mock('../WorkspaceFrame.vue', () => ({
  default: {
    name: 'WorkspaceFrame',
    template: '<section><slot /></section>',
  },
}))

describe('DesktopControlWorkspace', () => {
  it('renders core panels', () => {
    const wrapper = mount(DesktopControlWorkspace)
    expect(wrapper.find('[data-testid="recent-tasks-panel"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="recipe-panel"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="task-toolbox"]').exists()).toBe(true)
  })
})

