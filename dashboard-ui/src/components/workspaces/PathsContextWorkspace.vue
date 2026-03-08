<script setup lang="ts">
import type { WorkspaceCapabilities } from '../../types'
import { pathsContextTaskGroups } from '../../workspace-tools'
import BookmarksPanel from '../BookmarksPanel.vue'
import TaskToolbox from '../TaskToolbox.vue'
import WorkspaceFrame from '../WorkspaceFrame.vue'

defineProps<{
  capabilities?: WorkspaceCapabilities | null
}>()
</script>

<template>
  <WorkspaceFrame title="路径与上下文" description="将 Bookmarks 与 ctx / recent / gc / ws 等路径能力统一成一个工作台。">
    <BookmarksPanel />
    <TaskToolbox
      v-for="group in pathsContextTaskGroups"
      :key="group.id"
      :title="group.title"
      :description="group.description"
      :tasks="group.tasks"
      :capabilities="capabilities"
    />
  </WorkspaceFrame>
</template>
