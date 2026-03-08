<script setup lang="ts">
import { computed } from 'vue'

const props = withDefaults(defineProps<{ rows?: number; columns?: number }>(), {
  rows: 6,
  columns: 5,
})

const rowItems = computed(() => Array.from({ length: Math.max(1, props.rows) }, (_, i) => i))
const normalizedCols = computed(() => Math.min(8, Math.max(2, props.columns)))
const colItems = computed(() => Array.from({ length: normalizedCols.value }, (_, i) => i))
const gridClass = computed(() => `cols-${normalizedCols.value}`)
</script>

<template>
  <div class="skeleton-table" :class="gridClass">
    <div v-for="r in rowItems" :key="r" class="skeleton-row">
      <div v-for="c in colItems" :key="c" class="skeleton-cell" />
    </div>
  </div>
</template>

<style scoped>
.skeleton-table {
  display: grid;
  gap: var(--space-2);
}

.skeleton-row {
  display: grid;
  gap: var(--space-2);
}

.skeleton-cell {
  position: relative;
  height: var(--space-4);
  border-radius: var(--radius-sm);
  background: var(--ds-background-2);
  overflow: hidden;
}

.skeleton-cell::after {
  content: '';
  position: absolute;
  inset: 0;
  background: linear-gradient(
    90deg,
    var(--ds-background-2) 0%,
    var(--ds-background-1) 50%,
    var(--ds-background-2) 100%
  );
  transform: translateX(-100%);
  animation: skeleton-shimmer 1.2s ease-in-out infinite;
}

.cols-2 .skeleton-row { grid-template-columns: repeat(2, minmax(0, 1fr)); }
.cols-3 .skeleton-row { grid-template-columns: repeat(3, minmax(0, 1fr)); }
.cols-4 .skeleton-row { grid-template-columns: repeat(4, minmax(0, 1fr)); }
.cols-5 .skeleton-row { grid-template-columns: repeat(5, minmax(0, 1fr)); }
.cols-6 .skeleton-row { grid-template-columns: repeat(6, minmax(0, 1fr)); }
.cols-7 .skeleton-row { grid-template-columns: repeat(7, minmax(0, 1fr)); }
.cols-8 .skeleton-row { grid-template-columns: repeat(8, minmax(0, 1fr)); }

@keyframes skeleton-shimmer {
  0% { transform: translateX(-100%); }
  100% { transform: translateX(100%); }
}
</style>
