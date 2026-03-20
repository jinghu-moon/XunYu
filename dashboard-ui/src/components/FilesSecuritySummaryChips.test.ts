import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'

import FilesSecuritySummaryChips from './FilesSecuritySummaryChips.vue'

describe('FilesSecuritySummaryChips', () => {
  it('renders placeholders for empty context', () => {
    const wrapper = mount(FilesSecuritySummaryChips, {
      props: {
        currentDirectory: '',
        selectedPath: '',
        aclReferencePath: '',
        batchCount: 0,
      },
    })

    expect(wrapper.get('[data-testid="summary-chip-directory"]').text()).toContain('-')
    expect(wrapper.get('[data-testid="summary-chip-file"]').text()).toContain('-')
    expect(wrapper.get('[data-testid="summary-chip-acl-reference"]').text()).toContain('-')
    expect(wrapper.get('[data-testid="summary-chip-batch-count"]').text()).toContain('0')
  })

  it('renders current values and batch count', () => {
    const wrapper = mount(FilesSecuritySummaryChips, {
      props: {
        currentDirectory: 'D:/repo',
        selectedPath: 'D:/repo/src/main.rs',
        aclReferencePath: 'D:/repo/template.rs',
        batchCount: 3,
      },
    })

    expect(wrapper.get('[data-testid="summary-chip-directory"]').text()).toContain('D:/repo')
    expect(wrapper.get('[data-testid="summary-chip-file"]').text()).toContain('D:/repo/src/main.rs')
    expect(wrapper.get('[data-testid="summary-chip-acl-reference"]').text()).toContain('D:/repo/template.rs')
    expect(wrapper.get('[data-testid="summary-chip-batch-count"]').text()).toContain('3')
  })
})
