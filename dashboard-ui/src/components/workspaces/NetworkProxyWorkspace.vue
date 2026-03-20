<script setup lang="ts">
import type { StatisticsWorkspaceLinkPayload, WorkspaceCapabilities } from '../../types'
import { networkProxyTaskGroups } from '../../features/tasks'
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
  <WorkspaceFrame title="网络与代理" description="统一查看端口、进程、代理状态，并串联 px / kill 等网络排查任务。">
    <PortsPanel disable-kill />
    <ProxyPanel />
    <RecentTasksPanel
      title="最近网络任务"
      description="回看代理切换、端口排查与命令执行结果。"
      workspace="network-proxy"
      :limit="12"
      @link-panel="emit('link-panel', $event)"
    />
    <RecipePanel
      title="网络 Recipes"
      description="把常用代理与端口排查步骤固化为可复用流程。"
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
