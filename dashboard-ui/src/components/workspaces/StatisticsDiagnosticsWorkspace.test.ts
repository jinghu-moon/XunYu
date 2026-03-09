import { flushPromises, mount } from '@vue/test-utils'
import { defineComponent } from 'vue'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import StatisticsDiagnosticsWorkspace from './StatisticsDiagnosticsWorkspace.vue'

const DiagnosticsCenterPanelStub = defineComponent({
  props: {
    focusRequest: { type: Object, default: null },
  },
  emits: ['link-panel'],
  template: `
    <div>
      <div data-testid="diagnostics-focus">{{ JSON.stringify(focusRequest) }}</div>
      <button
        data-testid="emit-recent-link"
        type="button"
        @click="$emit('link-panel', { panel: 'recent-tasks', request: { selected_task_id: 'task-2', status: 'failed', dry_run: 'executed' } })"
      >
        recent
      </button>
      <button
        data-testid="emit-audit-link"
        type="button"
        @click="$emit('link-panel', { panel: 'audit', request: { search: 'D:/repo/demo.txt', action: 'dashboard.task.execute.acl:owner', result: 'success' } })"
      >
        audit
      </button>
    </div>
  `,
})

const RecipePanelStub = defineComponent({
  emits: ['link-panel'],
  template: `
    <button
      data-testid="emit-recipe-link"
      type="button"
      @click="$emit('link-panel', { panel: 'audit', request: { search: 'D:/tmp/demo.log', action: 'dashboard.task.execute.rm', result: 'success' } })"
    >
      recipe
    </button>
  `,
})

const TaskToolboxStub = defineComponent({
  emits: ['link-panel'],
  template: `
    <button
      data-testid="emit-toolbox-link"
      type="button"
      @click="$emit('link-panel', { panel: 'recent-tasks', request: { status: 'succeeded', dry_run: 'executed', action: 'cstat', search: 'D:/logs/demo.log' } })"
    >
      toolbox
    </button>
  `,
})

const RecentTasksPanelStub = defineComponent({
  props: {
    focusRequest: { type: Object, default: null },
  },
  emits: ['link-panel'],
  template: `
    <div>
      <div data-testid="recent-focus">{{ JSON.stringify(focusRequest) }}</div>
      <button
        data-testid="emit-diagnostics-from-recent"
        type="button"
        @click="$emit('link-panel', { panel: 'diagnostics-center', request: { panel: 'failed', task_id: 'task-2', target: 'D:/repo/demo.txt' } })"
      >
        diagnostics-from-recent
      </button>
    </div>
  `,
})

const AuditPanelStub = defineComponent({
  props: {
    focusRequest: { type: Object, default: null },
  },
  emits: ['link-panel'],
  template: `
    <div>
      <div data-testid="audit-focus">{{ JSON.stringify(focusRequest) }}</div>
      <button
        data-testid="emit-diagnostics-from-audit"
        type="button"
        @click="$emit('link-panel', { panel: 'diagnostics-center', request: { panel: 'governance', governance_family: 'acl', governance_status: 'failed', target: 'D:/repo/demo.txt', audit_action: 'dashboard.task.execute.acl:owner', audit_timestamp: 1700000001, audit_result: 'failed' } })"
      >
        diagnostics-from-audit
      </button>
    </div>
  `,
})

const WorkspaceFrameStub = defineComponent({
  template: '<section><slot /></section>',
})

