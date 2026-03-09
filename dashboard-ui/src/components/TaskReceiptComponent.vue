<script setup lang="ts">
import { computed } from 'vue'
import type { GuardedTaskReceipt } from '../types'

const props = defineProps<{
  receipt: GuardedTaskReceipt
}>()

function formatTime(ts: number) {
  return new Date(ts * 1000).toLocaleString()
}

const statusLabel = computed(() => (props.receipt.status === 'succeeded' ? '成功' : '失败'))
const summary = computed(() => props.receipt.summary || `${props.receipt.action} / ${props.receipt.target || '-'}`)
</script>

<template>
  <section class="receipt-card">
    <div class="receipt-card__header">
      <div>
        <h4 class="receipt-card__title">执行回执</h4>
        <p class="receipt-card__subtitle">{{ summary }}</p>
      </div>
      <span :class="['receipt-card__badge', props.receipt.process.success ? 'is-ok' : 'is-error']">
        {{ statusLabel }}
      </span>
    </div>
    <div class="receipt-card__meta">
      <div><strong>阶段</strong> {{ props.receipt.phase }}</div>
      <div><strong>动作</strong> {{ props.receipt.action }}</div>
      <div><strong>目标</strong> {{ props.receipt.target || '-' }}</div>
      <div><strong>保护链路</strong> {{ props.receipt.guarded ? '已启用' : '未启用' }}</div>
      <div><strong>Dry Run</strong> {{ props.receipt.dry_run ? '是' : '否' }}</div>
      <div><strong>审计</strong> {{ props.receipt.audit_action }} · {{ formatTime(props.receipt.audited_at) }}</div>
      <div><strong>令牌</strong> {{ props.receipt.token }}</div>
      <div><strong>耗时</strong> {{ props.receipt.process.duration_ms }} ms</div>
    </div>
    <pre class="receipt-card__output">{{ props.receipt.process.command_line }}

{{ props.receipt.process.stdout || props.receipt.process.stderr || '暂无执行输出' }}</pre>
  </section>
</template>

<style scoped>
.receipt-card {
  border: var(--card-border);
  border-radius: var(--card-radius);
  background: var(--surface-card);
  box-shadow: var(--card-shadow);
  padding: var(--card-padding);
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
}

.receipt-card__header {
  display: flex;
  justify-content: space-between;
  gap: var(--space-3);
  align-items: center;
}

.receipt-card__title {
  font: var(--type-title-sm);
  color: var(--text-primary);
}

.receipt-card__subtitle {
  margin-top: var(--space-1);
  color: var(--text-secondary);
  font: var(--type-caption);
}

.receipt-card__badge {
  display: inline-flex;
  align-items: center;
  padding: 2px var(--space-3);
  border-radius: var(--radius-full);
  font: var(--type-caption);
  font-weight: var(--weight-semibold);
}

.receipt-card__badge.is-ok {
  background: var(--color-success-bg);
  color: var(--color-success);
}

.receipt-card__badge.is-error {
  background: var(--color-danger-bg);
  color: var(--color-danger);
}

.receipt-card__meta {
  display: grid;
  gap: var(--space-2);
  color: var(--text-secondary);
  font: var(--type-body-sm);
}

.receipt-card__output {
  border: var(--border);
  border-radius: var(--radius-md);
  background: var(--ds-background-2);
  padding: var(--space-4);
  white-space: pre-wrap;
  word-break: break-word;
  color: var(--text-primary);
}
</style>
