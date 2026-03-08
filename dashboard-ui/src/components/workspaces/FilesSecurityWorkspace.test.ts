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
  },
  template: '<div data-testid="governance-path">{{ path }}</div>',
})

const BatchGovernancePanelStub = defineComponent({
  props: {
    paths: { type: Array, default: () => [] },
  },
  template: '<div data-testid="batch-paths">{{ paths.join("|") }}</div>',
})

const RecentTasksPanelStub = defineComponent({
  props: {
    workspace: { type: String, default: '' },
  },
  template: '<div data-testid="recent-workspace">{{ workspace }}</div>',
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
