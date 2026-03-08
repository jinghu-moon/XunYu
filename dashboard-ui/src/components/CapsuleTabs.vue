<script setup lang="ts">
import { ref, computed, onMounted, watch, nextTick, onUnmounted } from 'vue'

export interface CapsuleTabItem {
  value: string | number
  label: string
  icon?: any
}

const props = withDefaults(defineProps<{
  modelValue: string | number
  items: CapsuleTabItem[]
  equalWidth?: boolean
  stretch?: boolean
  layout?: 'flex' | 'grid'
  gridCols?: number
}>(), {
  equalWidth: false,
  stretch: false,
  layout: 'flex',
  gridCols: 2
})

const emit = defineEmits<{
  (e: 'update:modelValue', value: string | number): void
}>()

const tabsWrapperRef = ref<HTMLElement | null>(null)
const tabsRef = ref<HTMLElement | null>(null)
const tabRefs = ref<HTMLElement[]>([])
const indicatorStyle = ref({
  width: '0px',
  height: '0px',
  transform: 'translate(0px, 0px)',
  opacity: 0,
  transition: ''
})

const activeIndex = computed(() => {
  return props.items.findIndex(item => item.value === props.modelValue)
})

const gridStyle = computed(() => {
  if (props.layout === 'grid') {
    return {
      display: 'grid',
      gridTemplateColumns: `repeat(${props.gridCols}, 1fr)`
    }
  }
  return {}
})

// Max width calculation for equal width mode
const maxTabWidth = ref(0)
const updateMaxTabWidth = () => {
  if (!props.equalWidth || props.layout === 'grid') return

  let max = 0
  tabRefs.value.forEach(tab => {
    if (tab) {
      const originalWidth = tab.style.width
      tab.style.width = ''
      const w = tab.getBoundingClientRect().width
      if (w > max) max = w
      tab.style.width = originalWidth
    }
  })
  maxTabWidth.value = max
}

const updateIndicator = (enableTransition = true) => {
  const container = tabsRef.value
  if (!container || activeIndex.value === -1) return

  const activeTab = tabRefs.value[activeIndex.value]
  if (!activeTab) return

  // Temporarily disable transition if requested
  const transitionStyle = enableTransition
    ? 'transform 0.35s cubic-bezier(0.34, 1.25, 0.64, 1), width 0.35s cubic-bezier(0.34, 1.25, 0.64, 1), height 0.35s cubic-bezier(0.34, 1.25, 0.64, 1)'
    : 'none'

  indicatorStyle.value = {
    width: `${activeTab.offsetWidth}px`,
    height: `${activeTab.offsetHeight}px`,
    transform: `translate(${activeTab.offsetLeft}px, ${activeTab.offsetTop}px)`,
    opacity: 1,
    transition: transitionStyle
  }

  // Restore transition in next frame if it was disabled
  if (!enableTransition) {
    requestAnimationFrame(() => {
      indicatorStyle.value.transition = 'transform 0.35s cubic-bezier(0.34, 1.25, 0.64, 1), width 0.35s cubic-bezier(0.34, 1.25, 0.64, 1), height 0.35s cubic-bezier(0.34, 1.25, 0.64, 1)'
    })
  }

  scrollActiveIntoView(activeTab)
}

const scrollActiveIntoView = (tab: HTMLElement) => {
  const wrapper = tabsWrapperRef.value
  if (!wrapper) return

  const tabLeft = tab.offsetLeft
  const tabTop = tab.offsetTop
  const tabWidth = tab.offsetWidth
  const tabHeight = tab.offsetHeight
  const wrapperWidth = wrapper.offsetWidth
  const wrapperHeight = wrapper.clientHeight

  if (props.layout === 'grid') {
    const scrollTop = tabTop - wrapperHeight / 2 + tabHeight / 2
    wrapper.scrollTo({ top: scrollTop, behavior: 'smooth' })
  } else {
    // Horizontal scroll
    const scrollTo = tabLeft - wrapperWidth / 2 + tabWidth / 2
    wrapper.scrollTo({ left: scrollTo, behavior: 'smooth' })
  }
}

const handleTabClick = (value: string | number) => {
  emit('update:modelValue', value)
}

// Keyboard Navigation
const handleKeyDown = (e: KeyboardEvent) => {
  const navKeys = ['ArrowRight', 'ArrowLeft', 'ArrowUp', 'ArrowDown']
  if (!navKeys.includes(e.key)) return

  e.preventDefault()

  const len = props.items.length
  if (len === 0) return

  let newIndex = activeIndex.value
  const cols = props.gridCols || 2

  if (props.layout === 'grid') {
    if (e.key === 'ArrowRight') newIndex = newIndex + 1
    if (e.key === 'ArrowLeft') newIndex = newIndex - 1
    if (e.key === 'ArrowDown') newIndex = newIndex + cols
    if (e.key === 'ArrowUp') newIndex = newIndex - cols
  } else {
    if (e.key === 'ArrowRight') newIndex = (newIndex + 1) % len
    if (e.key === 'ArrowLeft') newIndex = (newIndex - 1 + len) % len
  }

  // Boundary check
  if (newIndex >= 0 && newIndex < len) {
    emit('update:modelValue', props.items[newIndex].value)
  }
}

// Watchers
watch(
  () => props.modelValue,
  async () => {
    await nextTick()
    updateIndicator(true)
  }
)

watch(
  [() => props.equalWidth, () => props.layout, () => props.gridCols, () => props.items],
  async () => {
    await nextTick()
    updateMaxTabWidth()
    updateIndicator(false) // No transition when layout changes
  },
  { deep: true }
)

let resizeObserver: ResizeObserver | null = null

