import { mount, VueWrapper } from '@vue/test-utils'
import { defineComponent } from 'vue'
import { describe, expect, it } from 'vitest'
import FilesSecurityWorkspace from './FilesSecurityWorkspace.vue'

const DiffPanelStub = defineComponent({
  emits: ['directoryChange', 'selectionChange'],
  template: `
    <div>
      <button data-testid="emit-directory" @click="$emit('directoryChange', 'D:/repo')">emit-directory</button>
      <button data-testid="emit-selection-a" @click="$emit('selectionChange', 'D:/repo/src/a.rs')">emit-selection-a</button>
      <button data-testid="emit-selection-b" @click="$emit('selectionChange', 'D:/repo/src/b.rs')">emit-selection-b</button>
    </div>
  `,
})

const RedirectPanelStub = defineComponent({
  template: '<div data-testid="redirect-stub">redirect</div>',
})

const TaskToolboxStub = defineComponent({
  props: {
    title: { type: String, default: '' },
    taskPresets: { type: Object, default: null },
    presetVersion: { type: Number, default: 0 },
  },
  template: `
    <div class="task-toolbox-stub" :data-title="title" :data-version="presetVersion">
      {{ JSON.stringify(taskPresets ?? {}) }}
    </div>
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
      <div data-testid="governance-acl-reference">{{ aclReferencePath }}</div>
    </div>
  `,
})

