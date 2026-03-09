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
  <WorkspaceFrame title="媒体与转换" description="统一承载图片转换、视频探测、压缩与 remux 工作流。">
    <div ref="recentTasksAnchor" data-testid="media-recent-tasks-anchor">
      <RecentTasksPanel
        title="最近媒体任务"
        description="回看 img 与 video 相关任务的执行结果。"
        workspace="media-conversion"
        :limit="12"
        :focus-request="recentTasksFocus"
        @link-panel="handleWorkspaceLink"
      />
    </div>
    <RecipePanel
      title="媒体 Recipes"
      description="把常见图片与视频处理流程固化成可复用步骤。"
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
