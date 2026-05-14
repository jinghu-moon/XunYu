import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import OperationDialog from './OperationDialog.vue'
import type { Preview, RiskLevel } from '../../generated/types'

function makePreview(overrides: Partial<Preview> = {}): Preview {
  return {
    description: 'Delete bookmark "test"',
    changes: [
      { action: 'delete', target: 'test-bookmark' },
      { action: 'delete', target: 'test-bookmark-2' },
    ],
    risk_level: 'Low',
    ...overrides,
  }
}

describe('OperationDialog', () => {
  describe('Preview 展示', () => {
    it('shows summary from Preview', () => {
      const preview = makePreview()
      const wrapper = mount(OperationDialog, { props: { preview } })

      expect(wrapper.find('[data-testid="preview-description"]').text()).toBe(
        'Delete bookmark "test"',
      )
    })

    it('lists changes', () => {
      const preview = makePreview()
      const wrapper = mount(OperationDialog, { props: { preview } })

      const items = wrapper.findAll('[data-testid="change-item"]')
      expect(items).toHaveLength(2)
      expect(items[0].text()).toContain('delete')
      expect(items[0].text()).toContain('test-bookmark')
      expect(items[1].text()).toContain('test-bookmark-2')
    })

    it('shows risk level badge', () => {
      const preview = makePreview({ risk_level: 'High' })
      const wrapper = mount(OperationDialog, { props: { preview } })

      const badge = wrapper.find('[data-testid="risk-badge"]')
      expect(badge.exists()).toBe(true)
      expect(badge.text()).toBe('High')
    })
  })

  describe('风险等级交互', () => {
    it('Low risk shows green confirm button', () => {
      const preview = makePreview({ risk_level: 'Low' })
      const wrapper = mount(OperationDialog, { props: { preview } })

      const btn = wrapper.find('[data-testid="confirm-btn"]')
      expect(btn.exists()).toBe(true)
      expect(btn.classes()).toContain('risk-low')
    })

    it('High risk shows red button with double confirm', async () => {
      const preview = makePreview({ risk_level: 'High' })
      const wrapper = mount(OperationDialog, { props: { preview } })

      const btn = wrapper.find('[data-testid="confirm-btn"]')
      expect(btn.classes()).toContain('risk-high')

      // First click enables the actual confirm
      await btn.trigger('click')
      expect(wrapper.find('[data-testid="confirm-btn"]').text()).toContain('Confirm')
    })

    it('Critical risk requires text input confirmation', () => {
      const preview = makePreview({ risk_level: 'Critical' })
      const wrapper = mount(OperationDialog, { props: { preview } })

      const input = wrapper.find('[data-testid="critical-confirm-input"]')
      expect(input.exists()).toBe(true)

      const btn = wrapper.find('[data-testid="confirm-btn"]')
      expect(btn.attributes('disabled')).toBeDefined()
    })
  })

  describe('事件', () => {
    it('emits confirm on user accept', async () => {
      const preview = makePreview({ risk_level: 'Low' })
      const wrapper = mount(OperationDialog, { props: { preview } })

      await wrapper.find('[data-testid="confirm-btn"]').trigger('click')
      expect(wrapper.emitted('confirm')).toBeTruthy()
    })

    it('emits cancel on user reject', async () => {
      const preview = makePreview()
      const wrapper = mount(OperationDialog, { props: { preview } })

      await wrapper.find('[data-testid="cancel-btn"]').trigger('click')
      expect(wrapper.emitted('cancel')).toBeTruthy()
    })

    it('emits cancel on Escape key', async () => {
      const preview = makePreview()
      const wrapper = mount(OperationDialog, { props: { preview } })

      window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }))
      await wrapper.vm.$nextTick()
      expect(wrapper.emitted('cancel')).toBeTruthy()
    })
  })
})
