import { describe, expect, it } from 'vitest'
import { findWorkspaceTaskDefinition } from './workspace-tools'

describe('workspace-tools integration automation tasks', () => {
  it('guards destructive alias removals with preview then execute', () => {
    const aliasRm = findWorkspaceTaskDefinition('integration-automation', 'alias:rm')
    const appRm = findWorkspaceTaskDefinition('integration-automation', 'alias:app-rm')

    expect(aliasRm?.mode).toBe('guarded')
    expect(aliasRm?.buildPreviewArgs?.({ name: 'gst' })).toEqual(['alias', 'which', 'gst'])
    expect(aliasRm?.buildExecuteArgs?.({ name: 'gst' })).toEqual(['alias', 'rm', 'gst'])

    expect(appRm?.mode).toBe('guarded')
    expect(appRm?.buildPreviewArgs?.({ name: 'code' })).toEqual(['alias', 'app', 'which', 'code'])
    expect(appRm?.buildExecuteArgs?.({ name: 'code' })).toEqual(['alias', 'app', 'rm', 'code'])
  })

  it('covers the remaining alias management actions in the workspace', () => {
    for (const action of [
      'alias:setup',
      'alias:add',
      'alias:ls',
      'alias:find',
      'alias:which',
      'alias:sync',
      'alias:export',
      'alias:import',
      'alias:app-add',
      'alias:app-ls',
      'alias:app-scan',
      'alias:app-which',
      'alias:app-sync',
    ]) {
      expect(findWorkspaceTaskDefinition('integration-automation', action)).not.toBeNull()
    }
  })
})

describe('workspace-tools media conversion tasks', () => {
  it('passes advanced img parameters through to CLI args', () => {
    const img = findWorkspaceTaskDefinition('media-conversion', 'img')

    expect(img).not.toBeNull()
    expect(
      img?.buildRunArgs?.({
        input: 'D:/images',
        output: 'D:/out',
        format: 'svg',
        quality: '92',
        svg_method: 'diffvg',
        svg_diffvg_iters: '220',
        svg_diffvg_strokes: '48',
        jpeg_backend: 'moz',
        png_lossy: false,
        png_dither_level: '0.25',
        webp_lossy: false,
        mw: '1920',
        mh: '1080',
        threads: '8',
        avif_threads: '4',
        overwrite: true,
      }),
    ).toEqual([
      'img',
      '-i',
      'D:/images',
      '-o',
      'D:/out',
      '-f',
      'svg',
      '-q',
      '92',
      '--svg-method',
      'diffvg',
      '--svg-diffvg-iters',
      '220',
      '--svg-diffvg-strokes',
      '48',
      '--jpeg-backend',
      'moz',
      '--png-lossy',
      'false',
      '--png-dither-level',
      '0.25',
      '--webp-lossy',
      'false',
      '--mw',
      '1920',
      '--mh',
      '1080',
      '-t',
      '8',
      '--avif-threads',
      '4',
      '--overwrite',
    ])
  })
})
