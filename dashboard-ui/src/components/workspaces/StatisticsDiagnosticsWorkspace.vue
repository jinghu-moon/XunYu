<script setup lang="ts">
import type { StatisticsWorkspaceLinkPayload, WorkspaceCapabilities } from '../../types'
import { statisticsDiagnosticsTaskGroups } from '../../features/tasks'
import { useStatisticsDiagnosticsBridge } from '../../features/workspaces/use-statistics-diagnostics-bridge'
import AuditPanel from '../AuditPanel.vue'
import DiagnosticsCenterPanel from '../DiagnosticsCenterPanel.vue'
import RecentTasksPanel from '../RecentTasksPanel.vue'
import RecipePanel from '../RecipePanel.vue'
import TaskToolbox from '../TaskToolbox.vue'
import WorkspaceFrame from '../WorkspaceFrame.vue'

const props = defineProps<{
  capabilities?: WorkspaceCapabilities | null
  externalLink?: {
    key: number
    payload: StatisticsWorkspaceLinkPayload
  } | null
}>()

const {
  auditAnchor,
  auditFocus,
  diagnosticsAnchor,
  diagnosticsFocus,
  handleDiagnosticsLink,
  recentTasksAnchor,
  recentTasksFocus,
} = useStatisticsDiagnosticsBridge(props)
</script>

<template>
  <WorkspaceFrame title="统计与诊断" description="集中查看诊断、任务复盘、Recipe 与审计时间线。">
    <div ref="diagnosticsAnchor" data-testid="statistics-diagnostics-anchor">
      <DiagnosticsCenterPanel :focus-request="diagnosticsFocus" @link-panel="handleDiagnosticsLink" />
    </div>
    <div ref="recentTasksAnchor" data-testid="statistics-recent-tasks-anchor">
      <RecentTasksPanel
        title="最近任务"
        description="按工作流回看最近执行、失败与 dry-run 任务。"
        :limit="20"
        :focus-request="recentTasksFocus"
        @link-panel="handleDiagnosticsLink"
      />
    </div>
    <RecipePanel
      title="Recipe 工作流"
      description="预演、确认并回放高频顺序任务。"
      @link-panel="handleDiagnosticsLink"
    />
    <div ref="auditAnchor" data-testid="statistics-audit-anchor">
      <AuditPanel :focus-request="auditFocus" @link-panel="handleDiagnosticsLink" />
    </div>
    <TaskToolbox
      v-for="group in statisticsDiagnosticsTaskGroups"
      :key="group.id"
      :title="group.title"
      :description="group.description"
      :tasks="group.tasks"
      :capabilities="capabilities"
      @link-panel="handleDiagnosticsLink"
    />
  </WorkspaceFrame>
</template>
