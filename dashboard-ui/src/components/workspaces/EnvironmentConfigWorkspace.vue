<script setup lang="ts">
import type { WorkspaceCapabilities } from '../../types'
import ConfigPanel from '../ConfigPanel.vue'
import EnvPanel from '../EnvPanel.vue'
import WorkspaceFrame from '../WorkspaceFrame.vue'

defineProps<{
  capabilities?: WorkspaceCapabilities | null
}>()
</script>

<template>
  <WorkspaceFrame title="环境与配置" description="保留现有 Config / Env 复合面板，将环境治理能力整体纳入单一工作台。">
    <ConfigPanel />
    <EnvPanel />
  </WorkspaceFrame>
</template>
