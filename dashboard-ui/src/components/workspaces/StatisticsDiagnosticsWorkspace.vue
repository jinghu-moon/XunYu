<script setup lang="ts">
import type { WorkspaceCapabilities } from '../../types'
import { statisticsDiagnosticsTaskGroups } from '../../workspace-tools'
import AuditPanel from '../AuditPanel.vue'
import TaskToolbox from '../TaskToolbox.vue'
import WorkspaceFrame from '../WorkspaceFrame.vue'

defineProps<{
  capabilities?: WorkspaceCapabilities | null
}>()
</script>

<template>
  <WorkspaceFrame title="统计与诊断" description="将审计中心与 cstat 统计任务统一收口。">
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
