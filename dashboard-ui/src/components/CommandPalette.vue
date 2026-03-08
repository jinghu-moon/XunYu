<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'

type CommandItem = {
  id: string
  label: string
  description?: string
  keywords?: string[]
  section?: string
  run: () => void
}

const props = defineProps<{
  modelValue: boolean
  commands: CommandItem[]
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: boolean): void
}>()

const query = ref('')
const activeIndex = ref(0)
const inputRef = ref<HTMLInputElement | null>(null)

const filtered = computed(() => {
  const q = query.value.trim().toLowerCase()
  if (!q) return props.commands
  const terms = q.split(/\s+/).filter(Boolean)
  return props.commands.filter(cmd => {
    const haystack = [
      cmd.label,
      cmd.description || '',
      cmd.section || '',
      ...(cmd.keywords || []),
    ]
      .join(' ')
      .toLowerCase()
    return terms.every(term => haystack.includes(term))
  })
})

watch(
  () => props.modelValue,
  open => {
    if (!open) return
    query.value = ''
    activeIndex.value = 0
    void nextTick(() => inputRef.value?.focus())
  },
)

watch(filtered, list => {
  if (!list.length) {
    activeIndex.value = 0
    return
  }
  if (activeIndex.value >= list.length) {
    activeIndex.value = 0
  }
})

function close() {
  emit('update:modelValue', false)
}

function onBackdropClick() {
  close()
}

function move(delta: number) {
  const list = filtered.value
  if (!list.length) return
  const next = (activeIndex.value + delta + list.length) % list.length
  activeIndex.value = next
}

function execute(cmd: CommandItem) {
  cmd.run()
  close()
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape') {
    e.preventDefault()
    close()
    return
  }
  if (e.key === 'ArrowDown') {
    e.preventDefault()
    move(1)
    return
  }
  if (e.key === 'ArrowUp') {
    e.preventDefault()
    move(-1)
    return
  }
  if (e.key === 'Enter') {
    e.preventDefault()
    const cmd = filtered.value[activeIndex.value]
    if (cmd) execute(cmd)
  }
}
</script>

<template>
  <Teleport to="body">
    <div v-if="modelValue" class="cmdk" role="dialog" aria-modal="true" @keydown="onKeydown">
      <div class="cmdk-backdrop" @click="onBackdropClick"></div>
      <div class="cmdk-panel">
        <div class="cmdk-header">
          <div class="cmdk-title">Command Palette</div>
          <div class="cmdk-hint">Esc 关闭 · Enter 执行</div>
        </div>
        <div class="cmdk-input-row">
          <input
            ref="inputRef"
            v-model="query"
            class="cmdk-input"
            placeholder="搜索命令或页面…"
            @keydown="onKeydown"
          />
        </div>
        <div class="cmdk-list">
          <button
            v-for="(cmd, idx) in filtered"
            :key="cmd.id"
            type="button"
            class="cmdk-item"
            :class="{ active: idx === activeIndex }"
            @mouseenter="activeIndex = idx"
            @click="execute(cmd)"
          >
            <div class="cmdk-item-main">
              <div class="cmdk-item-label">{{ cmd.label }}</div>
              <div v-if="cmd.description" class="cmdk-item-desc">{{ cmd.description }}</div>
            </div>
            <div v-if="cmd.section" class="cmdk-item-section">{{ cmd.section }}</div>
          </button>
          <div v-if="!filtered.length" class="cmdk-empty">没有匹配的命令</div>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.cmdk {
  position: fixed;
  inset: 0;
  z-index: var(--z-overlay);
  display: flex;
  align-items: flex-start;
  justify-content: center;
  padding: var(--space-8) var(--space-6);
}

.cmdk-backdrop {
  position: absolute;
  inset: 0;
  background: var(--gray-alpha-700);
}

.cmdk-panel {
  position: relative;
  z-index: var(--z-modal);
  width: 720px;
  max-height: 70vh;
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
  border-radius: var(--radius-lg);
  border: var(--card-border);
  background: var(--surface-panel);
  box-shadow: var(--shadow-md);
  padding: var(--space-5);
}

.cmdk-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-3);
}

.cmdk-title {
  font: var(--type-title);
  color: var(--text-primary);
}

.cmdk-hint {
  font: var(--type-caption);
  color: var(--text-tertiary);
}

.cmdk-input-row {
  display: flex;
  align-items: center;
  gap: var(--space-2);
}

.cmdk-input {
  width: 100%;
  padding: var(--comp-padding-md);
  border-radius: var(--radius-md);
  border: var(--border);
  background: var(--surface-card);
  color: var(--text-primary);
  font: var(--type-body);
  outline: none;
}

.cmdk-input:focus {
  border-color: var(--text-primary);
}

.cmdk-list {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  overflow: auto;
  padding-right: var(--space-1);
}

.cmdk-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-4);
  border-radius: var(--radius-md);
  border: 1px solid transparent;
  background: var(--surface-card);
  padding: var(--space-3) var(--space-4);
  color: var(--text-primary);
  cursor: pointer;
  text-align: left;
  transition: var(--transition-color);
}

.cmdk-item:hover,
.cmdk-item.active {
  border-color: var(--color-border-strong);
  background: var(--surface-card-muted);
}

.cmdk-item-main {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.cmdk-item-label {
  font: var(--type-title-sm);
}

.cmdk-item-desc {
  font: var(--type-caption);
  color: var(--text-tertiary);
}

.cmdk-item-section {
  font: var(--type-caption);
  color: var(--text-secondary);
  padding: 2px var(--space-2);
  border-radius: var(--radius-full);
  background: var(--surface-card-muted);
}

.cmdk-empty {
  padding: var(--space-4);
  border: var(--border);
  border-radius: var(--radius-md);
  text-align: center;
  color: var(--text-tertiary);
  background: var(--surface-card);
}
</style>
