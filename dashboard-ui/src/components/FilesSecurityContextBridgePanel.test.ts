import { mount } from '@vue/test-utils'
import { defineComponent } from 'vue'
import { describe, expect, it } from 'vitest'

import FilesSecurityContextBridgePanel from './FilesSecurityContextBridgePanel.vue'

const BatchGovernancePanelStub = defineComponent({
  props: {
    paths: { type: Array, default: () => [] },
  },
  emits: ['focus-recent-tasks', 'link-panel'],
  template: `
    <div>
      <div data-testid="batch-governance-paths">{{ paths.join('|') }}</div>
      <button
        data-testid="emit-batch-focus"
        @click="$emit('focus-recent-tasks', { status: 'succeeded', dry_run: 'executed', action: 'protect:set', search: 'D:/repo/a.txt' })"
      >
        emit-batch-focus
      </button>
      <button
        data-testid="emit-batch-link"
        @click="$emit('link-panel', { panel: 'audit', request: { result: 'success', action: 'workspace.protect.execute', search: 'D:/repo/a.txt' } })"
      >
        emit-batch-link
      </button>
    </div>
  `,
})

const ButtonStub = defineComponent({
  props: {
    disabled: { type: Boolean, default: false },
  },
  emits: ['click'],
  template: '<button :disabled="disabled" @click="$emit(\'click\')"><slot /></button>',
})

function mountPanel() {
  return mount(FilesSecurityContextBridgePanel, {
    props: {
      currentDirectory: 'D:/repo',
      selectedPath: 'D:/repo/src/a.rs',
      aclReferencePath: 'D:/repo/src/base.rs',
      batchPaths: ['D:/repo/src/a.rs', 'D:/repo/src/b.rs', 'D:/repo/src/c.rs'],
      batchPreview: ['D:/repo/src/a.rs', 'D:/repo/src/b.rs'],
      batchOverflow: 1,
      syncMessage: '\u5df2\u540c\u6b65\u6587\u4ef6\u4e0a\u4e0b\u6587\u3002',
      hasBatch: true,
      hasDirectory: true,
      hasSelection: true,
      canSyncAclComparison: true,
      canQueueSelection: true,
      capabilities: null,
    },
    global: {
      stubs: {
        BatchGovernancePanel: BatchGovernancePanelStub,
        Button: ButtonStub,
      },
    },
  })
}

async function clickButton(wrapper: ReturnType<typeof mountPanel>, label: string) {
  const button = wrapper.findAll('button').find((item) => item.text().includes(label))
  expect(button, `missing button: ${label}`).toBeTruthy()
  await button!.trigger('click')
}

describe('FilesSecurityContextBridgePanel', () => {
  it('renders context values and emits action intents', async () => {
    const wrapper = mountPanel()

    expect(wrapper.text()).toContain('D:/repo')
    expect(wrapper.text()).toContain('D:/repo/src/a.rs')
    expect(wrapper.text()).toContain('D:/repo/src/base.rs')
    expect(wrapper.text()).toContain('\u8fd8\u6709 1 \u9879\u672a\u5c55\u5f00\u3002')
    expect(wrapper.get('[data-testid="batch-governance-paths"]').text()).toBe(
      'D:/repo/src/a.rs|D:/repo/src/b.rs|D:/repo/src/c.rs',
    )

    await clickButton(wrapper, '\u540c\u6b65\u76ee\u5f55\u4efb\u52a1')
    await clickButton(wrapper, '\u540c\u6b65\u6587\u4ef6\u4efb\u52a1')
    await clickButton(wrapper, '\u540c\u6b65\u5168\u90e8')
    await clickButton(wrapper, '\u8bbe\u4e3a ACL \u53c2\u8003')
    await clickButton(wrapper, '\u540c\u6b65 ACL \u5bf9\u6bd4')
    await clickButton(wrapper, '\u52a0\u5165\u6279\u91cf\u961f\u5217')
    await clickButton(wrapper, '\u6e05\u7a7a')
    await clickButton(wrapper, '\u6279\u91cf\u586b\u5145\u67e5\u627e')
    await clickButton(wrapper, '\u6279\u91cf\u586b\u5145\u5907\u4efd')
    await wrapper.get('.files-security-context__remove-btn').trigger('click')

    expect(wrapper.emitted('sync-directory-context')).toHaveLength(1)
    expect(wrapper.emitted('sync-selection-context')).toHaveLength(1)
    expect(wrapper.emitted('sync-all-context')).toHaveLength(1)
    expect(wrapper.emitted('set-acl-reference')).toHaveLength(1)
    expect(wrapper.emitted('sync-acl-comparison-context')).toHaveLength(1)
    expect(wrapper.emitted('add-selection-to-batch')).toHaveLength(1)
    expect(wrapper.emitted('clear-batch')).toHaveLength(1)
    expect(wrapper.emitted('sync-batch-to-find')).toHaveLength(1)
    expect(wrapper.emitted('sync-batch-to-backup')).toHaveLength(1)
    expect(wrapper.emitted('remove-batch-path')).toEqual([['D:/repo/src/a.rs']])
  })

  it('forwards batch governance panel events', async () => {
    const wrapper = mountPanel()

    await wrapper.get('[data-testid="emit-batch-focus"]').trigger('click')
    await wrapper.get('[data-testid="emit-batch-link"]').trigger('click')

    expect(wrapper.emitted('focus-recent-tasks')).toEqual([[
      {
        status: 'succeeded',
        dry_run: 'executed',
        action: 'protect:set',
        search: 'D:/repo/a.txt',
      },
    ]])
    expect(wrapper.emitted('link-panel')).toEqual([[
      {
        panel: 'audit',
        request: {
          result: 'success',
          action: 'workspace.protect.execute',
          search: 'D:/repo/a.txt',
        },
      },
    ]])
  })
})