describe('StatisticsDiagnosticsWorkspace', () => {
  beforeEach(() => {
    HTMLElement.prototype.scrollIntoView = vi.fn()
  })

  it('accepts external link payloads from app-level routing', async () => {
    const wrapper = mount(StatisticsDiagnosticsWorkspace, {
      props: {
        externalLink: {
          key: 1,
          payload: {
            panel: 'audit',
            request: { action: 'video:compress', result: 'success', search: 'demo.small.mp4' },
          },
        },
      },
      global: {
        stubs: {
          DiagnosticsCenterPanel: DiagnosticsCenterPanelStub,
          RecentTasksPanel: RecentTasksPanelStub,
          AuditPanel: AuditPanelStub,
          RecipePanel: RecipePanelStub,
          TaskToolbox: TaskToolboxStub,
          WorkspaceFrame: WorkspaceFrameStub,
        },
      },
    })

    await flushPromises()
    expect(wrapper.get('[data-testid="audit-focus"]').text()).toContain('"action":"video:compress"')
    expect(wrapper.get('[data-testid="audit-focus"]').text()).toContain('"search":"demo.small.mp4"')
  })

  it('routes workspace links into diagnostics, recent-tasks, and audit focus state', async () => {
    const wrapper = mount(StatisticsDiagnosticsWorkspace, {
      global: {
        stubs: {
          DiagnosticsCenterPanel: DiagnosticsCenterPanelStub,
          RecentTasksPanel: RecentTasksPanelStub,
          AuditPanel: AuditPanelStub,
          RecipePanel: RecipePanelStub,
          TaskToolbox: TaskToolboxStub,
          WorkspaceFrame: WorkspaceFrameStub,
        },
      },
    })

    await wrapper.get('[data-testid="emit-recent-link"]').trigger('click')
    await flushPromises()
    expect(wrapper.get('[data-testid="recent-focus"]').text()).toContain('"selected_task_id":"task-2"')
    expect(wrapper.get('[data-testid="recent-focus"]').text()).toContain('"status":"failed"')
    expect(wrapper.get('[data-testid="recent-focus"]').text()).toContain('"dry_run":"executed"')

    await wrapper.get('[data-testid="emit-audit-link"]').trigger('click')
    await flushPromises()
    expect(wrapper.get('[data-testid="audit-focus"]').text()).toContain('"search":"D:/repo/demo.txt"')
    expect(wrapper.get('[data-testid="audit-focus"]').text()).toContain('"action":"dashboard.task.execute.acl:owner"')
    expect(wrapper.get('[data-testid="audit-focus"]').text()).toContain('"result":"success"')

    await wrapper.get('[data-testid="emit-recipe-link"]').trigger('click')
    await flushPromises()
    expect(wrapper.get('[data-testid="audit-focus"]').text()).toContain('"search":"D:/tmp/demo.log"')
    expect(wrapper.get('[data-testid="audit-focus"]').text()).toContain('"action":"dashboard.task.execute.rm"')

    await wrapper.get('[data-testid="emit-toolbox-link"]').trigger('click')
    await flushPromises()
    expect(wrapper.get('[data-testid="recent-focus"]').text()).toContain('"status":"succeeded"')
    expect(wrapper.get('[data-testid="recent-focus"]').text()).toContain('"dry_run":"executed"')
    expect(wrapper.get('[data-testid="recent-focus"]').text()).toContain('"action":"cstat"')
    expect(wrapper.get('[data-testid="recent-focus"]').text()).toContain('"search":"D:/logs/demo.log"')

    await wrapper.get('[data-testid="emit-diagnostics-from-recent"]').trigger('click')
    await flushPromises()
    expect(wrapper.get('[data-testid="diagnostics-focus"]').text()).toContain('"panel":"failed"')
    expect(wrapper.get('[data-testid="diagnostics-focus"]').text()).toContain('"task_id":"task-2"')

    await wrapper.get('[data-testid="emit-diagnostics-from-audit"]').trigger('click')
    await flushPromises()
    expect(wrapper.get('[data-testid="diagnostics-focus"]').text()).toContain('"panel":"governance"')
    expect(wrapper.get('[data-testid="diagnostics-focus"]').text()).toContain('"governance_family":"acl"')
    expect(wrapper.get('[data-testid="diagnostics-focus"]').text()).toContain('"audit_action":"dashboard.task.execute.acl:owner"')

    expect(HTMLElement.prototype.scrollIntoView).toHaveBeenCalledTimes(6)
  })
})
