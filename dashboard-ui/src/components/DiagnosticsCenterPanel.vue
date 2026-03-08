<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { fetchWorkspaceDiagnosticsSummary } from '../api'
import type { DiagnosticsSummaryResponse, EnvScope } from '../types'
import { Button } from './button'

const props = withDefaults(
  defineProps<{
    title?: string
    description?: string
  }>(),
  {
    title: '诊断中心',
    description: '集中查看环境 doctor、最近失败任务、高风险回执与审计时间线。',
  },
)

const scope = ref<EnvScope>('all')
const loading = ref(false)
const error = ref('')
const summary = ref<DiagnosticsSummaryResponse | null>(null)

function errorMessage(err: unknown): string {
  if (err instanceof Error && err.message.trim()) return err.message
  return '请求失败，请检查全局错误提示。'
}

async function load() {
  loading.value = true
  error.value = ''
  try {
    summary.value = await fetchWorkspaceDiagnosticsSummary(scope.value)
  } catch (err) {
    error.value = errorMessage(err)
  } finally {
    loading.value = false
  }
}

const doctorTone = computed(() => {
  if (summary.value?.doctor.load_error) return 'is-danger'
  if ((summary.value?.overview.doctor_errors ?? 0) > 0) return 'is-danger'
  if ((summary.value?.overview.doctor_warnings ?? 0) > 0) return 'is-warn'
  return 'is-ok'
})

function formatTime(ts: number) {
  return new Date(ts * 1000).toLocaleString()
}

watch(scope, () => {
  void load()
})

onMounted(() => {
  void load()
})
</script>

<template>
  <section class="diagnostics-center">
    <header class="diagnostics-center__header">
      <div>
        <h3 class="diagnostics-center__title">{{ props.title }}</h3>
        <p class="diagnostics-center__desc">{{ props.description }}</p>
      </div>
      <div class="diagnostics-center__actions">
        <label class="diagnostics-center__filter">
          <span>Scope</span>
          <select v-model="scope" data-testid="diagnostics-scope">
            <option value="all">all</option>
            <option value="user">user</option>
            <option value="system">system</option>
          </select>
        </label>
        <Button data-testid="diagnostics-refresh" preset="secondary" :loading="loading" @click="load">
          刷新
        </Button>
      </div>
    </header>

    <p v-if="error" class="diagnostics-center__error">{{ error }}</p>

    <div class="diagnostics-center__summary">
      <div class="diagnostics-center__card">
        <span>紧急项</span>
        <strong>{{ summary?.overview.urgent_items ?? '-' }}</strong>
      </div>
      <div class="diagnostics-center__card">
        <span>Doctor Issues</span>
        <strong>{{ summary?.overview.doctor_issues ?? '-' }}</strong>
      </div>
      <div class="diagnostics-center__card">
        <span>Failed Tasks</span>
        <strong>{{ summary?.overview.recent_failed_tasks ?? '-' }}</strong>
      </div>
      <div class="diagnostics-center__card">
        <span>Guarded Receipts</span>
        <strong>{{ summary?.overview.recent_guarded_receipts ?? '-' }}</strong>
      </div>
      <div class="diagnostics-center__card">
        <span>Audit Entries</span>
        <strong>{{ summary?.overview.audit_entries ?? '-' }}</strong>
      </div>
    </div>

    <div v-if="summary" class="diagnostics-center__grid">
      <section class="diagnostics-center__panel diagnostics-center__panel--doctor">
        <div class="diagnostics-center__panel-header">
          <h4>环境 Doctor</h4>
          <span :class="['diagnostics-center__badge', doctorTone]">
            {{ summary.doctor.load_error ? 'doctor error' : `${summary.doctor.errors} error / ${summary.doctor.warnings} warn` }}
          </span>
        </div>
        <p v-if="summary.doctor.load_error" class="diagnostics-center__muted">
          {{ summary.doctor.load_error }}
        </p>
        <p v-else class="diagnostics-center__muted">
          scope={{ summary.doctor.scope }}，fixable={{ summary.doctor.fixable }}
        </p>
        <div v-if="summary.doctor.issues.length" class="diagnostics-center__list">
          <article
            v-for="issue in summary.doctor.issues.slice(0, 6)"
            :key="`${issue.scope}-${issue.name}-${issue.message}`"
            class="diagnostics-center__item"
          >
            <div class="diagnostics-center__item-top">
              <strong>{{ issue.name || issue.kind }}</strong>
              <span class="diagnostics-center__badge" :class="issue.severity === 'error' ? 'is-danger' : 'is-warn'">
                {{ issue.severity }}
              </span>
            </div>
            <p class="diagnostics-center__muted">{{ issue.message }}</p>
          </article>
        </div>
        <p v-else class="diagnostics-center__muted">当前没有 doctor issue。</p>
      </section>

      <section class="diagnostics-center__panel">
        <div class="diagnostics-center__panel-header">
          <h4>最近失败任务</h4>
          <span class="diagnostics-center__badge is-danger">{{ summary.failed_tasks.length }}</span>
        </div>
        <div v-if="summary.failed_tasks.length" class="diagnostics-center__list">
          <article v-for="task in summary.failed_tasks" :key="task.id" class="diagnostics-center__item">
            <div class="diagnostics-center__item-top">
              <strong>{{ task.summary }}</strong>
              <span class="diagnostics-center__badge is-danger">{{ task.status }}</span>
            </div>
            <p class="diagnostics-center__muted">{{ task.workspace }} / {{ task.action }} / {{ formatTime(task.created_at) }}</p>
            <pre class="diagnostics-center__output">{{ task.process.command_line }}

