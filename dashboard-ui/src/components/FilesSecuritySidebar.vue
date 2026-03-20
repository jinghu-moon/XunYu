<script setup lang="ts">
import type { ComponentPublicInstance } from 'vue'
import type {
  RecentTasksFocusRequest,
  StatisticsWorkspaceLinkPayload,
  WorkspaceCapabilities,
} from '../types'
import FileGovernancePanel from './FileGovernancePanel.vue'
import FileVaultFoundationPanel from './FileVaultFoundationPanel.vue'
import FilesSecurityContextBridgePanel from './FilesSecurityContextBridgePanel.vue'
import RecentTasksPanel from './RecentTasksPanel.vue'
import RecipePanel from './RecipePanel.vue'

const props = withDefaults(
  defineProps<{
    currentDirectory: string
    selectedPath: string
    aclReferencePath: string
    batchPaths: string[]
    batchPreview: string[]
    batchOverflow: number
    syncMessage: string
    hasBatch: boolean
    hasDirectory: boolean
    hasSelection: boolean
    canSyncAclComparison: boolean
    canQueueSelection: boolean
    recentTasksFocus: RecentTasksFocusRequest | null
    recentTasksAnchorRef?: (element: Element | ComponentPublicInstance | null) => void
    capabilities?: WorkspaceCapabilities | null
  }>(),
  {
    capabilities: null,
    recentTasksAnchorRef: undefined,
  },
)

const emit = defineEmits<{
  (event: 'sync-directory-context'): void
  (event: 'sync-selection-context'): void
  (event: 'sync-all-context'): void
  (event: 'set-acl-reference'): void
  (event: 'sync-acl-comparison-context'): void
  (event: 'add-selection-to-batch'): void
  (event: 'clear-batch'): void
  (event: 'sync-batch-to-find'): void
  (event: 'sync-batch-to-backup'): void
  (event: 'remove-batch-path', path: string): void
  (event: 'focus-recent-tasks', request: Omit<RecentTasksFocusRequest, 'key'>): void
  (event: 'link-panel', payload: StatisticsWorkspaceLinkPayload): void
}>()

const TEXT = {
  recentTasksTitle: '\u6587\u4ef6\u4efb\u52a1\u4e2d\u5fc3',
  recentTasksDescription:
    '\u53ea\u663e\u793a Files & Security \u5de5\u4f5c\u53f0\u7684\u6700\u8fd1\u4efb\u52a1\uff0c\u652f\u6301\u5b89\u5168\u91cd\u653e\u3002',
  recipeTitle: '\u6587\u4ef6 Recipes',
  recipeDescription:
    '\u6c89\u6dc0\u6587\u4ef6\u6e05\u7406\u3001\u626b\u63cf\u3001\u5907\u4efd\u7b49\u987a\u5e8f\u6d41\u7a0b\uff0c\u907f\u514d\u91cd\u590d\u70b9\u547d\u4ee4\u3002',
} as const
</script>

<template>
  <aside class="files-security-sidebar">
    <FilesSecurityContextBridgePanel
      :current-directory="props.currentDirectory"
      :selected-path="props.selectedPath"
      :acl-reference-path="props.aclReferencePath"
      :batch-paths="props.batchPaths"
      :batch-preview="props.batchPreview"
      :batch-overflow="props.batchOverflow"
      :sync-message="props.syncMessage"
      :has-batch="props.hasBatch"
      :has-directory="props.hasDirectory"
      :has-selection="props.hasSelection"
      :can-sync-acl-comparison="props.canSyncAclComparison"
      :can-queue-selection="props.canQueueSelection"
      :capabilities="props.capabilities"
      @sync-directory-context="emit('sync-directory-context')"
      @sync-selection-context="emit('sync-selection-context')"
      @sync-all-context="emit('sync-all-context')"
      @set-acl-reference="emit('set-acl-reference')"
      @sync-acl-comparison-context="emit('sync-acl-comparison-context')"
      @add-selection-to-batch="emit('add-selection-to-batch')"
      @clear-batch="emit('clear-batch')"
      @sync-batch-to-find="emit('sync-batch-to-find')"
      @sync-batch-to-backup="emit('sync-batch-to-backup')"
      @remove-batch-path="emit('remove-batch-path', $event)"
      @focus-recent-tasks="emit('focus-recent-tasks', $event)"
      @link-panel="emit('link-panel', $event)"
    />

    <FileGovernancePanel
      :path="props.selectedPath"
      :acl-reference-path="props.aclReferencePath"
      :capabilities="props.capabilities"
    />

    <FileVaultFoundationPanel :path="props.selectedPath" :capabilities="props.capabilities" />

    <div :ref="props.recentTasksAnchorRef">
      <RecentTasksPanel
        :title="TEXT.recentTasksTitle"
        :description="TEXT.recentTasksDescription"
        workspace="files-security"
        :limit="12"
        :focus-request="props.recentTasksFocus"
      />
    </div>

    <RecipePanel
      :title="TEXT.recipeTitle"
      :description="TEXT.recipeDescription"
      category="files-security"
    />
  </aside>
</template>

<style scoped>
.files-security-sidebar {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}
</style>
