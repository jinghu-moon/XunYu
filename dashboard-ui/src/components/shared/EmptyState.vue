<script setup lang="ts">
withDefaults(
  defineProps<{
    message?: string
  }>(),
  { message: 'No data' },
)
</script>

<template>
  <div data-testid="empty-state" class="empty-state">
    <div v-if="$slots.icon" class="empty-icon">
      <slot name="icon" />
    </div>
    <p class="empty-message">{{ message }}</p>
    <div v-if="$slots.action" class="empty-action">
      <slot name="action" />
    </div>
  </div>
</template>

<style scoped>
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: var(--space-8) var(--space-4);
  color: var(--text-tertiary);
  text-align: center;
}
.empty-icon {
  margin-bottom: var(--space-3);
  font-size: 2rem;
}
.empty-message {
  margin: 0;
  font-size: var(--text-sm);
}
.empty-action {
  margin-top: var(--space-4);
}
</style>
