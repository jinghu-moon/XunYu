<script setup lang="ts">
import type { WorkspaceCapabilities } from '../../types'
import { integrationAutomationTaskGroups } from '../../workspace-tools'
import TaskToolbox from '../TaskToolbox.vue'
import WorkspaceFrame from '../WorkspaceFrame.vue'

defineProps<{
  capabilities?: WorkspaceCapabilities | null
}>()
</script>

<template>
  <WorkspaceFrame title="集成与自动化" description="提供 shell 集成、completion、alias 和批量改名的一站式操作台。">
    <TaskToolbox
      v-for="group in integrationAutomationTaskGroups"
      :key="group.id"
      :title="group.title"
      :description="group.description"
      :tasks="group.tasks"
      :capabilities="capabilities"
    />
  </WorkspaceFrame>
</template>
