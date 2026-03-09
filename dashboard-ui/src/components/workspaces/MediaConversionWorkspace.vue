<script setup lang="ts">
import { nextTick, ref } from 'vue'
import type { RecentTasksFocusRequest, StatisticsWorkspaceLinkPayload, WorkspaceCapabilities } from '../../types'
import { mediaConversionTaskGroups } from '../../workspace-tools'
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
  <WorkspaceFrame title="?????" description="???????????????????? remux?">
    <div ref="recentTasksAnchor" data-testid="media-recent-tasks-anchor">
      <RecentTasksPanel
        title="??????"
        description="?? img / video ???????????????"
        workspace="media-conversion"
        :limit="12"
        :focus-request="recentTasksFocus"
        @link-panel="handleWorkspaceLink"
      />
    </div>
    <RecipePanel
      title="?? Recipes"
      description="????????????????????????"
      category="media-conversion"
      @link-panel="handleWorkspaceLink"
    />
    <TaskToolbox
      v-for="group in mediaConversionTaskGroups"
      :key="group.id"
      :title="group.title"
      :description="group.description"
      :tasks="group.tasks"
      :capabilities="capabilities"
      @link-panel="handleWorkspaceLink"
    />
  </WorkspaceFrame>
</template>
