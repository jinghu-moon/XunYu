<script setup lang="ts">
import { ref } from 'vue'

const props = withDefaults(
  defineProps<{
    title: string
    collapsed?: boolean
  }>(),
  { collapsed: false },
)

const isCollapsed = ref(props.collapsed)

function toggle() {
  isCollapsed.value = !isCollapsed.value
}
</script>

<template>
  <div class="collapsible-panel" :class="{ 'collapsible-panel--collapsed': isCollapsed }">
    <button
      class="collapsible-panel__header"
      type="button"
      :aria-expanded="!isCollapsed"
      data-testid="panel-toggle"
      @click="toggle"
    >
      <span class="collapsible-panel__title">{{ title }}</span>
      <svg
        class="collapsible-panel__chevron"
        :class="{ 'collapsible-panel__chevron--collapsed': isCollapsed }"
        width="16"
        height="16"
        viewBox="0 0 16 16"
        fill="none"
        aria-hidden="true"
      >
        <path d="M4 6l4 4 4-4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" />
      </svg>
    </button>
    <div v-show="!isCollapsed" class="collapsible-panel__body" data-testid="panel-body">
      <slot />
    </div>
  </div>
</template>

<style scoped>
.collapsible-panel {
  border: var(--border);
  border-radius: var(--panel-radius);
  background: var(--surface-panel);
  overflow: hidden;
}

.collapsible-panel__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
  padding: var(--space-3) var(--space-4);
  background: none;
  border: none;
  cursor: pointer;
  color: var(--text-primary);
  font: var(--type-body-sm);
  font-weight: var(--weight-medium);
  text-align: left;
  transition: background var(--duration-fast) ease;
}

.collapsible-panel__header:hover {
  background: var(--surface-card-muted);
}

.collapsible-panel__chevron {
  transition: transform var(--duration-fast) ease;
  flex-shrink: 0;
}

.collapsible-panel__chevron--collapsed {
  transform: rotate(-90deg);
}

.collapsible-panel__body {
  padding: 0 var(--space-4) var(--space-4);
}
</style>
