<script setup lang="ts">
import type { EnvAuditEntry } from '../types'

const props = defineProps<{
  varName: string | null
  entries: EnvAuditEntry[]
  loading?: boolean
}>()

const emit = defineEmits<{
  (e: 'close'): void
}>()
</script>

<template>
  <Teleport to="body">
    <div v-if="varName" class="backdrop" @click="emit('close')" />
    <aside v-if="varName" class="drawer">
      <header class="drawer__header">
        <div>
          <h3>{{ varName }}</h3>
          <p>Variable history</p>
        </div>
        <button type="button" @click="emit('close')">Close</button>
      </header>
      <div class="drawer__body">
        <p v-if="loading" class="hint">Loading...</p>
        <p v-else-if="!entries.length" class="hint">No history entries.</p>
        <ul v-else class="timeline">
          <li v-for="(item, idx) in entries" :key="`${item.at}:${idx}`">
            <div class="meta">
              <span class="mono">{{ item.at }}</span>
              <span>{{ item.action }}</span>
              <span :class="item.result === 'ok' ? 'ok' : 'error'">{{ item.result }}</span>
            </div>
            <p class="msg mono">{{ item.message || '(no message)' }}</p>
          </li>
        </ul>
      </div>
    </aside>
  </Teleport>
</template>

<style scoped>
.backdrop {
  position: fixed;
  inset: 0;
  background: rgb(0 0 0 / 50%);
  z-index: 100;
}

.drawer {
  position: fixed;
  top: 0;
  right: 0;
  bottom: 0;
  width: min(560px, 95vw);
  background: var(--surface-panel);
  border-left: var(--border);
  z-index: 101;
  display: grid;
  grid-template-rows: auto 1fr;
}

.drawer__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-3);
  padding: var(--space-4);
  border-bottom: var(--border);
}

.drawer__header h3 {
  margin: 0;
  font: var(--type-title-sm);
}

.drawer__header p {
  margin: 0;
  color: var(--text-secondary);
  font: var(--type-body-xs);
}

.drawer__body {
  overflow: auto;
  padding: var(--space-3);
}

.timeline {
  list-style: none;
  margin: 0;
  padding: 0;
  display: grid;
  gap: var(--space-2);
}

.timeline li {
  border: var(--border);
  border-radius: var(--radius-sm);
  padding: var(--space-2);
  background: var(--surface-card);
}

.meta {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
  color: var(--text-secondary);
  font: var(--type-body-xs);
}

.msg {
  margin: var(--space-1) 0 0;
  white-space: pre-wrap;
  word-break: break-word;
}

.hint {
  color: var(--text-secondary);
}

.mono {
  font-family: var(--font-family-mono);
}

.ok {
  color: var(--state-success);
}

.error {
  color: var(--state-error);
}
</style>
