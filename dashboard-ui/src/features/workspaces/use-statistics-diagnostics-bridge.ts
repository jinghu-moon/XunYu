import { nextTick, ref, watch } from 'vue'

import type {
  AuditFocusRequest,
  DiagnosticsCenterFocusRequest,
  StatisticsWorkspaceLinkPayload,
} from '../../types'
import { useRecentTasksBridge } from './use-recent-tasks-bridge'

type ExternalStatisticsLink = {
  key: number
  payload: StatisticsWorkspaceLinkPayload
} | null | undefined

export function useStatisticsDiagnosticsBridge(props: {
  externalLink?: ExternalStatisticsLink
}) {
  const focusSequence = ref(0)
  const diagnosticsFocus = ref<DiagnosticsCenterFocusRequest | null>(null)
  const auditFocus = ref<AuditFocusRequest | null>(null)
  const diagnosticsAnchor = ref<HTMLElement | null>(null)
  const auditAnchor = ref<HTMLElement | null>(null)
  const { recentTasksAnchor, recentTasksFocus, focusRecentTasks } = useRecentTasksBridge()

  function nextFocusKey() {
    focusSequence.value += 1
    return focusSequence.value
  }

  async function focusDiagnosticsCenter(request: Omit<DiagnosticsCenterFocusRequest, 'key'>) {
    diagnosticsFocus.value = {
      key: nextFocusKey(),
      ...request,
    }
    await nextTick()
    diagnosticsAnchor.value?.scrollIntoView?.({ behavior: 'smooth', block: 'start' })
  }

  async function focusAudit(request: Omit<AuditFocusRequest, 'key'>) {
    auditFocus.value = {
      key: nextFocusKey(),
      ...request,
    }
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

  return {
    auditAnchor,
    auditFocus,
    diagnosticsAnchor,
    diagnosticsFocus,
    handleDiagnosticsLink,
    recentTasksAnchor,
    recentTasksFocus,
  }
}
