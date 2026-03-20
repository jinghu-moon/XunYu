<script setup lang="ts">
import type { StatisticsWorkspaceLinkPayload, WorkspaceCapabilities } from '../../types'
import { integrationAutomationTaskGroups } from '../../features/tasks'
import { useRecentTasksBridge } from '../../features/workspaces/use-recent-tasks-bridge'
import type { TaskPresetMap } from '../../features/workspaces/task-presets'
import { useTaskToolboxPresets } from '../../features/workspaces/use-task-toolbox-presets'
import RecentTasksPanel from '../RecentTasksPanel.vue'
import RecipePanel from '../RecipePanel.vue'
import ShellIntegrationGuidePanel from '../ShellIntegrationGuidePanel.vue'
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
    <ShellIntegrationGuidePanel @apply-task-presets="applyTaskPresets" />
    <RecipePanel
      title="自动化 Recipes"
      description="把 shell 集成与自动化流程固化成顺序工作流。"
      category="integration-automation"
      @link-panel="handleWorkspaceLink"
    />
    <div ref="toolboxAnchor" data-testid="integration-toolbox-anchor">
      <TaskToolbox
        v-for="group in integrationAutomationTaskGroups"
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
