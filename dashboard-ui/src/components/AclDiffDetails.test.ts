import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import type { WorkspaceTaskDetails } from '../types'
import AclDiffDetails from './AclDiffDetails.vue'

function createDiffDetails(): WorkspaceTaskDetails {
  return {
    kind: 'acl_diff',
    diff: {
      target: 'D:/repo/a.txt',
      reference: 'D:/repo/b.txt',
      common_count: 2,
      has_diff: true,
      owner_diff: {
        target: 'BUILTIN\\Administrators',
        reference: 'NT AUTHORITY\\SYSTEM',
      },
      inheritance_diff: {
        target_protected: false,
        reference_protected: true,
      },
      only_in_target: [
        {
          principal: 'BUILTIN\\Users',
          sid: 'S-1-5-32-545',
          rights: 'Read',
          ace_type: 'Allow',
          source: 'explicit',
          inheritance: 'BothInherit',
          propagation: 'None',
          orphan: false,
        },
      ],
      only_in_reference: [
        {
          principal: 'DOMAIN\\alice',
          sid: 'S-1-5-21-100',
          rights: 'Modify',
          ace_type: 'Allow',
          source: 'inherited',
          inheritance: 'ContainerInherit',
          propagation: 'InheritOnly',
          orphan: true,
        },
      ],
    },
  }
}

describe('AclDiffDetails', () => {
  it('renders structured acl diff entries', () => {
    const wrapper = mount(AclDiffDetails, {
      props: {
        details: createDiffDetails(),
      },
    })

    expect(wrapper.find('[data-testid="acl-diff-details"]').exists()).toBe(true)
    expect(wrapper.text()).toContain('S-1-5-32-545')
    expect(wrapper.text()).toContain('S-1-5-21-100')
    expect(wrapper.text()).toContain('S-1-5-32-545')
    expect(wrapper.text()).toContain('Read')
    expect(wrapper.text()).toContain('Modify')
  })

  it('renders before and after panels for diff transition', () => {
    const details = createDiffDetails()
    if (details.kind !== 'acl_diff') throw new Error('expected acl diff details')
    const current = details.diff
    const wrapper = mount(AclDiffDetails, {
      props: {
        details: {
          kind: 'acl_diff_transition',
          before: current,
          after: {
            ...current,
            has_diff: false,
            only_in_target: [],
            only_in_reference: [],
          },
        } satisfies WorkspaceTaskDetails,
      },
    })

    expect(wrapper.find('[data-testid="acl-diff-panel-before"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="acl-diff-panel-after"]').exists()).toBe(true)
    expect(wrapper.text()).toContain('S-1-5-32-545')
    expect(wrapper.get('[data-testid="acl-diff-panel-after"]').text()).toContain('D:/repo/b.txt')
  })
})
