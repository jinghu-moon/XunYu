<script setup lang="ts">
import type { WorkspaceCapabilities } from '../../types'
import { filesSecurityTaskGroups } from '../../workspace-tools'
import DiffPanel from '../DiffPanel.vue'
import RedirectPanel from '../RedirectPanel.vue'
import TaskToolbox from '../TaskToolbox.vue'
import WorkspaceFrame from '../WorkspaceFrame.vue'

defineProps<{
  capabilities?: WorkspaceCapabilities | null
}>()
</script>

<template>
  <WorkspaceFrame title="文件与安全" description="将 Redirect / Diff 与 tree / find / bak / rm / acl / protect / encrypt 收口到统一文件治理工作台。">
    <DiffPanel />
    <RedirectPanel />
    <TaskToolbox
      v-for="group in filesSecurityTaskGroups"
      :key="group.id"
      :title="group.title"
      :description="group.description"
      :tasks="group.tasks"
      :capabilities="capabilities"
    />
  </WorkspaceFrame>
</template>
