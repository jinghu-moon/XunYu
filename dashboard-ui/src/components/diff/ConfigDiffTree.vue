<script setup lang="ts">
import { computed, ref } from 'vue'
import type { ConfigDiffNode } from '../../types'

defineOptions({ name: 'ConfigDiffTree' })

const props = withDefaults(defineProps<{
  node: ConfigDiffNode
  level?: number
}>(), {
  level: 0,
})

const expanded = ref(props.level < 2)
const hasChildren = computed(() => (props.node.children?.length ?? 0) > 0)
const rowStyle = computed(() => ({
  paddingLeft: `${props.level * 14}px`,
}))
const statusText = computed(() => {
  if (props.node.status === 'added') return 'Added'
  if (props.node.status === 'removed') return 'Removed'
  if (props.node.status === 'modified') return 'Modified'
  return 'Unchanged'
})
const leafText = computed(() => {
  if (hasChildren.value) return ''
  const fmt = (value: unknown) => formatValue(value)
  if (props.node.status === 'modified') {
    return `${fmt(props.node.oldValue)} -> ${fmt(props.node.newValue)}`
  }
  if (props.node.status === 'added') return fmt(props.node.newValue)
  if (props.node.status === 'removed') return fmt(props.node.oldValue)
  return fmt(props.node.newValue ?? props.node.oldValue)
})

function formatValue(value: unknown): string {
  if (typeof value === 'string') {
    const trimmed = value.length > 72 ? `${value.slice(0, 72)}...` : value
    return `"${trimmed}"`
  }
  if (typeof value === 'number' || typeof value === 'boolean') return String(value)
  if (value == null) return 'null'
  try {
    const raw = JSON.stringify(value)
    if (!raw) return String(value)
    return raw.length > 80 ? `${raw.slice(0, 80)}...` : raw
  } catch {
    return String(value)
  }
}

function toggle() {
  if (!hasChildren.value) return
  expanded.value = !expanded.value
}
</script>

<template>
  <div class="cfg-node">
    <div class="cfg-row" :class="`cfg-row--${node.status}`" :style="rowStyle">
      <button class="cfg-toggle" :disabled="!hasChildren" @click="toggle">
        <template v-if="hasChildren">{{ expanded ? '▾' : '▸' }}</template>
        <template v-else>·</template>
      </button>
      <span class="cfg-key">{{ node.key || '(root)' }}</span>
      <span class="cfg-kind">{{ node.kind }}</span>
      <span class="cfg-status">{{ statusText }}</span>
      <span v-if="hasChildren" class="cfg-extra">{{ node.children?.length }} children</span>
      <span v-else class="cfg-value">{{ leafText }}</span>
    </div>

    <div v-if="hasChildren && expanded" class="cfg-children">
      <ConfigDiffTree
        v-for="child in node.children"
        :key="child.path || child.key"
        :node="child"
        :level="level + 1"
      />
    </div>
  </div>
</template>

<style scoped>
.cfg-node {
  display: flex;
  flex-direction: column;
}

.cfg-row {
  display: flex;
  align-items: center;
  gap: var(--space-1);
  min-height: 28px;
  border-bottom: var(--border);
  font: var(--type-body-sm);
}

.cfg-row--added {
  background: var(--color-success-bg);
}

.cfg-row--removed {
  background: var(--color-danger-bg);
}

.cfg-row--modified {
  background: var(--color-warning-bg);
}

.cfg-toggle {
  width: 18px;
  height: 18px;
  border: none;
  background: transparent;
  color: var(--text-secondary);
  cursor: pointer;
  padding: 0;
  line-height: 1;
}

.cfg-toggle:disabled {
  cursor: default;
  opacity: 0.7;
}

.cfg-key {
  color: var(--text-primary);
  font-family: var(--font-family-mono);
}

.cfg-kind {
  border: var(--border);
  border-radius: var(--radius-full);
  padding: 0 var(--space-1);
  color: var(--text-secondary);
  font: var(--type-caption);
  text-transform: uppercase;
}

.cfg-status {
  border: var(--border);
  border-radius: var(--radius-full);
  padding: 0 var(--space-1);
  color: var(--text-secondary);
  font: var(--type-caption);
}

.cfg-extra {
  margin-left: auto;
  color: var(--text-tertiary);
  font: var(--type-caption);
}

.cfg-value {
  margin-left: auto;
  color: var(--text-secondary);
  font: var(--type-caption);
  font-family: var(--font-family-mono);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 52%;
}

.cfg-children {
  display: flex;
  flex-direction: column;
}
</style>
