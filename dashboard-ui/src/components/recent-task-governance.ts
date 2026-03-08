import type { RecentTaskRecord, TaskProcessOutput } from '../types'
import { findWorkspaceTaskDefinition, type TaskFormState, type WorkspaceTaskDefinition } from '../workspace-tools'

export interface RecentTaskGovernanceContext {
  task: WorkspaceTaskDefinition
  form: TaskFormState
  phase: 'preview' | 'execute'
  process: TaskProcessOutput
}

function createInitialForm(task: WorkspaceTaskDefinition): TaskFormState {
  return task.fields.reduce<TaskFormState>((state, field) => {
    state[field.key] = field.defaultValue ?? (field.type === 'checkbox' ? false : '')
    return state
  }, {})
}

function readOption(args: string[], name: string): string {
  const index = args.indexOf(name)
  if (index < 0 || index + 1 >= args.length) return ''
  return args[index + 1] ?? ''
}

function readRepeatableOption(args: string[], name: string): string[] {
  const values: string[] = []
  for (let index = 0; index < args.length; index += 1) {
    if (args[index] === name && index + 1 < args.length) {
      values.push(args[index + 1] ?? '')
      index += 1
    }
  }
  return values.filter(Boolean)
}

function hasFlag(args: string[], name: string): boolean {
  return args.includes(name)
}

function lastArg(args: string[]): string {
  return args[args.length - 1] ?? ''
}

function parseFilesSecurityForm(task: WorkspaceTaskDefinition, args: string[]): TaskFormState {
  const form = createInitialForm(task)

  switch (task.action) {
    case 'protect:set':
      form.path = args[2] ?? ''
      form.deny = readOption(args, '--deny')
      form.require = readOption(args, '--require')
      form.systemAcl = hasFlag(args, '--system-acl')
      break
    case 'protect:clear':
      form.path = args[2] ?? ''
      form.systemAcl = hasFlag(args, '--system-acl')
      break
    case 'acl:add':
      form.path = readOption(args, '-p')
      form.principal = readOption(args, '--principal')
      form.rights = readOption(args, '--rights') || 'Read'
      form.aceType = readOption(args, '--ace-type') || 'Allow'
      form.inherit = readOption(args, '--inherit') || 'BothInherit'
      break
    case 'acl:diff':
      form.path = readOption(args, '-p')
      form.reference = readOption(args, '-r')
      form.output = readOption(args, '-o')
      break
    case 'acl:effective':
      form.path = readOption(args, '-p')
      form.user = readOption(args, '-u')
      break
    case 'acl:backup':
      form.path = readOption(args, '-p')
      form.output = readOption(args, '-o')
      break
    case 'acl:copy':
      form.path = readOption(args, '-p')
      form.reference = readOption(args, '-r')
      break
    case 'acl:restore':
      form.path = readOption(args, '-p')
      form.from = readOption(args, '--from')
      break
    case 'acl:purge':
      form.path = readOption(args, '-p')
      form.principal = readOption(args, '--principal')
      break
    case 'acl:inherit':
      form.path = readOption(args, '-p')
      form.mode = hasFlag(args, '--disable') ? 'disable' : 'enable'
      form.preserve = readOption(args, '--preserve') !== 'false'
      break
    case 'acl:owner':
      form.path = readOption(args, '-p')
      form.set = readOption(args, '--set')
      break
    case 'acl:repair':
      form.path = readOption(args, '-p')
      form.exportErrors = hasFlag(args, '--export-errors')
      break
    case 'encrypt':
      form.path = lastArg(args)
      form.efs = hasFlag(args, '--efs')
      form.to = readRepeatableOption(args, '--to').join('\n')
      form.out = readOption(args, '-o')
      break
    case 'decrypt':
      form.path = lastArg(args)
      form.efs = hasFlag(args, '--efs')
      form.identity = readRepeatableOption(args, '-i').join('\n')
      form.out = readOption(args, '-o')
      break
    default:
      break
  }

  return form
}

function resolveReplayArgs(record: RecentTaskRecord): string[] {
  if (!record.replay) return []
  return record.replay.kind === 'run' ? record.replay.request.args : record.replay.request.execute_args
}

export function resolveRecentTaskGovernanceContext(
  record: RecentTaskRecord,
  processOverride?: TaskProcessOutput,
  phaseOverride?: 'preview' | 'execute',
): RecentTaskGovernanceContext | null {
  if (record.workspace !== 'files-security') return null

  const task = findWorkspaceTaskDefinition(record.workspace, record.action)
  if (!task) return null

  const args = resolveReplayArgs(record)
  if (!args.length) return null

  const form = parseFilesSecurityForm(task, args)
  return {
    task,
    form,
    phase: phaseOverride ?? (record.phase === 'preview' ? 'preview' : 'execute'),
    process: processOverride ?? record.process,
  }
}
