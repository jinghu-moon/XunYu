import { computed, ref } from 'vue'

import {
  buildAclComparisonPresets,
  buildBatchBackupPresets,
  buildBatchFindPresets,
  buildDirectoryPresets,
  buildSelectionPresets,
  mergePresetMaps,
  normalizeFilesSecurityPath,
} from './files-security-context-core'
import type { TaskPresetMap } from '../workspaces/task-presets'

export function useFilesSecurityContext() {
  const currentDirectory = ref('')
  const selectedPath = ref('')
  const aclReferencePath = ref('')
  const batchPaths = ref<string[]>([])
  const taskPresets = ref<TaskPresetMap>({})
  const presetVersion = ref(0)
  const syncMessage = ref('\u7b49\u5f85\u4ece\u4e0a\u65b9\u6587\u4ef6\u7ba1\u7406\u5668\u540c\u6b65\u4e0a\u4e0b\u6587\u3002')

  const hasDirectory = computed(() => Boolean(currentDirectory.value.trim()))
  const hasSelection = computed(() => Boolean(selectedPath.value.trim()))
  const hasBatch = computed(() => batchPaths.value.length > 0)
  const canQueueSelection = computed(
    () => hasSelection.value && !batchPaths.value.includes(selectedPath.value.trim()),
  )
  const canSyncAclComparison = computed(() => {
    if (!hasSelection.value || !aclReferencePath.value.trim()) return false
    return normalizeFilesSecurityPath(selectedPath.value) !== normalizeFilesSecurityPath(aclReferencePath.value)
  })
  const batchPreview = computed(() => batchPaths.value.slice(0, 6))
  const batchOverflow = computed(() => Math.max(batchPaths.value.length - batchPreview.value.length, 0))

  function applyTaskPresets(presets: TaskPresetMap, message: string) {
    taskPresets.value = presets
    presetVersion.value += 1
    syncMessage.value = message
  }

  function syncDirectoryContext() {
    applyTaskPresets(
      buildDirectoryPresets(currentDirectory.value),
      `\u5df2\u5c06\u76ee\u5f55\u4e0a\u4e0b\u6587\u540c\u6b65\u5230 tree / find / bak\uff1a${currentDirectory.value || '-'}`,
    )
  }

  function syncSelectionContext() {
    applyTaskPresets(
      mergePresetMaps(
        buildDirectoryPresets(currentDirectory.value),
        buildSelectionPresets(currentDirectory.value, selectedPath.value),
      ),
      `\u5df2\u5c06\u6587\u4ef6\u4e0a\u4e0b\u6587\u540c\u6b65\u5230\u5220\u9664\u3001\u79fb\u52a8\u3001ACL\u3001\u52a0\u89e3\u5bc6\u7b49\u4efb\u52a1\uff1a${selectedPath.value || '-'}`,
    )
  }

  function syncAllContext() {
    applyTaskPresets(
      mergePresetMaps(
        buildDirectoryPresets(currentDirectory.value),
        buildSelectionPresets(currentDirectory.value, selectedPath.value),
      ),
      '\u5df2\u5c06\u5f53\u524d\u76ee\u5f55\u4e0e\u5f53\u524d\u6587\u4ef6\u540c\u6b65\u5230\u6587\u4ef6\u4efb\u52a1\u533a\u3002',
    )
  }

  function setAclReference() {
    const path = normalizeFilesSecurityPath(selectedPath.value)
    if (!path) return

    aclReferencePath.value = path
    syncMessage.value = `\u5df2\u8bbe\u7f6e ACL \u53c2\u8003\u8def\u5f84\uff1a${path}`
  }

  function syncAclComparisonContext() {
    applyTaskPresets(
      mergePresetMaps(
        buildSelectionPresets(currentDirectory.value, selectedPath.value),
        buildAclComparisonPresets(selectedPath.value, aclReferencePath.value),
      ),
      `\u5df2\u5c06 ACL \u5bf9\u6bd4 / \u590d\u5236\u4efb\u52a1\u540c\u6b65\u4e3a\uff1a${selectedPath.value || '-'} <- ${aclReferencePath.value || '-'}`,
    )
  }

  function syncBatchToFind() {
    applyTaskPresets(
      buildBatchFindPresets(currentDirectory.value, batchPaths.value),
      `\u5df2\u5c06 ${batchPaths.value.length} \u4e2a\u6761\u76ee\u586b\u5165\u9ad8\u7ea7\u67e5\u627e\u3002`,
    )
  }

  function syncBatchToBackup() {
    applyTaskPresets(
      buildBatchBackupPresets(currentDirectory.value, batchPaths.value),
      `\u5df2\u5c06 ${batchPaths.value.length} \u4e2a\u6761\u76ee\u586b\u5165\u5907\u4efd include\u3002`,
    )
  }

  function addSelectionToBatch() {
    const path = normalizeFilesSecurityPath(selectedPath.value)
    if (!path || batchPaths.value.includes(path)) return

    batchPaths.value = [...batchPaths.value, path]
    syncMessage.value = `\u5df2\u52a0\u5165\u6279\u91cf\u961f\u5217\uff1a${path}`
  }

  function removeBatchPath(path: string) {
    batchPaths.value = batchPaths.value.filter((item) => item !== path)
    syncMessage.value = batchPaths.value.length
      ? '\u5df2\u66f4\u65b0\u6279\u91cf\u961f\u5217\u3002'
      : '\u6279\u91cf\u961f\u5217\u5df2\u6e05\u7a7a\u3002'
  }

  function clearBatch() {
    batchPaths.value = []
    syncMessage.value = '\u5df2\u6e05\u7a7a\u6279\u91cf\u961f\u5217\u3002'
  }

  function onDirectoryChange(path: string) {
    currentDirectory.value = normalizeFilesSecurityPath(path)
  }

  function onSelectionChange(path: string) {
    selectedPath.value = normalizeFilesSecurityPath(path)
  }

  return {
    aclReferencePath,
    addSelectionToBatch,
    batchOverflow,
    batchPaths,
    batchPreview,
    canQueueSelection,
    canSyncAclComparison,
    clearBatch,
    currentDirectory,
    hasBatch,
    hasDirectory,
    hasSelection,
    onDirectoryChange,
    onSelectionChange,
    presetVersion,
    removeBatchPath,
    selectedPath,
    setAclReference,
    syncAclComparisonContext,
    syncAllContext,
    syncBatchToBackup,
    syncBatchToFind,
    syncDirectoryContext,
    syncMessage,
    syncSelectionContext,
    taskPresets,
  }
}
