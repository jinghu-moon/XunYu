import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import TaskReceiptComponent from './TaskReceiptComponent.vue'

describe('TaskReceiptComponent', () => {
  it('renders receipt metadata and command output', () => {
    const wrapper = mount(TaskReceiptComponent, {
      props: {
        receipt: {
          token: 'token-1',
          workspace: 'files-security',
          action: 'rm',
          target: 'D:/tmp/demo.txt',
          audit_action: 'workspace.rm.execute',
          audited_at: 1700000000,
          process: {
            command_line: 'xun rm D:/tmp/demo.txt',
            exit_code: 0,
            success: true,
            stdout: 'deleted',
            stderr: '',
            duration_ms: 30,
          },
        },
      },
    })

    expect(wrapper.text()).toContain('执行回执')
    expect(wrapper.text()).toContain('workspace.rm.execute')
    expect(wrapper.text()).toContain('D:/tmp/demo.txt')
    expect(wrapper.text()).toContain('deleted')
  })
})
