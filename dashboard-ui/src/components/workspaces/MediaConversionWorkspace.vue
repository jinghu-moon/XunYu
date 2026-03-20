<script setup lang="ts">
import type { StatisticsWorkspaceLinkPayload, WorkspaceCapabilities } from '../../types'
import { mediaConversionTaskGroups } from '../../features/tasks'
import { useRecentTasksBridge } from '../../features/workspaces/use-recent-tasks-bridge'
import RecentTasksPanel from '../RecentTasksPanel.vue'
import RecipePanel from '../RecipePanel.vue'
import TaskToolbox from '../TaskToolbox.vue'
import WorkspaceFrame from '../WorkspaceFrame.vue'

const emit = defineEmits<{
  (event: 'link-panel', payload: StatisticsWorkspaceLinkPayload): void
}>()

const { recentTasksAnchor, recentTasksFocus, handleRecentTasksLink } = useRecentTasksBridge()

defineProps<{
  capabilities?: WorkspaceCapabilities | null
}>()

async function handleWorkspaceLink(payload: StatisticsWorkspaceLinkPayload) {
  await handleRecentTasksLink(payload, (nextPayload) => {
    emit('link-panel', nextPayload)
  })
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
