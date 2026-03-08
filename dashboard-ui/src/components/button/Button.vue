<script setup lang="ts">
import { computed } from 'vue'
import type { ButtonProps } from './types'

const props = withDefaults(defineProps<ButtonProps>(), {
  preset: 'secondary',
  size: 'md',
  square: false,
  disabled: false,
  loading: false,
  type: 'button',
})

const isDisabled = computed(() => props.disabled || props.loading)
</script>

<template>
  <button
    :type="props.type"
    :disabled="isDisabled"
    :aria-busy="props.loading ? 'true' : 'false'"
    :class="[
      'btn',
      `btn--${props.preset}`,
      `btn--${props.size}`,
      { 'btn--square': props.square, 'btn--loading': props.loading },
    ]"
  >
    <span v-if="props.loading" class="btn__spinner" aria-hidden="true"></span>
    <span class="btn__content">
      <slot />
    </span>
  </button>
</template>

<style scoped>
.btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: var(--space-2);
  border: var(--border);
  border-radius: var(--radius-md);
  font-family: var(--font-family-base);
  font-weight: var(--weight-medium);
  cursor: pointer;
  user-select: none;
  white-space: nowrap;
  transition: var(--transition-color);
  outline: none;
}

.btn--loading {
  cursor: progress;
}

.btn__content {
  display: inline-flex;
  align-items: center;
  gap: var(--space-2);
}

.btn--loading .btn__content {
  opacity: 0.6;
}

.btn__spinner {
  width: var(--icon-sm);
  height: var(--icon-sm);
  border-radius: var(--radius-full);
  border: 2px solid var(--text-secondary);
  border-top-color: transparent;
  animation: btn-spin var(--duration-normal) linear infinite;
}

.btn--confirm {
  position: relative;
}

:slotted(.btn__confirm-badge) {
  position: absolute;
  top: calc(var(--space-1) * -1);
  right: calc(var(--space-1) * -1);
  min-width: var(--space-4);
  height: var(--space-4);
  padding: 0 var(--space-1);
  border-radius: var(--radius-full);
  background: var(--color-danger);
  color: var(--text-primary);
  font-size: var(--text-xs);
  display: inline-flex;
  align-items: center;
  justify-content: center;
  box-shadow: var(--shadow-xs);
}

.btn:active:not(:disabled) {
  transform: scale(0.95);
}

.btn:focus-visible {
  outline: var(--focus-ring-width) solid var(--text-primary);
  outline-offset: var(--focus-ring-offset);
}

.btn:disabled {
  opacity: var(--disabled-opacity);
  cursor: not-allowed;
}

/* ---- Sizes ---- */
.btn--sm {
  height: var(--height-sm);
  padding: var(--comp-padding-xs);
  font-size: var(--text-xs);
}
.btn--md {
  height: var(--height-md);
  padding: var(--comp-padding-sm);
  font-size: var(--text-sm);
}
.btn--lg {
  height: var(--height-lg);
  padding: var(--comp-padding-md);
  font-size: var(--text-base);
}

/* Square (icon-only) */
.btn--square.btn--sm { width: var(--height-sm); padding: 0; }
.btn--square.btn--md { width: var(--height-md); padding: 0; }
.btn--square.btn--lg { width: var(--height-lg); padding: 0; }

/* ---- Presets ---- */

/* Secondary: transparent bg, secondary border, hover border → primary */
.btn--secondary {
  background: var(--ds-background-1);
  color: var(--text-primary);
  border-color: var(--ds-color-5);
}
.btn--secondary:hover:not(:disabled) {
  border-color: var(--text-primary);
}

/* Primary: theme-color filled, hover lighter */
.btn--primary {
  background: var(--text-primary);
  color: var(--ds-background-1);
  border-color: var(--text-primary);
}
.btn--primary:hover:not(:disabled) {
  background: var(--ds-color-9);
  border-color: var(--ds-color-9);
}

/* Danger: red accent */
.btn--danger {
  background: transparent;
  color: var(--color-danger);
  border-color: var(--color-danger);
}
.btn--danger:hover:not(:disabled) {
  background: var(--red-100);
}

/* Ghost: no border */
.btn--ghost {
  background: transparent;
  color: var(--text-secondary);
  border-color: transparent;
}
.btn--ghost:hover:not(:disabled) {
  background: var(--ds-background-2);
  color: var(--text-primary);
}

@keyframes btn-spin {
  to {
    transform: rotate(360deg);
  }
}
</style>
