<script setup lang="ts">
import type { DiffHunk } from '../../types'
import DiffViewer from './DiffViewer.vue'

defineProps<{
  hunks: DiffHunk[]
  viewMode: 'unified' | 'split'
  kind: 'line' | 'ast'
}>()
</script>

<template>
  <div class="ld">
    <DiffViewer :hunks="hunks" :view-mode="viewMode" :kind="kind" />
  </div>
</template>

<style scoped>
.ld {
  display: block;
}
</style>
