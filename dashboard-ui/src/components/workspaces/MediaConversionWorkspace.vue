<script setup lang="ts">
import type { WorkspaceCapabilities } from '../../types'
import { mediaConversionTaskGroups } from '../../workspace-tools'
import TaskToolbox from '../TaskToolbox.vue'
import WorkspaceFrame from '../WorkspaceFrame.vue'

defineProps<{
  capabilities?: WorkspaceCapabilities | null
}>()
</script>

<template>
  <WorkspaceFrame title="媒体与转换" description="统一图像压缩、格式转换、视频探测、压缩与 remux。">
    <TaskToolbox
      v-for="group in mediaConversionTaskGroups"
      :key="group.id"
      :title="group.title"
      :description="group.description"
      :tasks="group.tasks"
      :capabilities="capabilities"
    />
  </WorkspaceFrame>
</template>
