import type { TaskFormState } from '../tasks'
import type { TaskPresetMap } from '../workspaces/task-presets'

const SELECTION_PATH_TASK_IDS = [
  'lock-who',
  'protect-status',
  'protect-set',
  'protect-clear',
  'acl-view',
  'acl-diff',
  'acl-add',
  'acl-effective',
  'acl-backup',
  'acl-copy',
  'acl-restore',
  'acl-purge',
  'acl-inherit',
  'acl-owner',
  'acl-repair',
  'encrypt',
  'decrypt',
] as const

export function normalizeFilesSecurityPath(path: string): string {
  return path.trim()
}

export function parentDirectory(path: string): string {
  const normalized = normalizeFilesSecurityPath(path)
  if (!normalized) return ''
  if (normalized === '/' || /^[A-Za-z]:[\\/]?$/.test(normalized)) {
    return normalized
  }

  const separator = normalized.includes('\\') ? '\\' : '/'
  const trimmed = normalized.replace(/[\\/]+$/, '')
  const index = trimmed.lastIndexOf(separator)
  if (index <= 0) {
    return /^[A-Za-z]:/.test(trimmed) ? `${trimmed.slice(0, 2)}\\` : '/'
  }

  const head = trimmed.slice(0, index)
  if (/^[A-Za-z]:$/.test(head)) return `${head}\\`
  return head
}

function pushTaskPreset(target: TaskPresetMap, taskId: string, values: Partial<TaskFormState>) {
  target[taskId] = {
    ...(target[taskId] ?? {}),
    ...values,
  }
}

export function mergePresetMaps(...maps: TaskPresetMap[]): TaskPresetMap {
  return maps.reduce<TaskPresetMap>((accumulator, current) => {
    for (const [taskId, values] of Object.entries(current)) {
      pushTaskPreset(accumulator, taskId, values)
    }
    return accumulator
  }, {})
}

export function buildDirectoryPresets(currentDirectory: string): TaskPresetMap {
  const directory = normalizeFilesSecurityPath(currentDirectory)
  if (!directory) return {}

  const next: TaskPresetMap = {}
  pushTaskPreset(next, 'tree', { path: directory })
  pushTaskPreset(next, 'find', { paths: directory })
  pushTaskPreset(next, 'bak-list', { dir: directory })
  pushTaskPreset(next, 'bak-create', { dir: directory })
  return next
}

export function buildSelectionPresets(currentDirectory: string, selectedPath: string): TaskPresetMap {
  const path = normalizeFilesSecurityPath(selectedPath)
  if (!path) return {}

  const directory = normalizeFilesSecurityPath(currentDirectory) || parentDirectory(path)
  const next: TaskPresetMap = {}
  if (directory) {
    pushTaskPreset(next, 'tree', { path: directory })
    pushTaskPreset(next, 'bak-create', { dir: directory })
  }

  pushTaskPreset(next, 'find', { paths: path })
  pushTaskPreset(next, 'bak-create', { include: path })
  pushTaskPreset(next, 'rm', { path })
  pushTaskPreset(next, 'mv', { src: path })
  pushTaskPreset(next, 'ren', { src: path })

  for (const taskId of SELECTION_PATH_TASK_IDS) {
    pushTaskPreset(next, taskId, { path })
  }

  return next
}

export function buildBatchFindPresets(currentDirectory: string, batchPaths: string[]): TaskPresetMap {
  if (!batchPaths.length) return {}

  const next = buildDirectoryPresets(currentDirectory)
  pushTaskPreset(next, 'find', { paths: batchPaths.join('\n') })
  return next
}

export function buildBatchBackupPresets(currentDirectory: string, batchPaths: string[]): TaskPresetMap {
  if (!batchPaths.length) return {}

  const next = buildDirectoryPresets(currentDirectory)
  const fallbackDirectory = normalizeFilesSecurityPath(currentDirectory) || parentDirectory(batchPaths[0])
  if (fallbackDirectory) {
    pushTaskPreset(next, 'bak-create', { dir: fallbackDirectory })
  }
  pushTaskPreset(next, 'bak-create', { include: batchPaths.join('\n') })
  return next
}

export function buildAclComparisonPresets(selectedPath: string, aclReferencePath: string): TaskPresetMap {
  const path = normalizeFilesSecurityPath(selectedPath)
  const reference = normalizeFilesSecurityPath(aclReferencePath)
  if (!path || !reference || path === reference) return {}

  const next: TaskPresetMap = {}
  pushTaskPreset(next, 'acl-diff', { path, reference })
  pushTaskPreset(next, 'acl-copy', { path, reference })
  return next
}
