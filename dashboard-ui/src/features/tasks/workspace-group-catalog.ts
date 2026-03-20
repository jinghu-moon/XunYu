import type { WorkspaceKey } from '../../types'
import type { WorkspaceTaskGroup } from './catalog'
import {
  desktopControlTaskGroups,
  filesSecurityTaskGroups,
  integrationAutomationTaskGroups,
  mediaConversionTaskGroups,
  networkProxyTaskGroups,
  pathsContextTaskGroups,
  statisticsDiagnosticsTaskGroups,
} from './catalog'

export const workspaceTaskGroupCatalog: Partial<Record<WorkspaceKey, WorkspaceTaskGroup[]>> = {
  overview: [],
  'paths-context': pathsContextTaskGroups,
  'network-proxy': networkProxyTaskGroups,
  'environment-config': [],
  'files-security': filesSecurityTaskGroups,
  'integration-automation': integrationAutomationTaskGroups,
  'media-conversion': mediaConversionTaskGroups,
  'desktop-control': desktopControlTaskGroups,
  'statistics-diagnostics': statisticsDiagnosticsTaskGroups,
}

export function getWorkspaceTaskGroups(workspace: WorkspaceKey): WorkspaceTaskGroup[] {
  return workspaceTaskGroupCatalog[workspace] ?? []
}
