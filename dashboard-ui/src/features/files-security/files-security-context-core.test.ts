import { describe, expect, it } from 'vitest'

import {
  buildAclComparisonPresets,
  buildBatchBackupPresets,
  buildBatchFindPresets,
  buildDirectoryPresets,
  buildSelectionPresets,
  mergePresetMaps,
  normalizeFilesSecurityPath,
  parentDirectory,
} from './files-security-context-core'

describe('files-security-context-core', () => {
  it('normalizes paths and resolves parent directories', () => {
    expect(normalizeFilesSecurityPath('  D:/repo/demo.txt  ')).toBe('D:/repo/demo.txt')
    expect(parentDirectory('D:/repo/demo.txt')).toBe('D:/repo')
    expect(parentDirectory('C:/')).toBe('C:/')
    expect(parentDirectory('/tmp/demo.txt')).toBe('/tmp')
  })

  it('builds directory presets for browsing and backup tasks', () => {
    expect(buildDirectoryPresets('D:/repo')).toEqual({
      tree: { path: 'D:/repo' },
      find: { paths: 'D:/repo' },
      'bak-list': { dir: 'D:/repo' },
      'bak-create': { dir: 'D:/repo' },
    })
  })

  it('builds selection presets and fans out path-bound tasks', () => {
    const presets = buildSelectionPresets('', 'D:/repo/demo.txt')

    expect(presets.tree).toEqual({ path: 'D:/repo' })
    expect(presets.find).toEqual({ paths: 'D:/repo/demo.txt' })
    expect(presets.rm).toEqual({ path: 'D:/repo/demo.txt' })
    expect(presets.mv).toEqual({ src: 'D:/repo/demo.txt' })
    expect(presets['acl-copy']).toEqual({ path: 'D:/repo/demo.txt' })
    expect(presets.decrypt).toEqual({ path: 'D:/repo/demo.txt' })
  })

  it('builds batch presets and preserves fallback backup directory', () => {
    const batchPaths = ['D:/repo/a.txt', 'D:/repo/b.txt']

    expect(buildBatchFindPresets('D:/repo', batchPaths)).toMatchObject({
      find: { paths: 'D:/repo/a.txt\nD:/repo/b.txt' },
    })
    expect(buildBatchBackupPresets('', batchPaths)).toEqual({
      'bak-create': {
        dir: 'D:/repo',
        include: 'D:/repo/a.txt\nD:/repo/b.txt',
      },
    })
  })

  it('merges presets and guards acl comparison against same-path inputs', () => {
    expect(
      mergePresetMaps(
        { find: { paths: 'D:/repo' } },
        { find: { paths: 'D:/repo/demo.txt' }, rm: { path: 'D:/repo/demo.txt' } },
      ),
    ).toEqual({
      find: { paths: 'D:/repo/demo.txt' },
      rm: { path: 'D:/repo/demo.txt' },
    })

    expect(buildAclComparisonPresets('D:/repo/demo.txt', 'D:/repo/demo.txt')).toEqual({})
    expect(buildAclComparisonPresets('D:/repo/demo.txt', 'D:/repo/base.txt')).toEqual({
      'acl-diff': { path: 'D:/repo/demo.txt', reference: 'D:/repo/base.txt' },
      'acl-copy': { path: 'D:/repo/demo.txt', reference: 'D:/repo/base.txt' },
    })
  })
})
