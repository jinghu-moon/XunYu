<script setup lang="ts">
import { computed } from 'vue'

const props = withDefaults(defineProps<{
  added: number
  removed: number
  modified: number
  unchanged: number
  unitLabel?: string
}>(), {
  unitLabel: 'items',
})

const total = computed(() => props.added + props.removed + props.modified + props.unchanged)
</script>

<template>
  <div class="ds">
    <span class="ds-chip ds-chip--add">+ {{ added }}</span>
    <span class="ds-chip ds-chip--remove">- {{ removed }}</span>
    <span class="ds-chip ds-chip--mod">~ {{ modified }}</span>
    <span class="ds-chip ds-chip--same">= {{ unchanged }}</span>
    <span class="ds-total">{{ total }} {{ unitLabel }}</span>
  </div>
</template>

<style scoped>
.ds {
  display: inline-flex;
  align-items: center;
  gap: var(--space-1);
  flex-wrap: wrap;
}

.ds-chip {
  display: inline-flex;
  align-items: center;
  border-radius: var(--radius-full);
  border: var(--border);
  padding: 2px var(--space-2);
  font: var(--type-caption);
}

.ds-chip--add {
  color: var(--color-success);
  background: var(--color-success-bg);
}

.ds-chip--remove {
  color: var(--color-danger);
  background: var(--color-danger-bg);
}

.ds-chip--mod {
  color: var(--color-warning);
  background: var(--color-warning-bg);
}

.ds-chip--same {
  color: var(--text-secondary);
  background: var(--surface-card-muted);
}

.ds-total {
  margin-left: var(--space-1);
  color: var(--text-secondary);
  font: var(--type-caption);
  font-family: var(--font-family-mono);
}
</style>
