import { describe, expect, it } from 'vitest'

import { getWorkspaceTaskGroups } from './workspace-group-catalog'

describe('workspace-group-catalog', () => {
  it('returns desktop task groups for desktop-control workspace', () => {
    const groups = getWorkspaceTaskGroups('desktop-control')

    expect(groups.length).toBeGreaterThan(0)
    expect(groups.some((group) => group.id === 'desktop-overview')).toBe(true)
  })

  it('returns an empty list for overview workspace', () => {
    expect(getWorkspaceTaskGroups('overview')).toEqual([])
  })
})
