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
  <WorkspaceFrame title="??????" description="?? shell ???completion?alias ?????????????">
    <div ref="recentTasksAnchor" data-testid="integration-recent-tasks-anchor">
      <RecentTasksPanel
        title="???????"
        description="?? init / completion / alias / brn ?????????"
        workspace="integration-automation"
        :limit="12"
        :focus-request="recentTasksFocus"
        @link-panel="handleWorkspaceLink"
      />
    </div>
    <RecipePanel
      title="??? Recipes"
      description="?? shell ?????????????????"
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
