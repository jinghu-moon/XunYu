<script setup lang="ts">
import { computed, ref } from 'vue'

interface Workspace {
  id: string
  label: string
}

const props = withDefaults(
  defineProps<{
    workspaces: Workspace[]
    active: string
    maxVisible?: number
  }>(),
  { maxVisible: 10 },
)

const emit = defineEmits<{
  change: [id: string]
}>()

const showOverflow = ref(false)

const visibleWorkspaces = computed(() =>
  props.workspaces.slice(0, props.maxVisible),
)

const overflowWorkspaces = computed(() =>
  props.workspaces.slice(props.maxVisible),
)

function select(id: string) {
  emit('change', id)
  showOverflow.value = false
}
</script>

<template>
  <nav class="workspace-nav">
    <button
      v-for="ws in visibleWorkspaces"
      :key="ws.id"
      data-testid="workspace-tab"
      :class="['tab', { active: ws.id === active }]"
      @click="select(ws.id)"
    >
      {{ ws.label }}
    </button>

    <div v-if="overflowWorkspaces.length" class="overflow-wrapper">
      <button
        data-testid="overflow-menu"
        class="tab"
        @click="showOverflow = !showOverflow"
      >
        +{{ overflowWorkspaces.length }}
      </button>
      <div v-if="showOverflow" class="overflow-dropdown">
        <button
          v-for="ws in overflowWorkspaces"
          :key="ws.id"
          :class="['dropdown-item', { active: ws.id === active }]"
          @click="select(ws.id)"
        >
          {{ ws.label }}
        </button>
      </div>
    </div>
  </nav>
</template>

<style scoped>
.workspace-nav {
  display: flex;
  align-items: center;
  gap: var(--space-1);
  padding: var(--space-2) var(--space-4);
  border-bottom: var(--border);
  overflow-x: auto;
}
.tab {
  padding: var(--space-2) var(--space-3);
  font-size: var(--text-sm);
  color: var(--text-secondary);
  background: transparent;
  border: none;
  border-radius: var(--radius-sm);
  cursor: pointer;
  white-space: nowrap;
}
.tab:hover {
  background: var(--ds-background-2);
  color: var(--text-primary);
}
.tab.active {
  color: var(--text-primary);
  background: var(--ds-background-2);
  font-weight: 600;
}
.overflow-wrapper {
  position: relative;
}
.overflow-dropdown {
  position: absolute;
  top: 100%;
  left: 0;
  z-index: 10;
  min-width: 160px;
  padding: var(--space-1);
  background: var(--ds-background-1);
  border: var(--border);
  border-radius: var(--radius-md);
  box-shadow: var(--shadow-md);
}
.dropdown-item {
  display: block;
  width: 100%;
  padding: var(--space-2) var(--space-3);
  font-size: var(--text-sm);
  color: var(--text-secondary);
  background: transparent;
  border: none;
  border-radius: var(--radius-sm);
  cursor: pointer;
  text-align: left;
}
.dropdown-item:hover {
  background: var(--ds-background-2);
  color: var(--text-primary);
}
.dropdown-item.active {
  color: var(--text-primary);
  font-weight: 600;
}
</style>
