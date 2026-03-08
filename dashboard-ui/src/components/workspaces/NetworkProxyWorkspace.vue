<script setup lang="ts">
import type { WorkspaceCapabilities } from '../../types'
import { networkProxyTaskGroups } from '../../workspace-tools'
import PortsPanel from '../PortsPanel.vue'
import ProxyPanel from '../ProxyPanel.vue'
import TaskToolbox from '../TaskToolbox.vue'
import WorkspaceFrame from '../WorkspaceFrame.vue'

defineProps<{
  capabilities?: WorkspaceCapabilities | null
}>()
</script>

<template>
  <WorkspaceFrame title="网络与代理" description="整合端口、进程、代理状态与代理执行。高风险 kill 动作统一走任务卡确认流。">
    <PortsPanel disable-kill />
    <ProxyPanel />
    <TaskToolbox
      v-for="group in networkProxyTaskGroups"
      :key="group.id"
      :title="group.title"
      :description="group.description"
      :tasks="group.tasks"
      :capabilities="capabilities"
    />
  </WorkspaceFrame>
</template>
