<script setup lang="ts">
import type {
  RecentTasksFocusRequest,
  StatisticsWorkspaceLinkPayload,
  WorkspaceCapabilities,
} from '../types'
import BatchGovernancePanel from './BatchGovernancePanel.vue'
import { Button } from './button'

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
    capabilities?: WorkspaceCapabilities | null
  }>(),
  {
    capabilities: null,
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
  title: '\u6587\u4ef6\u4e0a\u4e0b\u6587\u6865\u63a5',
  description:
    '\u4ece File Manager \u53d6\u5f53\u524d\u76ee\u5f55\u548c\u5f53\u524d\u6587\u4ef6\uff0c\u4e00\u952e\u586b\u5145\u5230\u4efb\u52a1\u5361\uff1b\u5371\u9669\u52a8\u4f5c\u4ecd\u7136\u5fc5\u987b\u8d70\u9884\u6f14\u3001\u786e\u8ba4\u3001\u56de\u6267\u3002',
  currentDirectory: '\u5f53\u524d\u76ee\u5f55',
  currentFile: '\u5f53\u524d\u6587\u4ef6',
  aclReference: 'ACL \u53c2\u8003',
  syncDirectory: '\u540c\u6b65\u76ee\u5f55\u4efb\u52a1',
  syncSelection: '\u540c\u6b65\u6587\u4ef6\u4efb\u52a1',
  syncAll: '\u540c\u6b65\u5168\u90e8',
  setAclReference: '\u8bbe\u4e3a ACL \u53c2\u8003',
  syncAclComparison: '\u540c\u6b65 ACL \u5bf9\u6bd4',
  addSelectionToBatch: '\u52a0\u5165\u6279\u91cf\u961f\u5217',
  batchTitle: '\u6279\u91cf\u961f\u5217\u4e0e\u6cbb\u7406',
  batchDescription:
    '\u6279\u91cf\u6536\u53e3 protect / encrypt / decrypt / ACL \u7b49\u9ad8\u98ce\u9669\u6cbb\u7406\u52a8\u4f5c\uff0c\u7edf\u4e00\u8d70\u9010\u9879\u9884\u6f14\u3001\u786e\u8ba4\u4e0e\u56de\u6267\u3002',
  clearBatch: '\u6e05\u7a7a',
  syncBatchToFind: '\u6279\u91cf\u586b\u5145\u67e5\u627e',
  syncBatchToBackup: '\u6279\u91cf\u586b\u5145\u5907\u4efd',
  removeBatchPath: '\u79fb\u9664',
  emptyBatch:
    '\u5148\u5728 File Manager \u9009\u4e2d\u6587\u4ef6\uff0c\u518d\u52a0\u5165\u6279\u91cf\u961f\u5217\u3002',
} as const

function batchOverflowText(count: number) {
  return `\u8fd8\u6709 ${count} \u9879\u672a\u5c55\u5f00\u3002`
}
</script>

