<script setup lang="ts">
import { nextTick, ref } from 'vue'
import type { RecentTasksFocusRequest, StatisticsWorkspaceLinkPayload, WorkspaceCapabilities } from '../../types'
import { desktopControlTaskGroups } from '../../workspace-tools'
import RecentTasksPanel from '../RecentTasksPanel.vue'
import RecipePanel from '../RecipePanel.vue'
import TaskToolbox from '../TaskToolbox.vue'
import WorkspaceFrame from '../WorkspaceFrame.vue'

type TaskPresetMap = Record<string, Partial<Record<string, string | boolean>>>

const emit = defineEmits<{
  (event: 'link-panel', payload: StatisticsWorkspaceLinkPayload): void
}>()

const recentTasksFocus = ref<RecentTasksFocusRequest | null>(null)
const recentTasksFocusKey = ref(0)
const recentTasksAnchor = ref<HTMLElement | null>(null)
const toolboxAnchor = ref<HTMLElement | null>(null)
const taskPresets = ref<TaskPresetMap | null>(null)
const presetVersion = ref(0)

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

async function applyTaskPresets(presets: TaskPresetMap) {
  taskPresets.value = presets
  presetVersion.value += 1
  await nextTick()
  toolboxAnchor.value?.scrollIntoView?.({ behavior: 'smooth', block: 'start' })
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
