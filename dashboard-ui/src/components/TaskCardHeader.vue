<script setup lang="ts">
import { computed } from 'vue'
import type { TaskNotice } from '../features/tasks'

const props = withDefaults(
  defineProps<{
    title: string
    description: string
    stateLabel: string
    stateTone?: '' | 'is-ok' | 'is-error'
    feature?: string | null
    notices?: TaskNotice[] | null
  }>(),
  {
    stateTone: '',
    feature: null,
    notices: null,
  },
)

const visibleNotices = computed(() => props.notices ?? [])
</script>

<template>
  <section class="task-card-header">
    <header class="task-card-header__main">
      <div>
        <h4 class="task-card-header__title">{{ props.title }}</h4>
        <p class="task-card-header__desc">{{ props.description }}</p>
      </div>
      <div class="task-card-header__side">
        <span :class="['task-card-header__badge', props.stateTone]">{{ props.stateLabel }}</span>
        <span v-if="props.feature" class="task-card-header__feature">{{ props.feature }}</span>
      </div>
    </header>

    <div v-if="visibleNotices.length" class="task-card-header__notices">
      <span
        v-for="notice in visibleNotices"
        :key="notice.text"
        :class="['task-card-header__notice', notice.tone === 'warning' ? 'is-warning' : 'is-info']"
      >
        {{ notice.text }}
      </span>
    </div>
  </section>
</template>

<style scoped>
.task-card-header {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
}

.task-card-header__main {
  display: flex;
  justify-content: space-between;
  gap: var(--space-3);
}

.task-card-header__side {
  display: flex;
  align-items: flex-start;
  gap: var(--space-2);
  flex-wrap: wrap;
  justify-content: flex-end;
}

.task-card-header__title {
  font: var(--type-title-sm);
  color: var(--text-primary);
  margin-bottom: var(--space-1);
}

.task-card-header__desc {
  color: var(--text-secondary);
  font: var(--type-body-sm);
}

.task-card-header__feature {
  border-radius: var(--radius-full);
  background: var(--ds-background-2);
  padding: 2px var(--space-3);
  color: var(--text-secondary);
  font: var(--type-caption);
  height: fit-content;
}

.task-card-header__notices {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
}

.task-card-header__notice {
  border-radius: var(--radius-full);
  padding: 2px var(--space-3);
  border: var(--border);
  background: var(--ds-background-2);
  color: var(--text-secondary);
  font: var(--type-caption);
}

.task-card-header__notice.is-warning {
  border-color: rgba(242, 178, 94, 0.4);
  background: rgba(255, 233, 200, 0.6);
  color: #8a5200;
}

.task-card-header__notice.is-info {
  border-color: rgba(120, 150, 200, 0.35);
  background: rgba(210, 225, 245, 0.5);
  color: #2f4866;
}

.task-card-header__badge {
  display: inline-flex;
  align-items: center;
  padding: 2px var(--space-3);
  border-radius: var(--radius-full);
  font-weight: var(--weight-semibold);
  background: var(--ds-background-2);
  color: var(--text-secondary);
}

.task-card-header__badge.is-ok {
  background: var(--color-success-bg);
  color: var(--color-success);
}

.task-card-header__badge.is-error {
  background: var(--color-danger-bg);
  color: var(--color-danger);
}
</style>