<template>
  <section class="files-security-context__card">
    <header class="files-security-context__card-header">
      <div>
        <h3 class="files-security-context__card-title">{{ TEXT.title }}</h3>
        <p class="files-security-context__card-desc">{{ TEXT.description }}</p>
      </div>
    </header>

    <div class="files-security-context__grid">
      <div class="files-security-context__item">
        <span class="files-security-context__label">{{ TEXT.currentDirectory }}</span>
        <strong class="files-security-context__value">{{ props.currentDirectory || '-' }}</strong>
      </div>
      <div class="files-security-context__item">
        <span class="files-security-context__label">{{ TEXT.currentFile }}</span>
        <strong class="files-security-context__value">{{ props.selectedPath || '-' }}</strong>
      </div>
      <div class="files-security-context__item">
        <span class="files-security-context__label">{{ TEXT.aclReference }}</span>
        <strong class="files-security-context__value">{{ props.aclReferencePath || '-' }}</strong>
      </div>
    </div>

    <div class="files-security-context__actions">
      <Button preset="secondary" :disabled="!props.hasDirectory" @click="emit('sync-directory-context')">{{ TEXT.syncDirectory }}</Button>
      <Button preset="secondary" :disabled="!props.hasSelection" @click="emit('sync-selection-context')">{{ TEXT.syncSelection }}</Button>
      <Button preset="primary" :disabled="!props.hasDirectory && !props.hasSelection" @click="emit('sync-all-context')">{{ TEXT.syncAll }}</Button>
      <Button preset="secondary" :disabled="!props.hasSelection" @click="emit('set-acl-reference')">{{ TEXT.setAclReference }}</Button>
      <Button preset="secondary" :disabled="!props.canSyncAclComparison" @click="emit('sync-acl-comparison-context')">{{ TEXT.syncAclComparison }}</Button>
      <Button preset="secondary" :disabled="!props.canQueueSelection" @click="emit('add-selection-to-batch')">{{ TEXT.addSelectionToBatch }}</Button>
    </div>

    <p class="files-security-context__message">{{ props.syncMessage }}</p>

    <div class="files-security-context__batch">
      <div class="files-security-context__batch-header">
        <div>
          <h4 class="files-security-context__batch-title">{{ TEXT.batchTitle }}</h4>
          <p class="files-security-context__batch-desc">{{ TEXT.batchDescription }}</p>
        </div>
        <Button preset="secondary" :disabled="!props.hasBatch" @click="emit('clear-batch')">{{ TEXT.clearBatch }}</Button>
      </div>

      <div class="files-security-context__actions files-security-context__actions--batch">
        <Button preset="secondary" :disabled="!props.hasBatch" @click="emit('sync-batch-to-find')">{{ TEXT.syncBatchToFind }}</Button>
        <Button preset="secondary" :disabled="!props.hasBatch" @click="emit('sync-batch-to-backup')">{{ TEXT.syncBatchToBackup }}</Button>
      </div>

      <div v-if="props.hasBatch" class="files-security-context__batch-list">
        <div v-for="path in props.batchPreview" :key="path" class="files-security-context__batch-item">
          <span>{{ path }}</span>
          <button type="button" class="files-security-context__remove-btn" @click="emit('remove-batch-path', path)">{{ TEXT.removeBatchPath }}</button>
        </div>
        <p v-if="props.batchOverflow > 0" class="files-security-context__batch-more">{{ batchOverflowText(props.batchOverflow) }}</p>
      </div>
      <p v-else class="files-security-context__empty">{{ TEXT.emptyBatch }}</p>
    </div>

    <BatchGovernancePanel
      :paths="props.batchPaths"
      :capabilities="props.capabilities"
      @focus-recent-tasks="emit('focus-recent-tasks', $event)"
      @link-panel="emit('link-panel', $event)"
    />
  </section>
</template>

<style scoped>
.files-security-context__card {
  border: var(--card-border);
  border-radius: var(--card-radius);
  background: var(--surface-card);
  box-shadow: var(--card-shadow);
  padding: var(--card-padding);
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}

.files-security-context__card-title,
.files-security-context__batch-title {
  font: var(--type-title-sm);
  color: var(--text-primary);
}

.files-security-context__card-desc,
.files-security-context__batch-desc,
.files-security-context__message,
.files-security-context__empty,
.files-security-context__batch-more {
  color: var(--text-secondary);
  font: var(--type-body-sm);
}

.files-security-context__grid {
  display: grid;
  gap: var(--space-3);
}

.files-security-context__item {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
  padding: var(--space-3);
  border: var(--border);
  border-radius: var(--radius-md);
  background: var(--surface-panel);
}

.files-security-context__label {
  color: var(--text-secondary);
  font: var(--type-caption);
}

.files-security-context__value {
  color: var(--text-primary);
  font: var(--type-body-sm);
  font-family: var(--font-family-mono);
  word-break: break-all;
}

.files-security-context__actions {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
}

.files-security-context__batch {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}

.files-security-context__actions--batch {
  margin-top: calc(var(--space-2) * -1);
}

.files-security-context__batch-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: var(--space-3);
}

.files-security-context__batch-list {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}

.files-security-context__batch-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: var(--space-3);
  padding: var(--space-2) var(--space-3);
  border: var(--border);
  border-radius: var(--radius-md);
  background: var(--surface-panel);
  font: var(--type-body-sm);
  color: var(--text-primary);
}

.files-security-context__batch-item span {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.files-security-context__remove-btn {
  border: none;
  background: transparent;
  color: var(--color-danger);
  cursor: pointer;
  font: var(--type-caption);
}
</style>
