<script setup lang="ts">
import type { WorkspaceCapabilities } from '../../types'
import { statisticsDiagnosticsTaskGroups } from '../../workspace-tools'
import AuditPanel from '../AuditPanel.vue'
import DiagnosticsCenterPanel from '../DiagnosticsCenterPanel.vue'
import RecentTasksPanel from '../RecentTasksPanel.vue'
import RecipePanel from '../RecipePanel.vue'
import TaskToolbox from '../TaskToolbox.vue'
import WorkspaceFrame from '../WorkspaceFrame.vue'

defineProps<{
  capabilities?: WorkspaceCapabilities | null
}>()
</script>

<template>
  <WorkspaceFrame title="统计与诊断" description="将最近任务、审计中心与 cstat 统计任务统一收口。">
    <DiagnosticsCenterPanel />
    <RecentTasksPanel title="任务中心" description="集中查看最近成功、失败和 Dry Run 记录，并支持安全重放。" :limit="20" />
    <RecipePanel title="Recipe 工作流" description="保存、预演并确认执行顺序工作流。危险步骤仍然走 Triple-Guard。" />
    <AuditPanel />
    <TaskToolbox
      v-for="group in statisticsDiagnosticsTaskGroups"
      :key="group.id"
      :title="group.title"
      :description="group.description"
      :tasks="group.tasks"
      :capabilities="capabilities"
    />
  </WorkspaceFrame>
</template>
