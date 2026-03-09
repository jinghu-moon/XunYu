<script setup lang="ts">
import { nextTick, ref } from 'vue'
import type { RecentTasksFocusRequest, StatisticsWorkspaceLinkPayload, WorkspaceCapabilities } from '../../types'
import { integrationAutomationTaskGroups } from '../../workspace-tools'
import RecentTasksPanel from '../RecentTasksPanel.vue'
import RecipePanel from '../RecipePanel.vue'
import TaskToolbox from '../TaskToolbox.vue'
import WorkspaceFrame from '../WorkspaceFrame.vue'

const emit = defineEmits<{
  (event: 'link-panel', payload: StatisticsWorkspaceLinkPayload): void
}>()

const recentTasksFocus = ref<RecentTasksFocusRequest | null>(null)
const recentTasksFocusKey = ref(0)
const recentTasksAnchor = ref<HTMLElement | null>(null)

defineProps<{
  capabilities?: WorkspaceCapabilities | null
}>()

function nextFocusKey() {
  recentTasksFocusKey.value += 1
  return recentTasksFocusKey.value
}

async function focusRecentTasks(request: Omit<RecentTasksFocusRequest, 'key'>) {
  recentTasksFocus.value = { key: nextFocusKey(), ...request }
  await nextTick()
  recentTasksAnchor.value?.scrollIntoView?.({ behavior: 'smooth', block: 'start' })
}

async function handleWorkspaceLink(payload: StatisticsWorkspaceLinkPayload) {
  if (payload.panel === 'recent-tasks') {
    await focusRecentTasks(payload.request)
    return
  }
  emit('link-panel', payload)
}
</script>

<template>
  <WorkspaceFrame title="集成与自动化" description="围绕 shell 初始化、completion、alias 与批量重命名组织自动化能力。">
    <div ref="recentTasksAnchor" data-testid="integration-recent-tasks-anchor">
      <RecentTasksPanel
        title="最近自动化任务"
        description="回看 init、completion、alias 与 brn 的执行结果。"
        workspace="integration-automation"
        :limit="12"
        :focus-request="recentTasksFocus"
        @link-panel="handleWorkspaceLink"
      />
    </div>
    <RecipePanel
      title="自动化 Recipes"
      description="把 shell 集成与自动化流程固化成顺序工作流。"
      category="integration-automation"
      @link-panel="handleWorkspaceLink"
    />
    <TaskToolbox
      v-for="group in integrationAutomationTaskGroups"
      :key="group.id"
      :title="group.title"
      :description="group.description"
      :tasks="group.tasks"
      :capabilities="capabilities"
      @link-panel="handleWorkspaceLink"
    />
  </WorkspaceFrame>
</template>
