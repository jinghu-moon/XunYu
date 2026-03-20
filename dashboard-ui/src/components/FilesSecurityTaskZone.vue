<script setup lang="ts">
import type { StatisticsWorkspaceLinkPayload, WorkspaceCapabilities } from '../types'
import { filesSecurityTaskGroups } from '../features/tasks'
import type { TaskPresetMap } from '../features/workspaces/task-presets'
import TaskToolbox from './TaskToolbox.vue'

const props = withDefaults(
  defineProps<{
    capabilities?: WorkspaceCapabilities | null
    taskPresets: TaskPresetMap
    presetVersion: number
  }>(),
  {
    capabilities: null,
  },
)

const emit = defineEmits<{
  (event: 'link-panel', payload: StatisticsWorkspaceLinkPayload): void
}>()

const TEXT = {
  title: '\u6587\u4ef6\u64cd\u4f5c\u4efb\u52a1\u533a',
  description:
    '\u5148\u5728\u4e0a\u65b9\u9501\u5b9a\u76ee\u5f55 / \u6587\u4ef6\uff0c\u518d\u901a\u8fc7\u201c\u540c\u6b65\u201d\u628a\u4e0a\u4e0b\u6587\u5e26\u5165\u4efb\u52a1\u5361\uff1b\u9ad8\u98ce\u9669\u52a8\u4f5c\u7ee7\u7eed\u7531\u7edf\u4e00\u786e\u8ba4\u5f39\u7a97\u548c\u56de\u6267\u7ec4\u4ef6\u515c\u5e95\u3002',
} as const
</script>

<template>
  <section class="files-security-task-zone">
    <header class="files-security-task-zone__header">
      <div>
        <h3 class="files-security-task-zone__title">{{ TEXT.title }}</h3>
        <p class="files-security-task-zone__desc">{{ TEXT.description }}</p>
      </div>
    </header>

    <TaskToolbox
      v-for="group in filesSecurityTaskGroups"
      :key="group.id"
      :title="group.title"
      :description="group.description"
      :tasks="group.tasks"
      :capabilities="props.capabilities"
      :task-presets="props.taskPresets"
      :preset-version="props.presetVersion"
      @link-panel="emit('link-panel', $event)"
    />
  </section>
</template>

<style scoped>
.files-security-task-zone {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}

.files-security-task-zone__header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: var(--space-3);
}

.files-security-task-zone__title {
  font: var(--type-title-sm);
  color: var(--text-primary);
}

.files-security-task-zone__desc {
  color: var(--text-secondary);
  font: var(--type-body-sm);
}
</style>
