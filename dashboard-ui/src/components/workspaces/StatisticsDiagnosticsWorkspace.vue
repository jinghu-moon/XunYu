<script setup lang="ts">
import { nextTick, ref, watch } from 'vue'
import type {
  AuditFocusRequest,
  DiagnosticsCenterFocusRequest,
  RecentTasksFocusRequest,
  StatisticsWorkspaceLinkPayload,
  WorkspaceCapabilities,
} from '../../types'
import { statisticsDiagnosticsTaskGroups } from '../../workspace-tools'
import AuditPanel from '../AuditPanel.vue'
import DiagnosticsCenterPanel from '../DiagnosticsCenterPanel.vue'
import RecentTasksPanel from '../RecentTasksPanel.vue'
import RecipePanel from '../RecipePanel.vue'
import TaskToolbox from '../TaskToolbox.vue'
import WorkspaceFrame from '../WorkspaceFrame.vue'

const focusSequence = ref(0)
const diagnosticsFocus = ref<DiagnosticsCenterFocusRequest | null>(null)
const recentTasksFocus = ref<RecentTasksFocusRequest | null>(null)
const auditFocus = ref<AuditFocusRequest | null>(null)
const diagnosticsAnchor = ref<HTMLElement | null>(null)
const recentTasksAnchor = ref<HTMLElement | null>(null)
const auditAnchor = ref<HTMLElement | null>(null)

const props = defineProps<{
  capabilities?: WorkspaceCapabilities | null
  externalLink?: {
    key: number
    payload: StatisticsWorkspaceLinkPayload
  } | null
}>()

function nextFocusKey() {
  focusSequence.value += 1
  return focusSequence.value
}

async function focusDiagnosticsCenter(request: Omit<DiagnosticsCenterFocusRequest, 'key'>) {
  diagnosticsFocus.value = { key: nextFocusKey(), ...request }
  await nextTick()
  diagnosticsAnchor.value?.scrollIntoView?.({ behavior: 'smooth', block: 'start' })
}

async function focusRecentTasks(request: Omit<RecentTasksFocusRequest, 'key'>) {
  recentTasksFocus.value = { key: nextFocusKey(), ...request }
  await nextTick()
  recentTasksAnchor.value?.scrollIntoView?.({ behavior: 'smooth', block: 'start' })
}

async function focusAudit(request: Omit<AuditFocusRequest, 'key'>) {
  auditFocus.value = { key: nextFocusKey(), ...request }
  await nextTick()
  auditAnchor.value?.scrollIntoView?.({ behavior: 'smooth', block: 'start' })
}

async function handleDiagnosticsLink(payload: StatisticsWorkspaceLinkPayload) {
  if (payload.panel === 'diagnostics-center') {
    await focusDiagnosticsCenter(payload.request)
    return
  }
  if (payload.panel === 'recent-tasks') {
    await focusRecentTasks(payload.request)
    return
  }
  await focusAudit(payload.request)
}

watch(
  () => props.externalLink?.key,
  async () => {
    if (!props.externalLink) return
    await handleDiagnosticsLink(props.externalLink.payload)
  },
  { immediate: true },
)
</script>

<template>
  <WorkspaceFrame title="?????" description="??????????????? Recipe / ???????">
    <div ref="diagnosticsAnchor" data-testid="statistics-diagnostics-anchor">
      <DiagnosticsCenterPanel :focus-request="diagnosticsFocus" @link-panel="handleDiagnosticsLink" />
    </div>
    <div ref="recentTasksAnchor" data-testid="statistics-recent-tasks-anchor">
      <RecentTasksPanel
        title="????"
        description="????????????????????"
        :limit="20"
        :focus-request="recentTasksFocus"
        @link-panel="handleDiagnosticsLink"
      />
    </div>
    <RecipePanel
      title="Recipe ???"
      description="????????????????????????????"
      @link-panel="handleDiagnosticsLink"
    />
    <div ref="auditAnchor" data-testid="statistics-audit-anchor">
      <AuditPanel :focus-request="auditFocus" @link-panel="handleDiagnosticsLink" />
    </div>
    <TaskToolbox
      v-for="group in statisticsDiagnosticsTaskGroups"
      :key="group.id"
      :title="group.title"
      :description="group.description"
      :tasks="group.tasks"
      :capabilities="capabilities"
      @link-panel="handleDiagnosticsLink"
    />
  </WorkspaceFrame>
</template>
