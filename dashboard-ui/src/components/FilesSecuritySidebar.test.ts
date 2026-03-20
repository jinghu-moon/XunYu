import { mount } from '@vue/test-utils'
import { defineComponent } from 'vue'
import { describe, expect, it, vi } from 'vitest'

import FilesSecuritySidebar from './FilesSecuritySidebar.vue'

const FilesSecurityContextBridgePanelStub = defineComponent({
  props: {
    currentDirectory: { type: String, default: '' },
    selectedPath: { type: String, default: '' },
    aclReferencePath: { type: String, default: '' },
    batchPaths: { type: Array, default: () => [] },
    batchPreview: { type: Array, default: () => [] },
    batchOverflow: { type: Number, default: 0 },
    syncMessage: { type: String, default: '' },
    hasBatch: { type: Boolean, default: false },
    hasDirectory: { type: Boolean, default: false },
    hasSelection: { type: Boolean, default: false },
    canSyncAclComparison: { type: Boolean, default: false },
    canQueueSelection: { type: Boolean, default: false },
  },
  emits: [
    'sync-directory-context',
    'sync-selection-context',
    'sync-all-context',
    'set-acl-reference',
    'sync-acl-comparison-context',
    'add-selection-to-batch',
    'clear-batch',
    'sync-batch-to-find',
    'sync-batch-to-backup',
    'remove-batch-path',
    'focus-recent-tasks',
    'link-panel',
  ],
  template: `
    <section>
      <div data-testid="bridge-directory">{{ currentDirectory }}</div>
      <div data-testid="bridge-selection">{{ selectedPath }}</div>
      <div data-testid="bridge-acl-reference">{{ aclReferencePath }}</div>
      <div data-testid="bridge-batch">{{ batchPaths.join('|') }}</div>
      <button data-testid="emit-sync-all" @click="$emit('sync-all-context')">sync-all</button>
      <button
        data-testid="emit-focus-recent"
        @click="$emit('focus-recent-tasks', { status: 'succeeded', dry_run: 'executed', action: 'protect:set', search: 'D:/repo/a.txt' })"
      >
        focus-recent
      </button>
      <button
        data-testid="emit-link-panel"
        @click="$emit('link-panel', { panel: 'audit', request: { result: 'success', action: 'workspace.files.test', search: 'D:/repo/a.txt' } })"
      >
        link-panel
      </button>
    </section>
  `,
})

const FileGovernancePanelStub = defineComponent({
  props: {
    path: { type: String, default: '' },
    aclReferencePath: { type: String, default: '' },
  },
  template: `
    <div>
      <div data-testid="governance-path">{{ path }}</div>
      <div data-testid="governance-reference">{{ aclReferencePath }}</div>
    </div>
  `,
})

const FileVaultFoundationPanelStub = defineComponent({
  props: {
    path: { type: String, default: '' },
  },
  template: '<div data-testid="vault-path">{{ path }}</div>',
})

const RecentTasksPanelStub = defineComponent({
  props: {
    workspace: { type: String, default: '' },
    focusRequest: { type: Object, default: null },
  },
  template: `
    <div>
      <div data-testid="recent-workspace">{{ workspace }}</div>
      <div data-testid="recent-focus">{{ JSON.stringify(focusRequest ?? null) }}</div>
    </div>
  `,
})

const RecipePanelStub = defineComponent({
  props: {
    category: { type: String, default: '' },
  },
  template: '<div data-testid="recipe-category">{{ category }}</div>',
})

function mountSidebar() {
  const recentTasksAnchorRef = vi.fn()
  const wrapper = mount(FilesSecuritySidebar, {
    props: {
      currentDirectory: 'D:/repo',
      selectedPath: 'D:/repo/src/main.rs',
      aclReferencePath: 'D:/repo/template.rs',
      batchPaths: ['D:/repo/src/main.rs', 'D:/repo/src/lib.rs'],
      batchPreview: ['D:/repo/src/main.rs'],
      batchOverflow: 1,
      syncMessage: 'synced',
      hasBatch: true,
      hasDirectory: true,
      hasSelection: true,
      canSyncAclComparison: true,
      canQueueSelection: false,
      recentTasksFocus: {
        key: 2,
        status: 'succeeded',
        dry_run: 'executed',
        action: 'protect:set',
        search: 'D:/repo/src/main.rs',
      },
      recentTasksAnchorRef,
      capabilities: null,
    },
    global: {
      stubs: {
        FilesSecurityContextBridgePanel: FilesSecurityContextBridgePanelStub,
        FileGovernancePanel: FileGovernancePanelStub,
        FileVaultFoundationPanel: FileVaultFoundationPanelStub,
        RecentTasksPanel: RecentTasksPanelStub,
        RecipePanel: RecipePanelStub,
      },
    },
  })

  return {
    wrapper,
    recentTasksAnchorRef,
  }
}

describe('FilesSecuritySidebar', () => {
  it('renders support panels with forwarded props', () => {
    const { wrapper, recentTasksAnchorRef } = mountSidebar()

    expect(wrapper.get('[data-testid="bridge-directory"]').text()).toBe('D:/repo')
    expect(wrapper.get('[data-testid="bridge-batch"]').text()).toBe('D:/repo/src/main.rs|D:/repo/src/lib.rs')
    expect(wrapper.get('[data-testid="governance-path"]').text()).toBe('D:/repo/src/main.rs')
    expect(wrapper.get('[data-testid="governance-reference"]').text()).toBe('D:/repo/template.rs')
    expect(wrapper.get('[data-testid="vault-path"]').text()).toBe('D:/repo/src/main.rs')
    expect(wrapper.get('[data-testid="recent-workspace"]').text()).toBe('files-security')
    expect(wrapper.get('[data-testid="recent-focus"]').text()).toContain('protect:set')
    expect(wrapper.get('[data-testid="recipe-category"]').text()).toBe('files-security')
    expect(recentTasksAnchorRef).toHaveBeenCalled()
  })

  it('re-emits bridge events upward', async () => {
    const { wrapper } = mountSidebar()

    await wrapper.get('[data-testid="emit-sync-all"]').trigger('click')
    await wrapper.get('[data-testid="emit-focus-recent"]').trigger('click')
    await wrapper.get('[data-testid="emit-link-panel"]').trigger('click')

    expect(wrapper.emitted('sync-all-context')).toHaveLength(1)
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
          action: 'workspace.files.test',
          search: 'D:/repo/a.txt',
        },
      },
    ]])
  })
})
