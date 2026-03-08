<script setup lang="ts">
import { computed, defineAsyncComponent, onBeforeUnmount, onMounted, ref, type Component } from 'vue'
import CapsuleTabs from './components/CapsuleTabs.vue'
import type { CapsuleTabItem } from './components/CapsuleTabs.vue'
import CommandPalette from './components/CommandPalette.vue'
import DensityToggle from './components/DensityToggle.vue'
import GlobalFeedback from './components/GlobalFeedback.vue'
import ThemeToggle from './components/ThemeToggle.vue'
import { fetchWorkspaceCapabilities } from './api'
import type { WorkspaceCapabilities, WorkspaceKey } from './types'
import { workspaceTabs } from './workspace-tools'
import { isToastMarked, notifyError } from './ui/feedback'

const workspace = ref<WorkspaceKey>('overview')
const paletteOpen = ref(false)
const capabilities = ref<WorkspaceCapabilities | null>(null)

function loadWorkspaceComponent(loader: Parameters<typeof defineAsyncComponent>[0]): Component {
  return defineAsyncComponent(loader)
}

type CommandItem = {
  id: string
  label: string
  description?: string
  keywords?: string[]
  section?: string
  run: () => void
}

const workspaceComponentMap: Record<WorkspaceKey, Component> = {
  overview: loadWorkspaceComponent(() => import('./components/workspaces/OverviewWorkspace.vue')),
  'paths-context': loadWorkspaceComponent(() => import('./components/workspaces/PathsContextWorkspace.vue')),
  'network-proxy': loadWorkspaceComponent(() => import('./components/workspaces/NetworkProxyWorkspace.vue')),
  'environment-config': loadWorkspaceComponent(() => import('./components/workspaces/EnvironmentConfigWorkspace.vue')),
  'files-security': loadWorkspaceComponent(() => import('./components/workspaces/FilesSecurityWorkspace.vue')),
  'integration-automation': loadWorkspaceComponent(() => import('./components/workspaces/IntegrationAutomationWorkspace.vue')),
  'media-conversion': loadWorkspaceComponent(() => import('./components/workspaces/MediaConversionWorkspace.vue')),
  'statistics-diagnostics': loadWorkspaceComponent(() => import('./components/workspaces/StatisticsDiagnosticsWorkspace.vue')),
}

const tabItems: CapsuleTabItem[] = workspaceTabs.map((item) => ({ value: item.value, label: item.label }))

const activeComponent = computed(() => workspaceComponentMap[workspace.value])

const commands = computed<CommandItem[]>(() => [
  ...workspaceTabs.map((item) => ({
    id: `nav-${item.value}`,
    label: `转到${item.label}`,
    description: item.description,
    keywords: [item.value, item.label],
    section: 'Navigate',
    run: () => {
      workspace.value = item.value
    },
  })),
  {
    id: 'reload-dashboard',
    label: '重新加载控制台',
    description: '刷新当前 Dashboard UI',
    keywords: ['reload', 'refresh'],
    section: 'Actions',
    run: () => {
      window.location.reload()
    },
  },
])

async function loadCapabilities() {
  capabilities.value = await fetchWorkspaceCapabilities()
}

function onUnhandledRejection(event: PromiseRejectionEvent) {
  if (isToastMarked(event.reason)) return
  notifyError(event.reason, 'Unhandled rejection')
}

function onWindowError(event: ErrorEvent) {
  if (isToastMarked(event.error)) return
  notifyError(event.error || event.message, 'Unhandled error')
}

function onGlobalKeydown(event: KeyboardEvent) {
  const key = event.key.toLowerCase()
  if ((event.ctrlKey || event.metaKey) && key === 'k') {
    event.preventDefault()
    paletteOpen.value = true
    return
  }
  if (key === 'escape' && paletteOpen.value) {
    event.preventDefault()
    paletteOpen.value = false
  }
}

onMounted(() => {
  void loadCapabilities()
  window.addEventListener('unhandledrejection', onUnhandledRejection)
  window.addEventListener('error', onWindowError)
  window.addEventListener('keydown', onGlobalKeydown)
})

