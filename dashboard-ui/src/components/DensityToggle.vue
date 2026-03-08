<script setup lang="ts">
import { onMounted, ref, watch } from 'vue'

const density = ref<'compact' | 'spacious'>('compact')

const options = [
  { value: 'compact' as const, label: 'Compact' },
  { value: 'spacious' as const, label: 'Spacious' },
]

function applyDensity(value: 'compact' | 'spacious') {
  const root = document.documentElement
  root.classList.toggle('density-compact', value === 'compact')
  root.classList.toggle('density-spacious', value === 'spacious')
}

onMounted(() => {
  const saved = localStorage.getItem('densityPreference')
  if (saved === 'compact' || saved === 'spacious') {
    density.value = saved
  }
  applyDensity(density.value)
})

watch(density, value => {
  localStorage.setItem('densityPreference', value)
  applyDensity(value)
})
</script>

<template>
  <div class="density-toggle" title="Table density">
    <span class="density-label">Density</span>
    <div class="density-group" role="group" aria-label="Table density">
      <button
        v-for="opt in options"
        :key="opt.value"
        type="button"
        class="density-btn"
        :class="{ active: density === opt.value }"
        @click="density = opt.value"
      >
        {{ opt.label }}
      </button>
    </div>
  </div>
</template>

<style scoped>
.density-toggle {
  display: inline-flex;
  align-items: center;
  gap: var(--space-2);
  padding: 2px var(--space-2);
  border: var(--border);
  border-radius: var(--radius-full);
  background: var(--surface-card);
}

.density-label {
  font: var(--type-caption);
  color: var(--text-tertiary);
  letter-spacing: var(--letter-spacing-wide);
  text-transform: uppercase;
}

.density-group {
  display: inline-flex;
  align-items: center;
  gap: 2px;
  background: var(--surface-card-muted);
  border-radius: var(--radius-full);
  padding: 2px;
}

.density-btn {
  border: 1px solid transparent;
  background: transparent;
  color: var(--text-secondary);
  font: var(--type-caption);
  padding: 2px var(--space-2);
  border-radius: var(--radius-full);
  cursor: pointer;
  transition: var(--transition-color);
}

.density-btn:hover,
.density-btn:focus-visible {
  border-color: var(--color-border-strong);
  color: var(--text-primary);
  outline: none;
}

.density-btn.active {
  background: var(--surface-panel);
  border-color: var(--color-border-strong);
  color: var(--text-primary);
}
</style>
