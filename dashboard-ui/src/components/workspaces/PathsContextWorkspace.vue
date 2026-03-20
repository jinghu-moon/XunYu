<script setup lang="ts">
import type { StatisticsWorkspaceLinkPayload, WorkspaceCapabilities } from '../../types'
import { pathsContextTaskGroups } from '../../features/tasks'
import { useRecentTasksBridge } from '../../features/workspaces/use-recent-tasks-bridge'
import BookmarksPanel from '../BookmarksPanel.vue'
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
  <WorkspaceFrame title="路径与上下文" description="围绕 Bookmarks、ctx、recent、gc 与工作区批量打开（ws）管理本地路径上下文。">
    <BookmarksPanel />
    <div ref="recentTasksAnchor" data-testid="paths-recent-tasks-anchor">
      <RecentTasksPanel
        title="最近路径任务"
        description="回看书签、上下文切换与路径治理相关操作。"
        workspace="paths-context"
        :limit="12"
        :focus-request="recentTasksFocus"
        @link-panel="handleWorkspaceLink"
      />
    </div>
    <RecipePanel
      title="路径 Recipes"
      description="把常用路径检查、清理与切换流程固化成可复用步骤。"
      category="paths-context"
      @link-panel="handleWorkspaceLink"
    />
    <TaskToolbox
      v-for="group in pathsContextTaskGroups"
      :key="group.id"
      :title="group.title"
      :description="group.description"
      :tasks="group.tasks"
      :capabilities="capabilities"
      @link-panel="handleWorkspaceLink"
    />
  </WorkspaceFrame>
</template>
