import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import UnifiedConfirmDialog from './UnifiedConfirmDialog.vue'

describe('UnifiedConfirmDialog', () => {
  it('renders preview, extra slot and emits confirm/cancel events', async () => {
    const wrapper = mount(UnifiedConfirmDialog, {
      attachTo: document.body,
      props: {
        modelValue: true,
        title: '删除文件',
        preview: {
          token: 'token-1',
          workspace: 'files-security',
          action: 'rm',
          target: 'D:/tmp/demo.txt',
          phase: 'preview',
          status: 'previewed',
          guarded: true,
          dry_run: true,
          ready_to_execute: true,
          summary: '删除 demo.txt',
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
      slots: {
        'preview-extra': '<div data-testid="slot-extra">预演补充摘要</div>',
      },
    })

    expect(wrapper.text()).toContain('删除文件')
    expect(wrapper.text()).toContain('preview ok')
    expect(wrapper.text()).toContain('可执行')
    expect(wrapper.text()).toContain('已启用')
    expect(wrapper.get('[data-testid="confirm-dialog-extra"]').text()).toContain('预演补充摘要')

    const buttons = [...document.body.querySelectorAll('button')]
    buttons[0]?.dispatchEvent(new MouseEvent('click'))
    expect(wrapper.emitted('update:modelValue')?.[0]).toEqual([false])

    buttons[1]?.dispatchEvent(new MouseEvent('click'))
    expect(wrapper.emitted('confirm')).toHaveLength(1)
  })
})
