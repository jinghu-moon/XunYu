# Workspace Tools

> **Type**: `Module`
> **Status**: `Stable`
> **Responsibility**: Define dashboard workspace tabs and task definitions, and map form values to CLI arguments.

## Context

- **Problem**: The dashboard needs a declarative catalog of tasks that can run CLI commands with validated inputs.
- **Role**: Provide workspace tabs, task groups, and argument builders used by the UI task runner.
- **Split status**: Consider splitting (multiple workspaces and domains in one file).
- **Collaborators**: `dashboard-ui/src/components/TaskToolCard.vue` (validation + execution), `dashboard-ui/src/api.ts` (run/guarded API), `dashboard-ui/src/types.ts` (capabilities + workspace keys), `src/commands/dashboard/handlers/workspaces.rs` (CLI runner)

## Architecture

workspace-tools.ts
├── Task field + task definitions
├── Workspace tabs
├── Workspace task group catalogs
└── findWorkspaceTaskDefinition()

**Data flow** (Mermaid):
```mermaid
graph TD
  A[WorkspaceTaskDefinition] --> B[TaskToolCard]
  B --> C[/api/workspaces/run]
  C --> D[CurrentProcessTaskRunner]
  D --> E[xun.exe CLI]
```

## Interface Schema

### Types

```ts
export type TaskFieldType = 'text' | 'textarea' | 'number' | 'select' | 'checkbox'
export type TaskFieldValue = string | boolean
export type TaskFormState = Record<string, TaskFieldValue>

export interface TaskFieldOption {
  label: string
  value: string
}

export interface TaskFieldDefinition {
  key: string
  label: string
  type: TaskFieldType
  placeholder?: string
  help?: string
  defaultValue?: TaskFieldValue
  required?: boolean
  options?: TaskFieldOption[]
  min?: number
  max?: number
}

export type TaskNoticeTone = 'info' | 'warning'

export interface TaskNotice {
  text: string
  tone?: TaskNoticeTone
}

export interface WorkspaceTaskDefinition {
  id: string
  workspace: WorkspaceKey
  title: string
  description: string
  action: string
  mode: 'run' | 'guarded'
  tone?: 'default' | 'danger'
  feature?: keyof WorkspaceCapabilities
  notices?: TaskNotice[]
  fields: TaskFieldDefinition[]
  target?: (values: TaskFormState) => string
  buildRunArgs?: (values: TaskFormState) => string[]
  buildPreviewArgs?: (values: TaskFormState) => string[]
  buildExecuteArgs?: (values: TaskFormState) => string[]
  previewSummary?: (values: TaskFormState) => string
}

export interface WorkspaceTaskGroup {
  id: string
  title: string
  description: string
  tasks: WorkspaceTaskDefinition[]
}

export interface WorkspaceTabDefinition {
  value: WorkspaceKey
  label: string
  description: string
}
```

### Exported Catalogs

- `workspaceTabs: WorkspaceTabDefinition[]`
- `pathsContextTaskGroups: WorkspaceTaskGroup[]`
- `networkProxyTaskGroups: WorkspaceTaskGroup[]`
- `filesSecurityTaskGroups: WorkspaceTaskGroup[]`
- `integrationAutomationTaskGroups: WorkspaceTaskGroup[]`
- `desktopControlTaskGroups: WorkspaceTaskGroup[]`
- `mediaConversionTaskGroups: WorkspaceTaskGroup[]`
- `statisticsDiagnosticsTaskGroups: WorkspaceTaskGroup[]`
- `findWorkspaceTaskDefinition(workspace, action): WorkspaceTaskDefinition | null`

### Enum Values

| Value | Behavior |
|---|---|
| `TaskFieldType: 'text'` | Single-line text input |
| `TaskFieldType: 'textarea'` | Multi-line text input |
| `TaskFieldType: 'number'` | Numeric input |
| `TaskFieldType: 'select'` | Select from options |
| `TaskFieldType: 'checkbox'` | Boolean input |
| `WorkspaceTaskDefinition.mode: 'run'` | Executes immediately |
| `WorkspaceTaskDefinition.mode: 'guarded'` | Preview then confirm |

## Constraints

**Invariants:**
- `WorkspaceTaskDefinition.workspace` MUST be a valid `WorkspaceKey`.
- Tasks that require feature gating MUST set `feature` to a `WorkspaceCapabilities` key.
- `mode == 'run'` tasks MUST define `buildRunArgs`.
- `mode == 'guarded'` tasks MUST define `buildPreviewArgs` and `buildExecuteArgs`.
- CLI-required fields MUST set `TaskFieldDefinition.required = true` to enable UI validation.
- `notices` SHOULD explain permissions or interactive requirements (e.g. admin-only, GUI dialogs).

**Error Handling:**

| Scenario | Condition | Behavior |
|---|---|---|
| Missing required field | `required === true` and empty input | UI blocks execution |
| Missing arg builders | `buildRunArgs` / `buildPreviewArgs` / `buildExecuteArgs` absent | Task cannot execute |

## Logic & Behavior

### Decision Rules

- `runTask(def) => { ...def, mode: 'run' }`
- `guardedTask(def) => { ...def, mode: 'guarded' }`
- `pushOption(args, name, value)` adds the pair only when `value` is non-empty.
- `workspaceTaskCatalog = flatMap(workspaceTaskGroupCatalog)`
- `findWorkspaceTaskDefinition` returns first task where `task.workspace == workspace AND task.action == action`.

### State Strategy

- **Source**: `TaskFormState` per task instance
- **Derived**: CLI args from `buildRunArgs` / `buildPreviewArgs` / `buildExecuteArgs`
- **Effects**: TaskToolCard triggers API requests after validation

## Dependencies

| Type | Package | Purpose |
|---|---|---|
| Internal | `dashboard-ui/src/types.ts` | Workspace keys and capabilities |
| Internal | `dashboard-ui/src/components/TaskToolCard.vue` | Validation + execution |
| Internal | `dashboard-ui/src/api.ts` | Run/guarded API calls |

## Patterns

### Basic Usage

```ts
const task = findWorkspaceTaskDefinition('desktop-control', 'desktop:hotkey-bind')
const args = task?.buildRunArgs?.({ hotkey: 'ctrl+alt+t', action: 'run:wt.exe' })
```

### Anti-Patterns

```ts
// Missing buildRunArgs for a run task -> task cannot execute.
runTask({ id: 'bad', workspace: 'desktop-control', title: 'Bad', description: '', action: 'bad', fields: [] })

// Missing required fields -> UI validation blocks execution.
findWorkspaceTaskDefinition('desktop-control', 'desktop:hotkey-bind')?.buildRunArgs?.({ hotkey: '' })
```
