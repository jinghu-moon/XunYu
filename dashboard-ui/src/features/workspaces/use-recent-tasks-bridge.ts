import { nextTick, ref } from 'vue'

import type {
  RecentTasksFocusRequest,
  StatisticsWorkspaceLinkPayload,
} from '../../types'

type NonRecentTasksHandler = (payload: StatisticsWorkspaceLinkPayload) => void | Promise<void>

export function useRecentTasksBridge() {
  const recentTasksFocus = ref<RecentTasksFocusRequest | null>(null)
  const recentTasksFocusKey = ref(0)
  const recentTasksAnchor = ref<HTMLElement | null>(null)

  function nextFocusKey() {
    recentTasksFocusKey.value += 1
    return recentTasksFocusKey.value
  }

  async function focusRecentTasks(request: Omit<RecentTasksFocusRequest, 'key'>) {
    recentTasksFocus.value = {
      key: nextFocusKey(),
      ...request,
    }
    await nextTick()
    recentTasksAnchor.value?.scrollIntoView?.({ behavior: 'smooth', block: 'start' })
  }

  async function handleRecentTasksLink(
    payload: StatisticsWorkspaceLinkPayload,
    forward?: NonRecentTasksHandler,
  ) {
    if (payload.panel === 'recent-tasks') {
      await focusRecentTasks(payload.request)
      return true
    }

    if (forward) {
      await forward(payload)
    }
    return false
  }

  return {
    recentTasksAnchor,
    recentTasksFocus,
    focusRecentTasks,
    handleRecentTasksLink,
  }
}
