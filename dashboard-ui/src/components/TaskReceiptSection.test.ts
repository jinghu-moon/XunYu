import { mount } from '@vue/test-utils'
import { defineComponent } from 'vue'
import { describe, expect, it } from 'vitest'
import type { GuardedTaskReceipt } from '../types'
import type { WorkspaceTaskDefinition } from '../workspace-tools'
import TaskReceiptSection from './TaskReceiptSection.vue'

const FileGovernanceSummaryStub = defineComponent({
  template: '<div data-testid="receipt-governance-summary">summary</div>',
})

const TaskReceiptComponentStub = defineComponent({
  props: {
    receipt: { type: Object, required: true },
  },
  template: '<div data-testid="receipt-component">{{ receipt.action }}</div>',
})

const task: WorkspaceTaskDefinition = {
  id: 'rm',
  workspace: 'files-security',
  title: 'Remove',
  description: 'desc',
  action: 'rm',
  mode: 'guarded',
  fields: [],
  buildPreviewArgs: () => ['rm', '--dry-run'],
  buildExecuteArgs: () => ['rm', '-y'],
}

const receipt: GuardedTaskReceipt = {
  token: 'token-1',
  workspace: 'files-security',
  action: 'rm',
  target: 'D:/tmp/demo.txt',
  phase: 'execute',
  status: 'succeeded',
  guarded: true,
  dry_run: false,
  summary: 'removed',
  audit_action: 'dashboard.task.execute.rm',
  audited_at: 1700000000,
  process: {
    command_line: 'xun rm -y D:/tmp/demo.txt',
    exit_code: 0,
    success: true,
    stdout: 'removed',
    stderr: '',
    duration_ms: 18,
  },
  details: null,
}

describe('TaskReceiptSection', () => {
  it('renders governance summary and receipt component', () => {
    const wrapper = mount(TaskReceiptSection, {
      props: {
        task,
        form: {},
        receipt,
      },
      global: {
        stubs: {
          FileGovernanceSummary: FileGovernanceSummaryStub,
          TaskReceiptComponent: TaskReceiptComponentStub,
        },
      },
    })

    expect(wrapper.find('[data-testid="receipt-governance-summary"]').exists()).toBe(true)
    expect(wrapper.get('[data-testid="receipt-component"]').text()).toBe('rm')
  })

  it('re-emits receipt link actions', async () => {
    const wrapper = mount(TaskReceiptSection, {
      props: {
        task,
        form: {},
        receipt,
        recentLinkTestId: 'receipt-recent-link',
        auditLinkTestId: 'receipt-audit-link',
      },
      global: {
        stubs: {
          FileGovernanceSummary: FileGovernanceSummaryStub,
          TaskReceiptComponent: TaskReceiptComponentStub,
        },
      },
    })

    await wrapper.get('[data-testid="receipt-recent-link"]').trigger('click')
    await wrapper.get('[data-testid="receipt-audit-link"]').trigger('click')

    expect(wrapper.emitted('focus-recent-tasks')).toHaveLength(1)
    expect(wrapper.emitted('focus-audit')).toHaveLength(1)
  })
})
