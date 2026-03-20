import type { TaskFormState, WorkspaceTaskDefinition } from './catalog.types'
import { JSON_FORMAT } from './catalog.options'

export function readText(values: TaskFormState, key: string): string {
  const value = values[key]
  return typeof value === 'string' ? value.trim() : ''
}

export function readBool(values: TaskFormState, key: string): boolean {
  return values[key] === true
}

export function splitItems(raw: string): string[] {
  return raw
    .split(/[\n,]+/)
    .map((item) => item.trim())
    .filter(Boolean)
}

export function splitCommand(raw: string): string[] {
  const matches = raw.match(/"([^"]*)"|'([^']*)'|\S+/g) ?? []
  return matches.map((part) => part.replace(/^['"]|['"]$/g, ''))
}

export function pushOption(args: string[], name: string, value: string) {
  if (value) args.push(name, value)
}

export function pushRepeatableOption(args: string[], name: string, raw: string) {
  for (const item of splitItems(raw)) {
    args.push(name, item)
  }
}

export function runTask(definition: Omit<WorkspaceTaskDefinition, 'mode'>): WorkspaceTaskDefinition {
  return { ...definition, mode: 'run' }
}

export function guardedTask(definition: Omit<WorkspaceTaskDefinition, 'mode'>): WorkspaceTaskDefinition {
  return { ...definition, mode: 'guarded' }
}

export function pathTarget(values: TaskFormState, key = 'path'): string {
  return readText(values, key)
}

export function previewPath(values: TaskFormState, key = 'path'): string[] {
  const path = readText(values, key)
  const args = ['find', '--dry-run', '-f', JSON_FORMAT]
  pushOption(args, '--test-path', path)
  return args
}

export function moveLikeArgs(command: 'mv' | 'ren', values: TaskFormState, dryRun: boolean): string[] {
  const src = readText(values, 'src')
  const dst = readText(values, 'dst')
  const args: string[] = [command]
  if (readBool(values, 'unlock')) args.push('--unlock')
  if (readBool(values, 'forceKill')) args.push('--force-kill')
  if (dryRun) args.push('--dry-run')
  if (!dryRun) args.push('-y')
  if (readBool(values, 'force')) args.push('--force')
  pushOption(args, '--reason', readText(values, 'reason'))
  args.push(src, dst)
  return args
}
