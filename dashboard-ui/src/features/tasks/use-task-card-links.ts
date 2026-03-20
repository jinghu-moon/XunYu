import type { Ref } from 'vue'

import type {
  GuardedTaskReceipt,
  StatisticsWorkspaceLinkPayload,
  WorkspaceTaskRunResponse,
} from '../../types'
import {
  buildAuditLinkPayloadFromReceipt,
  buildAuditLinkPayloadFromRunResult,
  buildRecentTasksLinkPayloadFromReceipt,
  buildRecentTasksLinkPayloadFromRunResult,
} from './task-link-payloads'

type LinkEmitter = (payload: StatisticsWorkspaceLinkPayload) => void

function emitTaskCardLink(
  payload: StatisticsWorkspaceLinkPayload | null,
  emit: LinkEmitter,
) {
  if (payload) {
    emit(payload)
  }
}

export function useTaskCardLinks(options: {
  action: string
  result: Ref<WorkspaceTaskRunResponse | null>
  receipt: Ref<GuardedTaskReceipt | null>
  emit: LinkEmitter
}) {
  function focusRecentTasksForResult() {
    emitTaskCardLink(
      buildRecentTasksLinkPayloadFromRunResult(options.result.value, options.action),
      options.emit,
    )
  }

  function focusAuditForResult() {
    emitTaskCardLink(
      buildAuditLinkPayloadFromRunResult(options.result.value),
      options.emit,
    )
  }

  function focusRecentTasksForReceipt() {
    emitTaskCardLink(
      buildRecentTasksLinkPayloadFromReceipt(options.receipt.value),
      options.emit,
    )
  }

  function focusAuditForReceipt() {
    emitTaskCardLink(
      buildAuditLinkPayloadFromReceipt(options.receipt.value),
      options.emit,
    )
  }

  return {
    focusRecentTasksForResult,
    focusAuditForResult,
    focusRecentTasksForReceipt,
    focusAuditForReceipt,
  }
}
