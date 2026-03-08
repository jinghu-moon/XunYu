<script setup lang="ts">
const props = defineProps<{
  title: string
  description: string
}>()
</script>

<template>
  <section class="workspace-frame">
    <header class="workspace-frame__header">
      <div>
        <h2 class="workspace-frame__title">{{ props.title }}</h2>
        <p class="workspace-frame__desc">{{ props.description }}</p>
      </div>
      <div v-if="$slots.summary" class="workspace-frame__summary">
        <slot name="summary" />
      </div>
    </header>
    <div class="workspace-frame__body">
      <slot />
    </div>
  </section>
</template>

<style scoped>
.workspace-frame {
  display: flex;
  flex-direction: column;
  gap: var(--space-5);
}

.workspace-frame__header {
  display: flex;
  justify-content: space-between;
  gap: var(--space-4);
  align-items: flex-start;
}

.workspace-frame__title {
  font: var(--type-title-lg);
  color: var(--text-primary);
  margin-bottom: var(--space-2);
}

.workspace-frame__desc {
  font: var(--type-body-sm);
  color: var(--text-secondary);
  max-width: 820px;
}

.workspace-frame__summary {
  min-width: 280px;
}

.workspace-frame__body {
  display: flex;
  flex-direction: column;
  gap: var(--space-5);
}
</style>