onBeforeUnmount(() => {
  window.removeEventListener('unhandledrejection', onUnhandledRejection)
  window.removeEventListener('error', onWindowError)
  window.removeEventListener('keydown', onGlobalKeydown)
})
</script>

<template>
  <div class="app">
    <header>
      <div class="header-title">
        <h1>XunYu Local Console</h1>
        <p>8 个工作台统一承载本地命令、配置、文件、安全与诊断能力。</p>
      </div>
      <CapsuleTabs v-model="workspace" :items="tabItems" />
      <div class="header-controls">
        <DensityToggle />
        <ThemeToggle />
      </div>
    </header>
    <main>
      <component :is="activeComponent" :capabilities="capabilities" />
    </main>
    <CommandPalette v-model="paletteOpen" :commands="commands" />
    <GlobalFeedback />
  </div>
</template>

<style>
* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

body {
  font-family: var(--font-family-base);
  background: var(--surface-page);
  color: var(--text-primary);
  -webkit-font-smoothing: antialiased;
}

.app {
  width: clamp(1100px, 94vw, 1880px);
  margin: 0 auto;
  padding: var(--space-6);
  min-height: 100vh;
}

header {
  display: grid;
  grid-template-columns: minmax(260px, 360px) 1fr auto;
  align-items: start;
  gap: var(--space-4);
  margin-bottom: var(--space-8);
}

.header-title h1 {
  font: var(--type-title-lg);
  letter-spacing: var(--letter-spacing-tight);
}

.header-title p {
  margin-top: var(--space-2);
  color: var(--text-secondary);
  font: var(--type-body-sm);
}

.header-controls {
  display: inline-flex;
  align-items: center;
  gap: var(--space-2);
}

main {
  background: var(--surface-panel);
  border: var(--panel-border);
  border-radius: var(--panel-radius);
  padding: var(--panel-padding);
  box-shadow: var(--panel-shadow);
}

::view-transition-old(root),
::view-transition-new(root) {
  animation: none;
  mix-blend-mode: normal;
}

::view-transition-old(root) { z-index: 1; }
::view-transition-new(root) { z-index: 9999; }

table {
  width: 100%;
  border-collapse: separate;
  border-spacing: 0;
}

th,
td {
  text-align: left;
  padding: var(--table-cell-padding-y) var(--table-cell-padding-x);
  border-bottom: var(--border);
}

th {
  color: var(--text-secondary);
  font: var(--type-caption);
  text-transform: uppercase;
  font-weight: var(--weight-medium);
  letter-spacing: var(--letter-spacing-wide);
}

td {
  font: var(--type-body-sm);
  color: var(--text-primary);
}

input,
select,
textarea {
  padding: var(--comp-padding-sm);
  border: var(--border);
  border-radius: var(--radius-sm);
  background: var(--surface-panel);
  color: var(--text-primary);
  font: var(--type-body-sm);
  outline: none;
  transition: border-color var(--duration-fast) ease;
}

input:focus,
select:focus,
textarea:focus {
  border-color: var(--text-secondary);
}

.toolbar {
  display: flex;
  gap: var(--space-2);
  margin-bottom: var(--space-4);
  align-items: center;
}

.tag-pill {
  display: inline-flex;
  align-items: center;
  gap: var(--space-1);
  padding: 2px var(--space-2);
  border-radius: var(--radius-full);
  font: var(--type-caption);
  border: 1px solid transparent;
  white-space: nowrap;
}

.tag-pill--lang {
  background: var(--color-info-bg);
  color: var(--color-info);
  border-color: var(--color-info);
}

.tag-pill--tool {
  background: var(--color-success-bg);
  color: var(--color-success);
  border-color: var(--color-success);
}

.tag-pill--env {
  background: var(--color-warning-bg);
  color: var(--color-warning);
  border-color: var(--color-warning);
}

.tag-pill--work {
  background: var(--color-danger-bg);
  color: var(--color-danger);
  border-color: var(--color-danger);
}

.tag-pill--path {
  background: var(--surface-card-muted);
  color: var(--text-secondary);
  border-color: var(--color-border-strong);
}

.tag-pill--general {
  background: var(--ds-background-2);
  color: var(--text-secondary);
  border-color: var(--color-border-strong);
}
</style>
