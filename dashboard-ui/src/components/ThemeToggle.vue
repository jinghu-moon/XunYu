<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount, watch } from 'vue'
import { IconSun, IconMoon, IconDeviceDesktop, IconCheck } from '@tabler/icons-vue'

const themeItems = [
  { value: 'system', label: 'System', icon: IconDeviceDesktop },
  { value: 'light', label: 'Light', icon: IconSun },
  { value: 'dark', label: 'Dark', icon: IconMoon }
]

const currentThemeStatus = ref<'system'|'light'|'dark'>('system')
const theme = ref<'light'|'dark'>('dark')
const showThemeMenu = ref(false)
let mediaQuery: MediaQueryList | null = null

function onSystemThemeChange() {
  if (currentThemeStatus.value !== 'system') return
  applyTheme('system')
}

onMounted(() => {
  const saved = localStorage.getItem('themePreference') as 'system' | 'light' | 'dark' | null
  if (saved) {
    currentThemeStatus.value = saved
  }
  applyTheme(currentThemeStatus.value)

  // 点击外部关闭菜单
  window.addEventListener('click', closeMenuOutside)

  mediaQuery = window.matchMedia('(prefers-color-scheme: light)')
  if (mediaQuery.addEventListener) {
    mediaQuery.addEventListener('change', onSystemThemeChange)
  } else {
    mediaQuery.addListener(onSystemThemeChange)
  }
})

onBeforeUnmount(() => {
  window.removeEventListener('click', closeMenuOutside)
  if (!mediaQuery) return
  if (mediaQuery.removeEventListener) {
    mediaQuery.removeEventListener('change', onSystemThemeChange)
  } else {
    mediaQuery.removeListener(onSystemThemeChange)
  }
})

watch(currentThemeStatus, (newVal) => {
  localStorage.setItem('themePreference', newVal)
  applyTheme(newVal)
})

function closeMenuOutside(e: MouseEvent) {
  const target = e.target as HTMLElement
  if (!target.closest('.header-actions')) {
    showThemeMenu.value = false
  }
}

function applyTheme(pref: 'system'|'light'|'dark', event?: MouseEvent) {
  let isLight = false
  if (pref === 'system') {
    isLight = window.matchMedia('(prefers-color-scheme: light)').matches
  } else {
    isLight = pref === 'light'
  }
  
  const willBeDark = !isLight
  const currentIsDark = theme.value === 'dark'
  
  if (
    document.startViewTransition && 
    !window.matchMedia('(prefers-reduced-motion: reduce)').matches && 
    event && 
    willBeDark !== currentIsDark
  ) {
    const x = event.clientX ?? innerWidth / 2
    const y = event.clientY ?? innerHeight / 2
    const endRadius = Math.hypot(Math.max(x, innerWidth - x), Math.max(y, innerHeight - y))
    
    const transition = document.startViewTransition(() => {
      executeThemeDOMUpdate(isLight, pref)
    })
    
    transition.ready.then(() => {
      document.documentElement.animate(
        {
          clipPath: [
            `circle(0px at ${x}px ${y}px)`,
            `circle(${endRadius}px at ${x}px ${y}px)`
          ]
        },
        {
          duration: 500,
          easing: 'cubic-bezier(0.4, 0, 0.2, 1)',
          pseudoElement: '::view-transition-new(root)'
        }
      )
    })
  } else {
    executeThemeDOMUpdate(isLight, pref)
  }
}

function executeThemeDOMUpdate(isLight: boolean, pref: 'system'|'light'|'dark') {
  theme.value = isLight ? 'light' : 'dark'
  currentThemeStatus.value = pref
  
  if (isLight) {
    document.documentElement.classList.add('light')
  } else {
    document.documentElement.classList.remove('light')
  }
}

function selectTheme(val: 'system'|'light'|'dark', e: MouseEvent) {
  showThemeMenu.value = false
  if (val !== currentThemeStatus.value) {
    localStorage.setItem('themePreference', val)
    applyTheme(val, e)
  }
}

const currentThemeIcon = computed(() => {
  return themeItems.find(i => i.value === currentThemeStatus.value)?.icon || IconDeviceDesktop
})
</script>

<template>
  <div class="header-actions">
    <button class="theme-toggle-trigger" @click="showThemeMenu = !showThemeMenu">
      <component :is="currentThemeIcon" :size="18" />
    </button>
    <div class="theme-menu" :class="{ show: showThemeMenu }">
        <button 
          v-for="item in themeItems" 
          :key="item.value"
          class="theme-option" 
          :class="{ active: currentThemeStatus === item.value }"
          @click="(e) => selectTheme(item.value as any, e)"
        >
          <div class="option-content">
            <component :is="item.icon" :size="16" />
            <span>{{ item.label }}</span>
          </div>
          <IconCheck v-if="currentThemeStatus === item.value" :size="16" />
        </button>
    </div>
  </div>
</template>

<style scoped>
.header-actions {
  position: relative;
  z-index: 100;
}

.theme-toggle-trigger {
  background: var(--ds-background-1);
  border: var(--border);
  color: var(--text-primary);
  width: var(--height-sm);
  height: var(--height-sm);
  border-radius: var(--radius-full);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: var(--transition-fast);
}

.theme-toggle-trigger:hover {
  background: var(--ds-background-2);
}

.theme-menu {
  position: absolute;
  top: calc(100% + var(--space-2));
  right: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
  background: var(--ds-background-1);
  border: var(--border);
  border-radius: var(--radius-md);
  box-shadow: var(--shadow-md);
  padding: var(--space-1);
  min-width: 130px;
  z-index: 100;
  
  /* --- 动画核心设置 --- */
  transform-origin: top right;
  opacity: 0;
  transform: scale(0.95) translateY(-10px);
  pointer-events: none;
  transition: var(--transition-popup);
}

.theme-menu.show {
  opacity: 1;
  transform: scale(1) translateY(0);
  pointer-events: auto;
}

.theme-option {
  background: transparent;
  border: none;
  color: var(--text-secondary);
  padding: var(--space-2) var(--space-3);
  width: 100%;
  text-align: left;
  font-size: var(--text-sm);
  border-radius: var(--radius-sm);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: space-between;
  transition: all var(--duration-fast);
}

.theme-option:hover {
  background: var(--ds-background-2);
  color: var(--text-primary);
}

.theme-option.active {
  font-weight: var(--weight-medium);
  color: var(--text-primary);
  background: var(--ds-background-2);
}

.option-content {
  display: flex;
  align-items: center;
  gap: var(--space-2);
}
</style>
