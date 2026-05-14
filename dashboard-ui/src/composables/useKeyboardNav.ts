import { onMounted, onBeforeUnmount, type Ref } from 'vue'
import type { WorkspaceTabDefinition } from '../features/tasks/catalog'

const FOCUSABLE_SELECTOR = 'input, textarea, select, [contenteditable="true"]'

function isEditableTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false
  if (target.matches(FOCUSABLE_SELECTOR)) return true
  if (target.isContentEditable) return true
  return false
}

/**
 * 全局 Tab/Shift+Tab 工作区切换。
 * 焦点在可编辑元素时跳过，CommandPalette 打开时跳过。
 */
export function useKeyboardNav(
  workspace: Ref<WorkspaceKey>,
  tabs: WorkspaceTabDefinition[],
  paletteOpen: Ref<boolean>,
) {
  function onKeyDown(e: KeyboardEvent) {
    if (e.key !== 'Tab') return
    if (paletteOpen.value) return
    if (isEditableTarget(e.target)) return

    e.preventDefault()

    const currentIndex = tabs.findIndex((t) => t.value === workspace.value)
    if (currentIndex === -1) return

    const nextIndex = e.shiftKey
      ? (currentIndex - 1 + tabs.length) % tabs.length
      : (currentIndex + 1) % tabs.length

    workspace.value = tabs[nextIndex].value
  }

  onMounted(() => window.addEventListener('keydown', onKeyDown))
  onBeforeUnmount(() => window.removeEventListener('keydown', onKeyDown))
}
