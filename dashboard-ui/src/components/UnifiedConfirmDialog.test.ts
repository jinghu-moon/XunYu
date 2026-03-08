import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import UnifiedConfirmDialog from './UnifiedConfirmDialog.vue'

describe('UnifiedConfirmDialog', () => {
  it('renders preview and emits confirm/cancel events', async () => {
    const wrapper = mount(UnifiedConfirmDialog, {
      props: {
        modelValue: true,
        title: '删除文件',
        preview: {
          token: 'token-1',
          workspace: 'files-security',
          action: 'rm',
          target: 'D:/tmp/demo.txt',
          preview_summary: '删除 demo.txt',
          expires_in_secs: 300,
          process: {
            command_line: 'xun rm --dry-run D:/tmp/demo.txt',
            exit_code: 0,
            success: true,
            stdout: 'preview ok',
            stderr: '',
            duration_ms: 12,
          },
        },
      },
    })

    expect(wrapper.text()).toContain('删除文件')
    expect(wrapper.text()).toContain('preview ok')

    const buttons = wrapper.findAll('button')
    await buttons[0].trigger('click')
    expect(wrapper.emitted('update:modelValue')?.[0]).toEqual([false])

    await buttons[1].trigger('click')
    expect(wrapper.emitted('confirm')).toHaveLength(1)
  })
})
