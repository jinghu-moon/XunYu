import { mount } from '@vue/test-utils'
import { defineComponent } from 'vue'
import { describe, expect, it } from 'vitest'
import type { WorkspaceTaskDefinition } from '../workspace-tools'
import TaskConfirmDialog from './TaskConfirmDialog.vue'

const UnifiedConfirmDialogStub = defineComponent({
  props: {
    modelValue: { type: Boolean, default: false },
    title: { type: String, default: '' },
    preview: { type: Object, default: null },
    busy: { type: Boolean, default: false },
    confirmDisabled: { type: Boolean, default: false },
  },
  emits: ['update:modelValue', 'confirm'],
  template: `
    <section>
      <div data-testid="dialog-title">{{ title }}</div>
      <div data-testid="dialog-preview-target">{{ preview?.target ?? '-' }}</div>
      <div data-testid="dialog-busy">{{ busy ? 'busy' : 'idle' }}</div>
      <div data-testid="dialog-disabled">{{ confirmDisabled ? 'disabled' : 'enabled' }}</div>
      <div data-testid="dialog-extra"><slot name="preview-extra" /></div>
      <button data-testid="emit-close" @click="$emit('update:modelValue', false)">close</button>
      <button data-testid="emit-confirm" @click="$emit('confirm')">confirm</button>
    </section>
  `,
})

const FileGovernanceSummaryStub = defineComponent({
  props: {
    phase: { type: String, default: '' },
  },
  template: '<div data-testid="governance-summary">{{ phase }}</div>',
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

describe('TaskConfirmDialog', () => {
  it('assembles preview summary into confirm dialog slot', () => {
    const wrapper = mount(TaskConfirmDialog, {
      props: {
        modelValue: true,
        title: 'Confirm remove',
        task,
        form: {},
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
          summary: 'remove demo',
          preview_summary: 'remove demo',
          expires_in_secs: 300,
          process: {
            command_line: 'xun rm --dry-run D:/tmp/demo.txt',
            exit_code: 0,
            success: true,
            stdout: 'preview ok',
            stderr: '',
            duration_ms: 10,
          },
          details: null,
        },
        busy: true,
        confirmDisabled: false,
      },
      global: {
        stubs: {
          UnifiedConfirmDialog: UnifiedConfirmDialogStub,
          FileGovernanceSummary: FileGovernanceSummaryStub,
        },
      },
    })

    expect(wrapper.get('[data-testid="dialog-title"]').text()).toBe('Confirm remove')
    expect(wrapper.get('[data-testid="dialog-preview-target"]').text()).toBe('D:/tmp/demo.txt')
    expect(wrapper.get('[data-testid="dialog-busy"]').text()).toBe('busy')
    expect(wrapper.get('[data-testid="dialog-disabled"]').text()).toBe('enabled')
    expect(wrapper.get('[data-testid="dialog-extra"]').text()).toContain('preview')
  })

  it('re-emits close and confirm events', async () => {
    const wrapper = mount(TaskConfirmDialog, {
      props: {
        modelValue: true,
        title: 'Confirm remove',
        task,
        form: {},
        preview: null,
      },
      global: {
        stubs: {
          UnifiedConfirmDialog: UnifiedConfirmDialogStub,
          FileGovernanceSummary: FileGovernanceSummaryStub,
        },
      },
    })

    await wrapper.get('[data-testid="emit-close"]').trigger('click')
    await wrapper.get('[data-testid="emit-confirm"]').trigger('click')

    expect(wrapper.emitted('update:modelValue')).toEqual([[false]])
    expect(wrapper.emitted('confirm')).toHaveLength(1)
  })
})
