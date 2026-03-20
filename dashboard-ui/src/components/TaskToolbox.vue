<script setup lang="ts">
import type { StatisticsWorkspaceLinkPayload, WorkspaceCapabilities } from '../types'
import type { WorkspaceTaskDefinition } from '../features/tasks'
import TaskToolCard from './TaskToolCard.vue'

const emit = defineEmits<{
  (event: 'link-panel', payload: StatisticsWorkspaceLinkPayload): void
}>()

const props = withDefaults(
  defineProps<{
    title: string
    description?: string
    tasks: WorkspaceTaskDefinition[]
    capabilities?: WorkspaceCapabilities | null
    taskPresets?: Record<string, Partial<Record<string, string | boolean>>> | null
    presetVersion?: number
  }>(),
  {
    description: '',
    capabilities: null,
    taskPresets: null,
    presetVersion: 0,
  },
)
</script>

<template>
  <section class="task-toolbox">
    <header class="task-toolbox__header">
      <div>
        <h3 class="task-toolbox__title">{{ props.title }}</h3>
        <p v-if="props.description" class="task-toolbox__desc">{{ props.description }}</p>
      </div>
    </header>
    <div class="task-toolbox__grid">
      <TaskToolCard
        v-for="task in props.tasks"
        :key="task.id"
        :task="task"
        :capabilities="props.capabilities"
        :initial-values="props.taskPresets?.[task.id] ?? null"
        :preset-version="props.presetVersion"
        @link-panel="emit('link-panel', $event)"
      />
    </div>
  </section>
</template>

<style scoped>
.task-toolbox {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}

.task-toolbox__title {
  font: var(--type-title);
  color: var(--text-primary);
  margin-bottom: var(--space-1);
}

.task-toolbox__desc {
  color: var(--text-secondary);
  font: var(--type-body-sm);
}

.task-toolbox__grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(320px, 1fr));
  gap: var(--space-4);
}
</style>
