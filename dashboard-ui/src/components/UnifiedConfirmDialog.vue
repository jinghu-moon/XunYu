<script setup lang="ts">
import { computed } from 'vue'
import type { GuardedTaskPreviewResponse } from '../types'
import { Button } from './button'

const props = withDefaults(
  defineProps<{
    modelValue: boolean
    title: string
    warning?: string
    preview?: GuardedTaskPreviewResponse | null
    busy?: boolean
    confirmDisabled?: boolean
  }>(),
  {
    warning: '此操作具有破坏性，请先核对预演输出。',
    preview: null,
    busy: false,
    confirmDisabled: false,
  },
)

const emit = defineEmits<{
  (e: 'update:modelValue', value: boolean): void
  (e: 'confirm'): void
}>()

function close() {
  emit('update:modelValue', false)
}

function confirm() {
  emit('confirm')
}

const previewSummary = computed(() => props.preview?.summary || props.preview?.preview_summary || '-')
const previewReadyLabel = computed(() => (props.preview?.ready_to_execute ? '可执行' : '不可执行'))
</script>

<template>
  <Teleport to="body">
    <div v-if="props.modelValue" class="confirm-overlay" @click.self="close">
      <div class="confirm-dialog" role="dialog" aria-modal="true">
        <div class="confirm-dialog__header">
          <div>
            <h3 class="confirm-dialog__title">{{ props.title }}</h3>
            <p class="confirm-dialog__warning">{{ props.warning }}</p>
          </div>
        </div>
        <div class="confirm-dialog__body">
          <div v-if="props.preview" class="confirm-dialog__meta">
            <div><strong>阶段</strong> {{ props.preview.phase }}</div>
            <div><strong>目标</strong> {{ props.preview.target || '-' }}</div>
            <div><strong>摘要</strong> {{ previewSummary }}</div>
            <div><strong>状态</strong> {{ previewReadyLabel }}</div>
            <div><strong>保护链路</strong> {{ props.preview.guarded ? '已启用' : '未启用' }}</div>
            <div><strong>Dry Run</strong> {{ props.preview.dry_run ? '是' : '否' }}</div>
            <div><strong>过期</strong> {{ props.preview.expires_in_secs }}s</div>
          </div>
          <div v-if="$slots['preview-extra']" data-testid="confirm-dialog-extra" class="confirm-dialog__extra">
            <slot name="preview-extra" />
          </div>
          <pre v-if="props.preview" class="confirm-dialog__output">{{ props.preview.process.command_line }}

{{ props.preview.process.stdout || props.preview.process.stderr || '暂无预演输出' }}</pre>
        </div>
        <footer class="confirm-dialog__footer">
          <Button preset="secondary" @click="close">取消</Button>
          <Button preset="danger" :loading="props.busy" :disabled="props.confirmDisabled" @click="confirm">确认执行</Button>
        </footer>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.confirm-overlay {
  position: fixed;
  inset: 0;
  background: rgba(12, 18, 28, 0.56);
  display: grid;
  place-items: center;
  z-index: 2000;
  padding: var(--space-6);
}

.confirm-dialog {
  width: min(760px, 100%);
  border-radius: var(--radius-lg);
  border: var(--border);
  background: var(--surface-panel);
  box-shadow: var(--shadow-lg);
  padding: var(--space-5);
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}

.confirm-dialog__title {
  font: var(--type-title);
  color: var(--text-primary);
  margin-bottom: var(--space-2);
}

.confirm-dialog__warning {
  color: var(--color-danger);
  font: var(--type-body-sm);
}

.confirm-dialog__meta {
  display: grid;
  gap: var(--space-2);
  font: var(--type-body-sm);
  color: var(--text-secondary);
}

.confirm-dialog__extra {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
}

.confirm-dialog__output {
  margin-top: var(--space-3);
  max-height: 320px;
  overflow: auto;
  border-radius: var(--radius-md);
  background: var(--ds-background-2);
  border: var(--border);
  padding: var(--space-4);
  white-space: pre-wrap;
  word-break: break-word;
  color: var(--text-primary);
}

.confirm-dialog__footer {
  display: flex;
  justify-content: flex-end;
  gap: var(--space-2);
}
</style>
