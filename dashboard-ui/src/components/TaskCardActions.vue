<script setup lang="ts">
import { computed } from 'vue'
import { Button } from './button'

const props = withDefaults(
  defineProps<{
    tone?: 'default' | 'danger'
    actionLabel: string
    disabled: boolean
    loading: boolean
    hintText?: string
    hintTone?: 'default' | 'error'
    failureHint?: string
  }>(),
  {
    tone: 'default',
    hintText: '',
    hintTone: 'default',
    failureHint: '',
  },
)

const emit = defineEmits<{
  (event: 'trigger'): void
}>()

const buttonPreset = computed(() => (props.tone === 'danger' ? 'danger' : 'primary'))
</script>

<template>
  <section class="task-card-actions-panel">
    <div class="task-card-actions-panel__row">
      <Button
        data-testid="task-card-action-trigger"
        :preset="buttonPreset"
        :disabled="props.disabled"
        :loading="props.loading"
        @click="emit('trigger')"
      >
        {{ props.actionLabel }}
      </Button>
      <span
        v-if="props.hintText"
        :class="[
          'task-card-actions-panel__hint',
          props.hintTone === 'error' ? 'task-card-actions-panel__hint--error' : '',
        ]"
      >
        {{ props.hintText }}
      </span>
    </div>

    <div
      v-if="props.failureHint"
      class="task-card-actions-panel__hint task-card-actions-panel__hint--warn"
    >
      {{ props.failureHint }}
    </div>
  </section>
</template>

<style scoped>
.task-card-actions-panel {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}

.task-card-actions-panel__row {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  flex-wrap: wrap;
}

.task-card-actions-panel__hint {
  color: var(--text-tertiary);
  font: var(--type-caption);
}

.task-card-actions-panel__hint--error {
  color: var(--color-danger);
}

.task-card-actions-panel__hint--warn {
  color: #8a5200;
}
</style>