{{ task.process.stderr || task.process.stdout || 'No output' }}</pre>
          </article>
        </div>
        <p v-else class="diagnostics-center__muted">最近没有失败任务。</p>
      </section>

      <section class="diagnostics-center__panel">
        <div class="diagnostics-center__panel-header">
          <h4>高风险动作回执</h4>
          <span class="diagnostics-center__badge is-ok">{{ summary.guarded_receipts.length }}</span>
        </div>
        <div v-if="summary.guarded_receipts.length" class="diagnostics-center__list">
          <article v-for="task in summary.guarded_receipts" :key="task.id" class="diagnostics-center__item">
            <div class="diagnostics-center__item-top">
              <strong>{{ task.summary }}</strong>
              <span :class="['diagnostics-center__badge', task.status === 'failed' ? 'is-danger' : 'is-ok']">
                {{ task.status }}
              </span>
            </div>
            <p class="diagnostics-center__muted">{{ task.audit_action || '-' }} / {{ formatTime(task.created_at) }}</p>
            <pre class="diagnostics-center__output">{{ task.process.command_line }}

{{ task.process.stdout || task.process.stderr || 'No output' }}</pre>
          </article>
        </div>
        <p v-else class="diagnostics-center__muted">最近没有 guarded receipt。</p>
      </section>

      <section class="diagnostics-center__panel">
        <div class="diagnostics-center__panel-header">
          <h4>审计时间线</h4>
          <span class="diagnostics-center__badge">{{ summary.audit_timeline.length }}</span>
        </div>
        <div v-if="summary.audit_timeline.length" class="diagnostics-center__list">
          <article v-for="entry in summary.audit_timeline" :key="`${entry.timestamp}-${entry.action}-${entry.target}`" class="diagnostics-center__item">
            <div class="diagnostics-center__item-top">
              <strong>{{ entry.action }}</strong>
              <span :class="['diagnostics-center__badge', entry.result === 'failed' ? 'is-danger' : 'is-ok']">
                {{ entry.result }}
              </span>
            </div>
            <p class="diagnostics-center__muted">{{ entry.target || '-' }} / {{ formatTime(entry.timestamp) }}</p>
            <p class="diagnostics-center__muted">{{ entry.reason || '无原因说明' }}</p>
          </article>
        </div>
        <p v-else class="diagnostics-center__muted">暂无审计事件。</p>
      </section>
    </div>
  </section>
</template>

<style scoped>
.diagnostics-center {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}

.diagnostics-center__header,
.diagnostics-center__panel-header,
.diagnostics-center__item-top,
.diagnostics-center__actions {
  display: flex;
  justify-content: space-between;
  gap: var(--space-3);
  align-items: center;
}

.diagnostics-center__title {
  font: var(--type-title);
  color: var(--text-primary);
}

.diagnostics-center__desc,
.diagnostics-center__muted {
  color: var(--text-secondary);
  font: var(--type-body-sm);
}

.diagnostics-center__error {
  color: var(--color-danger);
}

.diagnostics-center__summary {
  display: grid;
  grid-template-columns: repeat(5, minmax(0, 1fr));
  gap: var(--space-3);
}

.diagnostics-center__grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: var(--space-4);
}

.diagnostics-center__card,
.diagnostics-center__panel {
  border: var(--card-border);
  border-radius: var(--card-radius);
  background: var(--surface-card);
  box-shadow: var(--card-shadow);
  padding: var(--card-padding);
}

.diagnostics-center__card {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}

.diagnostics-center__card span {
  color: var(--text-secondary);
  font: var(--type-caption);
}

.diagnostics-center__card strong {
  color: var(--text-primary);
  font: var(--type-title-sm);
}

.diagnostics-center__panel,
.diagnostics-center__list {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
}

.diagnostics-center__item {
  border: var(--border);
  border-radius: var(--radius-md);
  background: var(--surface-panel);
  padding: var(--space-3);
}

.diagnostics-center__badge {
  display: inline-flex;
  align-items: center;
  padding: 2px var(--space-3);
  border-radius: var(--radius-full);
  background: var(--ds-background-2);
  color: var(--text-secondary);
  font: var(--type-caption);
}

.diagnostics-center__badge.is-ok {
  background: var(--color-success-bg);
  color: var(--color-success);
}

.diagnostics-center__badge.is-warn {
  background: var(--color-warning-bg);
  color: var(--color-warning);
}

.diagnostics-center__badge.is-danger {
  background: var(--color-danger-bg);
  color: var(--color-danger);
}

.diagnostics-center__output {
  border: var(--border);
  border-radius: var(--radius-md);
  background: var(--ds-background-2);
  padding: var(--space-4);
  white-space: pre-wrap;
  word-break: break-word;
  color: var(--text-primary);
}

.diagnostics-center__filter {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
}
</style>
