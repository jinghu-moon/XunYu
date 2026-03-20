<script setup lang="ts">
import type { ComponentPublicInstance } from 'vue'
import type { StatisticsWorkspaceLinkPayload, WorkspaceCapabilities } from '../../types'
import { useFilesSecurityContext } from '../../features/files-security/use-files-security-context'
import { useRecentTasksBridge } from '../../features/workspaces/use-recent-tasks-bridge'
import DiffPanel from '../DiffPanel.vue'
import FilesSecuritySidebar from '../FilesSecuritySidebar.vue'
import FilesSecuritySummaryChips from '../FilesSecuritySummaryChips.vue'
import FilesSecurityTaskZone from '../FilesSecurityTaskZone.vue'
import RedirectPanel from '../RedirectPanel.vue'
import WorkspaceFrame from '../WorkspaceFrame.vue'

const emit = defineEmits<{
  (event: 'link-panel', payload: StatisticsWorkspaceLinkPayload): void
}>()

defineProps<{
  capabilities?: WorkspaceCapabilities | null
}>()

const { recentTasksAnchor, recentTasksFocus, focusRecentTasks, handleRecentTasksLink } = useRecentTasksBridge()
const {
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
} = useFilesSecurityContext()

const TEXT = {
  title: '\u6587\u4ef6\u4e0e\u5b89\u5168',
  description:
    '\u4fdd\u7559 Diff / Redirect \u89c2\u5bdf\u80fd\u529b\uff0c\u5e76\u628a tree / find / bak / rm / acl / protect / encrypt \u6536\u53e3\u5230\u7edf\u4e00\u6587\u4ef6\u6cbb\u7406\u5de5\u4f5c\u53f0\u3002',
} as const

async function handleWorkspaceLink(payload: StatisticsWorkspaceLinkPayload) {
  await handleRecentTasksLink(payload, (nextPayload) => {
    emit('link-panel', nextPayload)
  })
}

function setRecentTasksAnchor(element: Element | ComponentPublicInstance | null) {
  recentTasksAnchor.value = element as HTMLElement | null
}
</script>

<template>
  <WorkspaceFrame
    :title="TEXT.title"
    :description="TEXT.description"
  >
    <template #summary>
      <FilesSecuritySummaryChips
        :current-directory="currentDirectory"
        :selected-path="selectedPath"
        :acl-reference-path="aclReferencePath"
        :batch-count="batchPaths.length"
      />
    </template>

    <div class="files-security__top">
      <div class="files-security__main">
        <DiffPanel
          @directory-change="onDirectoryChange"
          @selection-change="onSelectionChange"
        />
        <RedirectPanel />
      </div>

      <FilesSecuritySidebar
        :current-directory="currentDirectory"
        :selected-path="selectedPath"
        :acl-reference-path="aclReferencePath"
        :batch-paths="batchPaths"
        :batch-preview="batchPreview"
        :batch-overflow="batchOverflow"
        :sync-message="syncMessage"
        :has-batch="hasBatch"
        :has-directory="hasDirectory"
        :has-selection="hasSelection"
        :can-sync-acl-comparison="canSyncAclComparison"
        :can-queue-selection="canQueueSelection"
        :recent-tasks-focus="recentTasksFocus"
        :recent-tasks-anchor-ref="setRecentTasksAnchor"
        :capabilities="capabilities"
        @sync-directory-context="syncDirectoryContext"
        @sync-selection-context="syncSelectionContext"
        @sync-all-context="syncAllContext"
        @set-acl-reference="setAclReference"
        @sync-acl-comparison-context="syncAclComparisonContext"
        @add-selection-to-batch="addSelectionToBatch"
        @clear-batch="clearBatch"
        @sync-batch-to-find="syncBatchToFind"
        @sync-batch-to-backup="syncBatchToBackup"
        @remove-batch-path="removeBatchPath"
        @focus-recent-tasks="focusRecentTasks"
        @link-panel="handleWorkspaceLink"
      />
    </div>

    <FilesSecurityTaskZone
      :capabilities="capabilities"
      :task-presets="taskPresets"
      :preset-version="presetVersion"
      @link-panel="handleWorkspaceLink"
    />
  </WorkspaceFrame>
</template>

<style scoped>
.files-security__top {
  display: grid;
  grid-template-columns: minmax(0, 1.8fr) minmax(340px, 0.9fr);
  gap: var(--space-5);
  align-items: start;
}

.files-security__main {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}

@media (max-width: 1280px) {
  .files-security__top {
    grid-template-columns: 1fr;
  }
}
</style>
