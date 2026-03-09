import { flushPromises, mount } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import AuditPanel from './AuditPanel.vue'

const apiMocks = vi.hoisted(() => ({
  fetchAudit: vi.fn(),
}))

vi.mock('../api', () => ({
  fetchAudit: apiMocks.fetchAudit,
}))

vi.mock('../ui/feedback', () => ({
  pushToast: vi.fn(),
}))

vi.mock('../ui/export', () => ({
  downloadCsv: vi.fn(),
  downloadJson: vi.fn(),
}))

describe('AuditPanel', () => {
  beforeEach(() => {
    apiMocks.fetchAudit.mockReset()
  })

  it('applies focus requests and reloads audit filters', async () => {
    const response = {
      entries: [
        {
          timestamp: 1700000001,
          action: 'dashboard.task.execute.acl:owner',
          target: 'D:/repo/demo.txt',
          user: 'dashboard',
          params: '{}',
          result: 'success',
          reason: '',
        },
      ],
      stats: {
        total: 1,
        by_action: { 'dashboard.task.execute.acl:owner': 1 },
        by_result: { success: 1 },
      },
    }
    apiMocks.fetchAudit.mockResolvedValue(response)

    const wrapper = mount(AuditPanel)
    await flushPromises()

    await wrapper.setProps({
      focusRequest: {
        key: 1,
        search: 'D:/repo/demo.txt',
        action: 'dashboard.task.execute.acl:owner',
        result: 'success',
      },
    })
    await flushPromises()

    expect(apiMocks.fetchAudit).toHaveBeenNthCalledWith(1, {
      limit: 400,
      search: undefined,
      action: undefined,
      result: undefined,
    })
    expect(apiMocks.fetchAudit).toHaveBeenNthCalledWith(2, {
      limit: 400,
      search: 'D:/repo/demo.txt',
      action: 'dashboard.task.execute.acl:owner',
      result: 'success',
    })
    expect((wrapper.get('[data-testid="audit-search"]').element as HTMLInputElement).value).toBe('D:/repo/demo.txt')
    expect((wrapper.get('[data-testid="audit-action"]').element as HTMLSelectElement).value).toBe('dashboard.task.execute.acl:owner')
    expect((wrapper.get('[data-testid="audit-result"]').element as HTMLSelectElement).value).toBe('success')
    expect(wrapper.get('[data-testid="audit-active-filters"]').text()).toContain('D:/repo/demo.txt')
    expect(wrapper.get('[data-testid="audit-active-filters"]').text()).toContain('dashboard.task.execute.acl:owner')
    expect(wrapper.get('[data-testid="audit-active-filters"]').text()).toContain('success')

    await wrapper.get('[data-testid="clear-audit-filters"]').trigger('click')
    await flushPromises()

    expect(apiMocks.fetchAudit).toHaveBeenNthCalledWith(3, {
      limit: 400,
      search: undefined,
      action: undefined,
      result: undefined,
    })
    expect((wrapper.get('[data-testid="audit-search"]').element as HTMLInputElement).value).toBe('')
    expect((wrapper.get('[data-testid="audit-action"]').element as HTMLSelectElement).value).toBe('')
    expect((wrapper.get('[data-testid="audit-result"]').element as HTMLSelectElement).value).toBe('')
    expect(wrapper.find('[data-testid="audit-active-filters"]').exists()).toBe(false)
  })
  it('emits diagnostics-center focus requests from audit entries', async () => {
    apiMocks.fetchAudit.mockResolvedValue({
      entries: [
        {
          timestamp: 1700000001,
          action: 'dashboard.task.execute.acl:owner',
          target: 'D:/repo/demo.txt',
          user: 'dashboard',
          params: '{}',
          result: 'failed',
          reason: 'denied',
        },
      ],
      stats: {
        total: 1,
        by_action: { 'dashboard.task.execute.acl:owner': 1 },
        by_result: { failed: 1 },
      },
    })

    const wrapper = mount(AuditPanel)
    await flushPromises()

    await wrapper.get('[data-testid="audit-link-diagnostics-1700000001"]').trigger('click')

    expect(wrapper.emitted('link-panel')).toHaveLength(1)
    expect(wrapper.emitted('link-panel')?.[0]?.[0]).toEqual({
      panel: 'diagnostics-center',
      request: {
        panel: 'governance',
        governance_family: 'acl',
        governance_status: 'failed',
        target: 'D:/repo/demo.txt',
        audit_action: 'dashboard.task.execute.acl:owner',
        audit_result: 'failed',
        audit_timestamp: 1700000001,
      },
    })
  })

})
