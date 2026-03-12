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

describe('workspace-tools desktop control tasks', () => {
  it('covers core desktop control actions', () => {
    for (const action of [
      'desktop:daemon-status',
      'desktop:daemon-start',
      'desktop:daemon-stop',
      'desktop:daemon-reload',
      'desktop:hotkey-list',
      'desktop:remap-list',
      'desktop:snippet-list',
      'desktop:layout-list',
      'desktop:workspace-list',
      'desktop:window-focus',
      'desktop:theme-status',
      'desktop:awake-status',
      'desktop:color-pick',
      'desktop:hosts-add',
      'desktop:hosts-remove',
      'desktop:hosts-list',
      'desktop:app-list',
      'desktop:run',
    ]) {
      expect(findWorkspaceTaskDefinition('desktop-control', action)).not.toBeNull()
    }
  })

  it('builds desktop task args for key actions', () => {
    const hotkeyBind = findWorkspaceTaskDefinition('desktop-control', 'desktop:hotkey-bind')
    expect(
      hotkeyBind?.buildRunArgs?.({
        hotkey: 'ctrl+alt+t',
        action: 'run:wt.exe',
        app: 'code.exe',
      }),
    ).toEqual(['desktop', 'hotkey', 'bind', 'ctrl+alt+t', 'run:wt.exe', '--app', 'code.exe'])

    const remapAdd = findWorkspaceTaskDefinition('desktop-control', 'desktop:remap-add')
    expect(remapAdd?.mode).toBe('guarded')
    expect(
      remapAdd?.buildPreviewArgs?.({
        from: 'ctrl+alt+1',
        to: 'alt+1',
        app: 'code.exe',
        exact: true,
      }),
    ).toEqual(['desktop', 'remap', 'add', 'ctrl+alt+1', 'alt+1', '--app', 'code.exe', '--exact', '--dry-run'])
    expect(
      remapAdd?.buildExecuteArgs?.({
        from: 'ctrl+alt+1',
        to: 'alt+1',
        app: 'code.exe',
        exact: true,
      }),
    ).toEqual(['desktop', 'remap', 'add', 'ctrl+alt+1', 'alt+1', '--app', 'code.exe', '--exact'])

    const layoutNew = findWorkspaceTaskDefinition('desktop-control', 'desktop:layout-new')
    expect(
      layoutNew?.buildRunArgs?.({
        name: 'dev',
        layout_type: 'grid',
        rows: '2',
        cols: '3',
        gap: '8',
      }),
    ).toEqual([
      'desktop',
      'layout',
      'new',
      'dev',
      '--layout-type',
      'grid',
      '--rows',
      '2',
      '--cols',
      '3',
      '--gap',
      '8',
    ])

    const windowTop = findWorkspaceTaskDefinition('desktop-control', 'desktop:window-top')
    expect(windowTop?.buildRunArgs?.({ mode: 'enable', app: 'code.exe' })).toEqual([
      'desktop',
      'window',
      'top',
      '--enable',
      '--app',
      'code.exe',
    ])
    expect(windowTop?.buildRunArgs?.({ mode: 'disable' })).toEqual(['desktop', 'window', 'top', '--disable'])

    const awakeOn = findWorkspaceTaskDefinition('desktop-control', 'desktop:awake-on')
    expect(awakeOn?.buildRunArgs?.({ duration: '45m', display_on: true })).toEqual([
      'desktop',
      'awake',
      'on',
      '--duration',
      '45m',
      '--display-on',
    ])

    const hostsAdd = findWorkspaceTaskDefinition('desktop-control', 'desktop:hosts-add')
    expect(hostsAdd?.mode).toBe('guarded')
    expect(hostsAdd?.buildPreviewArgs?.({ host: 'example.com', ip: '127.0.0.1' })).toEqual([
      'desktop',
      'hosts',
      'add',
      'example.com',
      '127.0.0.1',
      '--dry-run',
    ])
    expect(hostsAdd?.buildExecuteArgs?.({ host: 'example.com', ip: '127.0.0.1' })).toEqual([
      'desktop',
      'hosts',
      'add',
      'example.com',
      '127.0.0.1',
    ])

    const runCmd = findWorkspaceTaskDefinition('desktop-control', 'desktop:run')
    expect(runCmd?.buildRunArgs?.({ command: 'wt.exe -d .' })).toEqual(['desktop', 'run', 'wt.exe -d .'])
  })
})
