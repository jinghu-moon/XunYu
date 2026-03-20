import { mount } from '@vue/test-utils'
import { defineComponent } from 'vue'
import { describe, expect, it } from 'vitest'
import type { WorkspaceTaskDefinition } from '../workspace-tools'
import TaskProcessResultPanel from './TaskProcessResultPanel.vue'

const FileGovernanceSummaryStub = defineComponent({
  props: {
    phase: { type: String, default: '' },
  },
  template: '<div data-testid="governance-summary">{{ phase }}</div>',
})

const task: WorkspaceTaskDefinition = {
  id: 'recent',
  workspace: 'paths-context',
  title: 'Recent',
  description: 'desc',
  action: 'recent',
  mode: 'run',
  fields: [],
  buildRunArgs: () => ['recent'],
}

describe('TaskProcessResultPanel', () => {
  it('renders preview metadata and output without action links', () => {
    const wrapper = mount(TaskProcessResultPanel, {
      props: {
        task,
        form: {},
        phase: 'preview',
        process: {
          command_line: 'xun recent --dry-run',
          exit_code: 0,
          success: true,
          stdout: 'preview ok',
          stderr: '',
          duration_ms: 8,
        },
        details: null,
        badgeText: 'preview-ok',
        badgeTone: 'is-ok',
        metaText: 'preview summary',
        showLinks: false,
      },
      global: {
        stubs: {
          FileGovernanceSummary: FileGovernanceSummaryStub,
        },
      },
    })

    expect(wrapper.text()).toContain('preview-ok')
    expect(wrapper.text()).toContain('preview summary')
    expect(wrapper.text()).toContain('preview ok')
    expect(wrapper.get('[data-testid="governance-summary"]').text()).toBe('preview')
    expect(wrapper.find('[data-testid="task-process-link-recent"]').exists()).toBe(false)
  })

  it('renders execute links and re-emits link actions', async () => {
    const wrapper = mount(TaskProcessResultPanel, {
      props: {
        task,
        form: {},
        phase: 'execute',
        process: {
          command_line: 'xun recent',
          exit_code: 1,
          success: false,
          stdout: '',
          stderr: 'failed',
          duration_ms: 12,
        },
        details: null,
        badgeText: 'failed',
        badgeTone: 'is-error',
        metaText: '12 ms',
        showLinks: true,
        recentLinkTestId: 'recent-link',
        auditLinkTestId: 'audit-link',
      },
      global: {
        stubs: {
          FileGovernanceSummary: FileGovernanceSummaryStub,
        },
      },
    })

    await wrapper.get('[data-testid="recent-link"]').trigger('click')
    await wrapper.get('[data-testid="audit-link"]').trigger('click')

    expect(wrapper.emitted('focus-recent-tasks')).toHaveLength(1)
    expect(wrapper.emitted('focus-audit')).toHaveLength(1)
    expect(wrapper.text()).toContain('failed')
    expect(wrapper.text()).toContain('12 ms')
    expect(wrapper.text()).toContain('xun recent')
  })
})
