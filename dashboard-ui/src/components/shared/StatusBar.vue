<script setup lang="ts">
import { useWsStore } from '../../stores/ws'

defineProps<{
  lastOperation?: string
}>()

const ws = useWsStore()
</script>

<template>
  <footer class="status-bar">
    <span data-testid="ws-status" class="ws-status">
      <span :class="['dot', ws.isConnected ? 'dot--connected' : 'dot--disconnected']" />
      {{ ws.isConnected ? 'Connected' : 'Disconnected' }}
    </span>
    <span v-if="lastOperation" data-testid="last-op" class="last-op">
      Last: {{ lastOperation }}
    </span>
  </footer>
</template>

<style scoped>
.status-bar {
  display: flex;
  align-items: center;
  gap: var(--space-4);
  padding: var(--space-2) var(--space-4);
  font-size: var(--text-xs);
  color: var(--text-secondary);
  border-top: var(--border);
  background: var(--ds-background-2);
}
.ws-status {
  display: inline-flex;
  align-items: center;
  gap: var(--space-1);
}
.dot {
  display: inline-block;
  width: 8px;
  height: 8px;
  border-radius: 50%;
}
.dot--connected {
  background: var(--color-success);
}
.dot--disconnected {
  background: var(--color-danger);
}
.last-op {
  color: var(--text-tertiary);
}
</style>