onMounted(async () => {
  await nextTick()
  updateMaxTabWidth()
  updateIndicator(false)

  if (tabsRef.value) {
    resizeObserver = new ResizeObserver(() => {
      requestAnimationFrame(() => {
        updateMaxTabWidth()
        updateIndicator(false)
      })
    })
    resizeObserver.observe(tabsRef.value)
  }
})

onUnmounted(() => {
  if (resizeObserver) {
    resizeObserver.disconnect()
    resizeObserver = null
  }
})
</script>

<template>
  <div class="capsule-tabs-wrapper" ref="tabsWrapperRef" style="margin-right: var(--space-4);">
    <div class="capsule-tabs" ref="tabsRef" role="tablist" tabindex="0"
      :class="{ 'layout-grid': layout === 'grid', 'layout-stretch': stretch }" :style="gridStyle"
      @keydown="handleKeyDown">
      <div class="capsule-indicator" :style="indicatorStyle"></div>

      <div v-for="item in items" :key="item.value" ref="tabRefs" class="capsule-tab"
        :class="{ active: modelValue === item.value }" role="tab" :aria-selected="modelValue === item.value"
        :tabindex="modelValue === item.value ? 0 : -1"
        :style="(equalWidth && layout !== 'grid') ? { width: `${maxTabWidth}px` } : {}"
        @click="handleTabClick(item.value)">
        <div class="tab-content">
          <component v-if="item.icon" :is="item.icon" class="icon" />
          <span>{{ item.label }}</span>
        </div>
        <!-- Ghost element for spacing stability -->
        <div class="tab-ghost" aria-hidden="true">
          <component v-if="item.icon" :is="item.icon" class="icon" />
          <span>{{ item.label }}</span>
        </div>
      </div>
      <!-- Slot for custom items -->
      <slot name="extra" />
    </div>
  </div>
</template>

<style scoped>
.capsule-tabs-wrapper {
  transition: max-width 0.3s, max-height 0.3s;
  background: var(--ds-background-2);
  /* Adapted for Geist */
  border-radius: var(--radius-md);
  /* Adapted for Geist */
  border: var(--border);
  /* Adapted for Geist */
  padding: var(--space-1);
  /* Adapted for Geist */
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

/* Horizontal Scroll Mode */
.capsule-tabs-wrapper:not(:has(.layout-grid)) {
  overflow-x: auto;
  overflow-y: hidden;
}

/* Vertical Scroll Mode (Grid) */
.capsule-tabs-wrapper:has(.layout-grid) {
  overflow-y: auto;
  overflow-x: hidden;
  max-height: 400px;
}

/* Scrollbar Styles */
.capsule-tabs-wrapper::-webkit-scrollbar {
  width: 6px;
  height: 6px;
}

.capsule-tabs-wrapper::-webkit-scrollbar-track {
  background: transparent;
  margin: 6px 0;
}

.capsule-tabs-wrapper::-webkit-scrollbar-thumb {
  background-color: transparent;
  border-radius: var(--radius-full);
  border: 1px solid transparent;
  background-clip: content-box;
}

.capsule-tabs-wrapper:hover::-webkit-scrollbar-thumb {
  background-color: var(--gray-alpha-400);
}

.capsule-tabs-wrapper::-webkit-scrollbar-thumb:hover {
  background-color: var(--gray-alpha-600);
}

.capsule-tabs-wrapper::-webkit-scrollbar-button {
  display: none !important;
  width: 0 !important;
  height: 0 !important;
  background: transparent !important;
}

.capsule-tabs {
  position: relative;
  display: inline-flex;
  background: transparent;
  border: none;
  padding: 0;
  transition: gap 0.3s;
  box-sizing: border-box;
  outline: none;
  gap: var(--space-1);
  /* Adapted for Geist */
}

.capsule-tabs.layout-grid {
  display: grid;
  gap: var(--space-1);
  width: 100%;
}

.capsule-tabs.layout-stretch {
  display: flex;
  width: 100%;
}

.capsule-tabs.layout-stretch .capsule-tab {
  flex: 1;
  justify-content: center;
}

.capsule-indicator {
  position: absolute;
  top: 0;
  left: 0;
  background: var(--ds-background-1);
  /* Adapted for Geist */
  border-radius: var(--radius-sm);
  /* Adapted for Geist */
  box-shadow: var(--shadow-sm);
  /* Adapted for Geist */
  z-index: 0;
  pointer-events: none;
}

.capsule-tab {
  position: relative;
  z-index: 1;
  padding: var(--space-1) var(--space-4);
  /* Adapted for Geist */
  min-width: 40px;
  cursor: pointer;
  user-select: none;
  display: inline-grid;
  grid-template-areas: "stack";
  place-items: center;
  color: var(--text-secondary);
  /* Adapted for Geist */
  font-size: var(--text-sm);
  /* Adapted for Geist */
  border-radius: var(--radius-sm);
  /* Adapted for Geist */
  transition: color var(--duration-fast);
  /* Adapted for Geist */
}

.capsule-tabs.layout-grid .capsule-tab {
  width: 100%;
  padding: var(--space-2) 0;
  /* Adapted for Geist */
}

.capsule-tab:focus-visible {
  outline: none;
}

.capsule-tab.active {
  color: var(--text-primary);
  /* Adapted for Geist */
  font-weight: 600;
  /* Adapted for Geist */
  text-shadow: 0 0 0.5px currentColor;
}

.tab-content {
  grid-area: stack;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: var(--space-1);
  /* Adapted for Geist */
  white-space: nowrap;
}

.tab-ghost {
  grid-area: stack;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: var(--space-1);
  /* Adapted for Geist */
  font-weight: 600;
  visibility: hidden;
  pointer-events: none;
  opacity: 0;
}

.icon {
  width: 18px;
  height: 18px;
  fill: currentColor;
}
</style>
