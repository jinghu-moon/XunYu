<script setup lang="ts">
import type { StatisticsWorkspaceLinkPayload, WorkspaceCapabilities } from '../../types'
import { desktopControlTaskGroups } from '../../features/tasks'
import { useRecentTasksBridge } from '../../features/workspaces/use-recent-tasks-bridge'
import type { TaskPresetMap } from '../../features/workspaces/task-presets'
import { useTaskToolboxPresets } from '../../features/workspaces/use-task-toolbox-presets'
import RecentTasksPanel from '../RecentTasksPanel.vue'
import RecipePanel from '../RecipePanel.vue'
import TaskToolbox from '../TaskToolbox.vue'
import WorkspaceFrame from '../WorkspaceFrame.vue'

const emit = defineEmits<{
  (event: 'link-panel', payload: StatisticsWorkspaceLinkPayload): void
}>()

const { recentTasksAnchor, recentTasksFocus, handleRecentTasksLink } = useRecentTasksBridge()
const { toolboxAnchor, taskPresets, presetVersion, applyTaskPresets } =
  useTaskToolboxPresets<TaskPresetMap>()

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
  <WorkspaceFrame title="桌面控制" description="窗口、布局、热键与工作区的本地控制入口。">
    <div ref="recentTasksAnchor" data-testid="desktop-recent-tasks-anchor">
      <RecentTasksPanel
        title="最近桌面任务"
        description="回看 desktop 相关任务的执行结果。"
        workspace="desktop-control"
        :limit="12"
        :focus-request="recentTasksFocus"
        @link-panel="handleWorkspaceLink"
      />
    </div>
    <RecipePanel
      title="桌面 Recipes"
      description="把桌面控制流程固化为顺序工作流。"
      category="desktop-control"
      @link-panel="handleWorkspaceLink"
    />
    <div ref="toolboxAnchor" data-testid="desktop-toolbox-anchor">
      <TaskToolbox
        v-for="group in desktopControlTaskGroups"
        :key="group.id"
        :title="group.title"
        :description="group.description"
        :tasks="group.tasks"
        :capabilities="capabilities"
        :task-presets="taskPresets"
        :preset-version="presetVersion"
        @link-panel="handleWorkspaceLink"
      />
    </div>
  </WorkspaceFrame>
</template>
