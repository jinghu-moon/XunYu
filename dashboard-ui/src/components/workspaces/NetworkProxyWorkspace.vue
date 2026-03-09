<script setup lang="ts">
import type { StatisticsWorkspaceLinkPayload, WorkspaceCapabilities } from '../../types'
import { networkProxyTaskGroups } from '../../workspace-tools'
import PortsPanel from '../PortsPanel.vue'
import ProxyPanel from '../ProxyPanel.vue'
import RecipePanel from '../RecipePanel.vue'
import RecentTasksPanel from '../RecentTasksPanel.vue'
import TaskToolbox from '../TaskToolbox.vue'
import WorkspaceFrame from '../WorkspaceFrame.vue'

const emit = defineEmits<{
  (event: 'link-panel', payload: StatisticsWorkspaceLinkPayload): void
}>()

defineProps<{
  capabilities?: WorkspaceCapabilities | null
}>()
</script>

<template>
  <WorkspaceFrame title="?????" description="????????????????????? kill ????????????">
    <PortsPanel disable-kill />
    <ProxyPanel />
    <RecentTasksPanel
      title="??????"
      description="????????????????????"
      workspace="network-proxy"
      :limit="12"
      @link-panel="emit('link-panel', $event)"
    />
    <RecipePanel
      title="?? Recipes"
      description="????????????????"
      category="network-proxy"
      @link-panel="emit('link-panel', $event)"
    />
    <TaskToolbox
      v-for="group in networkProxyTaskGroups"
      :key="group.id"
      :title="group.title"
      :description="group.description"
      :tasks="group.tasks"
      :capabilities="capabilities"
      @link-panel="emit('link-panel', $event)"
    />
  </WorkspaceFrame>
</template>
