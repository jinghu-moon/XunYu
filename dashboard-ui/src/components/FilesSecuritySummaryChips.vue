<script setup lang="ts">
import { computed } from 'vue'

const props = defineProps<{
  currentDirectory: string
  selectedPath: string
  aclReferencePath: string
  batchCount: number
}>()

const TEXT = {
  currentDirectory: '\u76ee\u5f55',
  currentFile: '\u6587\u4ef6',
  aclReference: 'ACL \u53c2\u8003',
  batchCount: '\u6279\u91cf',
} as const

const chips = computed(() => [
  {
    key: 'directory',
    label: TEXT.currentDirectory,
    value: props.currentDirectory || '-',
  },
  {
    key: 'file',
    label: TEXT.currentFile,
    value: props.selectedPath || '-',
  },
  {
    key: 'acl-reference',
    label: TEXT.aclReference,
    value: props.aclReferencePath || '-',
  },
  {
    key: 'batch-count',
    label: TEXT.batchCount,
    value: String(props.batchCount),
  },
])
</script>

<template>
  <div class="files-security-summary-chips">
    <span
      v-for="chip in chips"
      :key="chip.key"
      :data-testid="`summary-chip-${chip.key}`"
      class="files-security-summary-chips__item"
    >
      {{ chip.label }} {{ chip.value }}
    </span>
  </div>
</template>

<style scoped>
.files-security-summary-chips {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
}

.files-security-summary-chips__item {
  display: inline-flex;
  max-width: 320px;
  align-items: center;
  padding: 2px var(--space-3);
  border-radius: var(--radius-full);
  background: var(--ds-background-2);
  color: var(--text-secondary);
  font: var(--type-caption);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>
