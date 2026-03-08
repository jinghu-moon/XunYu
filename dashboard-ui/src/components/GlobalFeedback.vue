<script setup lang="ts">
import { computed } from 'vue'
import { removeToast, useFeedbackState } from '../ui/feedback'

const state = useFeedbackState()
const isLoading = computed(() => state.loadingCount > 0)
</script>

<template>
  <div>
    <div v-if="isLoading" class="global-loading">
      <div class="spinner" />
      <div class="loading-text">Loading...</div>
    </div>
    <div class="toast-stack">
      <div v-for="t in state.toasts" :key="t.id" :class="['toast', `toast--${t.level}`]">
        <div class="toast-title">{{ t.title }}</div>
        <div v-if="t.detail" class="toast-detail">{{ t.detail }}</div>
        <button class="toast-close" type="button" @click="removeToast(t.id)">×</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.toast-stack {
  position: fixed;
  top: var(--space-4);
  right: var(--space-4);
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  z-index: var(--z-max);
}
.toast {
  position: relative;
  min-width: 260px;
  max-width: 420px;
  padding: var(--space-3) var(--space-4);
  border-radius: var(--radius-md);
  border: 1px solid var(--color-border-strong);
  background: var(--bg-toast, var(--ds-background-1));
  box-shadow: var(--shadow-sm);
  backdrop-filter: var(--glass-sm, none);
}
.toast--error {
  border-color: var(--color-danger);
}
.toast--warning {
  border-color: var(--color-warning);
}
.toast--info {
  border-color: var(--color-info);
}
.toast--success {
  border-color: var(--color-success);
}
.toast-title {
  font-size: var(--text-sm);
  font-weight: var(--weight-semibold);
  color: var(--text-primary);
  margin-bottom: var(--space-1);
}
.toast-detail {
  font-size: var(--text-xs);
  color: var(--text-secondary);
  white-space: pre-wrap;
  word-break: break-word;
}
.toast-close {
  position: absolute;
  top: var(--space-2);
  right: var(--space-2);
  border: none;
  background: transparent;
  color: var(--text-tertiary);
  cursor: pointer;
  font-size: 16px;
  line-height: 1;
}
.toast-close:hover {
  color: var(--text-primary);
}
.global-loading {
  position: fixed;
  bottom: var(--space-4);
  right: var(--space-4);
  display: inline-flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-2) var(--space-3);
  border-radius: var(--radius-full);
  border: var(--border);
  background: var(--bg-toast, var(--ds-background-1));
  box-shadow: var(--shadow-sm);
  z-index: var(--z-max);
}
.spinner {
  width: 14px;
  height: 14px;
  border-radius: 50%;
  border: 2px solid var(--text-tertiary);
  border-top-color: var(--text-primary);
  animation: spin 0.9s linear infinite;
}
.loading-text {
  font-size: var(--text-xs);
  color: var(--text-secondary);
}
@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}
</style>