const BatchGovernancePanelStub = defineComponent({
  props: {
    paths: { type: Array, default: () => [] },
  },
  emits: ['focus-recent-tasks', 'link-panel'],
  template: `
    <div>
      <div data-testid="batch-paths">{{ paths.join("|") }}</div>
      <button
        data-testid="emit-batch-recent-focus"
        @click="$emit('focus-recent-tasks', { status: 'succeeded', dry_run: 'executed', action: 'protect:set', search: 'D:/repo/src/a.rs' })"
      >
        emit-batch-recent-focus
      </button>
      <button
        data-testid="emit-batch-audit-link"
        @click="$emit('link-panel', { panel: 'audit', request: { action: 'workspace.protect.execute', result: 'success', search: 'D:/repo/src/a.rs' } })"
      >
        emit-batch-audit-link
      </button>
      <button
        data-testid="emit-batch-diagnostics-link"
        @click="$emit('link-panel', { panel: 'diagnostics-center', request: { panel: 'governance', governance_family: 'protect', governance_status: 'succeeded', target: 'D:/repo/src/a.rs', audit_action: 'workspace.protect.execute', audit_result: 'success', audit_timestamp: 1700000000 } })"
      >
        emit-batch-diagnostics-link
      </button>
    </div>
  `,
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

const WorkspaceFrameStub = defineComponent({
  template: '<section><slot name="summary" /><slot /></section>',
})

const ButtonStub = defineComponent({
  props: {
    disabled: { type: Boolean, default: false },
  },
  emits: ['click'],
  template: '<button :disabled="disabled" @click="$emit(\'click\')"><slot /></button>',
})

function mountWorkspace() {
  return mount(FilesSecurityWorkspace, {
    global: {
      stubs: {
        DiffPanel: DiffPanelStub,
        RedirectPanel: RedirectPanelStub,
        TaskToolbox: TaskToolboxStub,
        FileGovernancePanel: FileGovernancePanelStub,
        BatchGovernancePanel: BatchGovernancePanelStub,
        RecentTasksPanel: RecentTasksPanelStub,
        RecipePanel: RecipePanelStub,
        WorkspaceFrame: WorkspaceFrameStub,
        Button: ButtonStub,
      },
    },
  })
}

async function clickButton(wrapper: VueWrapper<any>, label: string) {
  const button = wrapper.findAll('button').find((item) => item.text().includes(label))
  expect(button, `missing button: ${label}`).toBeTruthy()
  await button!.trigger('click')
}

describe('FilesSecurityWorkspace', () => {
  it('syncs directory and selection context into task presets', async () => {
    const wrapper = mountWorkspace()

    await wrapper.get('[data-testid="emit-directory"]').trigger('click')
    await wrapper.get('[data-testid="emit-selection-a"]').trigger('click')
    await clickButton(wrapper, '同步全部')

    const toolboxText = wrapper.find('.task-toolbox-stub').text()
    expect(toolboxText).toContain('"tree":{"path":"D:/repo"}')
    expect(toolboxText).toContain('"rm":{"path":"D:/repo/src/a.rs"}')
    expect(toolboxText).toContain('"mv":{"src":"D:/repo/src/a.rs"}')
    expect(wrapper.get('[data-testid="governance-path"]').text()).toBe('D:/repo/src/a.rs')
    expect(wrapper.get('[data-testid="recent-workspace"]').text()).toBe('files-security')
    expect(wrapper.get('[data-testid="recipe-category"]').text()).toBe('files-security')
  })

  it('sets acl reference and syncs acl compare/copy presets', async () => {
    const wrapper = mountWorkspace()

    await wrapper.get('[data-testid="emit-selection-a"]').trigger('click')
    await clickButton(wrapper, '设为 ACL 参考')
    expect(wrapper.get('[data-testid="governance-acl-reference"]').text()).toBe('D:/repo/src/a.rs')

    await wrapper.get('[data-testid="emit-selection-b"]').trigger('click')
    await clickButton(wrapper, '同步 ACL 对比')

    const toolboxText = wrapper.find('.task-toolbox-stub').text()
    expect(toolboxText).toContain('"acl-diff":{"path":"D:/repo/src/b.rs","reference":"D:/repo/src/a.rs"}')
    expect(toolboxText).toContain('"acl-copy":{"path":"D:/repo/src/b.rs","reference":"D:/repo/src/a.rs"}')
  })

  it('bridges batch governance focus into recent tasks panel', async () => {
    const wrapper = mountWorkspace()

    await wrapper.get('[data-testid="emit-batch-recent-focus"]').trigger('click')

    const recentFocus = wrapper.get('[data-testid="recent-focus"]').text()
    expect(recentFocus).toContain('"status":"succeeded"')
    expect(recentFocus).toContain('"dry_run":"executed"')
    expect(recentFocus).toContain('"action":"protect:set"')
    expect(recentFocus).toContain('"search":"D:/repo/src/a.rs"')
  })

  it('re-emits batch governance workspace links upward', async () => {
    const wrapper = mountWorkspace()

    await wrapper.get('[data-testid="emit-batch-audit-link"]').trigger('click')

    expect(wrapper.emitted('link-panel')).toEqual([[{
      panel: 'audit',
      request: {
        action: 'workspace.protect.execute',
        result: 'success',
        search: 'D:/repo/src/a.rs',
      },
    }]])
  })

  it('re-emits batch governance diagnostics links upward', async () => {
    const wrapper = mountWorkspace()

    await wrapper.get('[data-testid="emit-batch-diagnostics-link"]').trigger('click')

    expect(wrapper.emitted('link-panel')).toEqual([[{
      panel: 'diagnostics-center',
      request: {
        panel: 'governance',
        governance_family: 'protect',
        governance_status: 'succeeded',
        target: 'D:/repo/src/a.rs',
        audit_action: 'workspace.protect.execute',
        audit_result: 'success',
        audit_timestamp: 1700000000,
      },
    }]])
  })

  it('accumulates batch targets and projects them into find / backup presets', async () => {
    const wrapper = mountWorkspace()

    await wrapper.get('[data-testid="emit-directory"]').trigger('click')
    await wrapper.get('[data-testid="emit-selection-a"]').trigger('click')
    await clickButton(wrapper, '加入批量队列')
    await wrapper.get('[data-testid="emit-selection-b"]').trigger('click')
    await clickButton(wrapper, '加入批量队列')
    await clickButton(wrapper, '批量填充查找')

    let toolboxText = wrapper.find('.task-toolbox-stub').text()
    expect(toolboxText).toContain('D:/repo/src/a.rs\\nD:/repo/src/b.rs')
    expect(wrapper.get('[data-testid="batch-paths"]').text()).toBe('D:/repo/src/a.rs|D:/repo/src/b.rs')

    await clickButton(wrapper, '批量填充备份')
    toolboxText = wrapper.find('.task-toolbox-stub').text()
    expect(toolboxText).toContain('"bak-create":{"dir":"D:/repo","include":"D:/repo/src/a.rs\\nD:/repo/src/b.rs"}')
  })
})
