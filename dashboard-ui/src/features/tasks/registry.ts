import type { WorkspaceTaskDefinition } from './catalog'
import { workspaceTaskGroupCatalog } from './workspace-group-catalog'

const workspaceTaskCatalog = Object.values(workspaceTaskGroupCatalog).flatMap(
  (groups) => groups?.flatMap((group) => group.tasks) ?? [],
)

export function findWorkspaceTaskDefinition(
  workspace: string,
  action: string,
): WorkspaceTaskDefinition | null {
  return workspaceTaskCatalog.find((task) => task.workspace === workspace && task.action === action) ?? null
}
