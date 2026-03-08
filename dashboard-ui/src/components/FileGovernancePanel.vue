<script setup lang="ts">
import { computed, reactive, watch } from 'vue'
import { runWorkspaceTask } from '../api'
import type { WorkspaceCapabilities, WorkspaceTaskRunResponse } from '../types'
import { Button } from './button'

type ProbeKey = 'lock' | 'protect' | 'acl'

interface ProbeState {
  loading: boolean
  error: string
  result: WorkspaceTaskRunResponse | null
}

interface ProbeDefinition {
  id: ProbeKey
  title: string
  description: string
  feature?: keyof WorkspaceCapabilities
  buildRequest: (path: string) => {
    workspace: string
    action: string
    target: string
    args: string[]
  }
}

const props = withDefaults(
  defineProps<{
    path?: string
    capabilities?: WorkspaceCapabilities | null
  }>(),
  {
    path: '',
    capabilities: null,
  },
)

const probeStates = reactive<Record<ProbeKey, ProbeState>>({
  lock: { loading: false, error: '', result: null },
  protect: { loading: false, error: '', result: null },
  acl: { loading: false, error: '', result: null },
})

const probeDefinitions: ProbeDefinition[] = [
  {
    id: 'lock',
    title: '锁占用快照',
    description: '查看当前文件是否被进程占用。',
    feature: 'lock',
    buildRequest: (path) => ({
      workspace: 'files-security',
      action: 'lock:who',
      target: path,
      args: ['lock', 'who', '-f', 'json', path],
    }),
  },
  {
    id: 'protect',
    title: '保护规则快照',
    description: '查看当前路径命中的保护状态。',
    feature: 'protect',
    buildRequest: (path) => ({
      workspace: 'files-security',
      action: 'protect:status',
      target: path,
      args: ['protect', 'status', '-f', 'json', path],
    }),
  },
  {
    id: 'acl',
    title: 'ACL 摘要',
    description: '读取当前路径的权限摘要。',
    buildRequest: (path) => ({
      workspace: 'files-security',
      action: 'acl:view',
      target: path,
      args: ['acl', 'view', '-p', path],
    }),
  },
]

const activePath = computed(() => props.path.trim())

const cards = computed(() =>
  probeDefinitions.map((definition) => {
    const state = probeStates[definition.id]
    const available = !definition.feature || props.capabilities?.[definition.feature] !== false
    return {
      ...definition,
      available,
      loading: state.loading,
      error: state.error,
      result: state.result,
    }
  }),
)

const canRefresh = computed(() => Boolean(activePath.value))

function resetStates() {
  for (const key of Object.keys(probeStates) as ProbeKey[]) {
    probeStates[key].loading = false
    probeStates[key].error = ''
    probeStates[key].result = null
  }
}

function errorMessage(err: unknown): string {
  if (err instanceof Error && err.message.trim()) return err.message
  return '请求失败，请检查全局错误提示。'
}

function formatProcessOutput(result: WorkspaceTaskRunResponse | null): string {
  if (!result) return '-'
  const raw = result.process.stdout || result.process.stderr || 'No command output'
  try {
    return JSON.stringify(JSON.parse(raw), null, 2)
  } catch {
    return raw
  }
}

async function refreshSnapshot() {
  const path = activePath.value
  if (!path) return
  await Promise.all(
    cards.value.map(async (card) => {
      const state = probeStates[card.id]
      state.result = null
      state.error = ''
      if (!card.available) return
      state.loading = true
      try {
        state.result = await runWorkspaceTask(card.buildRequest(path))
      } catch (err) {
        state.error = errorMessage(err)
      } finally {
        state.loading = false
      }
    }),
  )
}

watch(
  () => activePath.value,
  () => {
    resetStates()
  },
  { immediate: true },
)
</script>

<template>
  <section class="governance-panel">
    <header class="governance-panel__header">
      <div>
        <h3 class="governance-panel__title">治理快照</h3>
        <p class="governance-panel__desc">
          聚合当前文件的锁占用、保护规则和 ACL 摘要；危险改动仍通过下方任务卡进入 Triple-Guard。
        </p>
      </div>
      <Button data-testid="refresh-governance" preset="secondary" :disabled="!canRefresh" @click="refreshSnapshot">
        刷新快照
      </Button>
    </header>

    <div v-if="!activePath" class="governance-panel__placeholder">
      先在上方 File Manager 选中文件，再刷新治理快照。
    </div>
    <template v-else>
      <div class="governance-panel__path">
        <span class="governance-panel__path-label">治理对象</span>
        <strong class="governance-panel__path-value">{{ activePath }}</strong>
      </div>

      <div class="governance-panel__grid">
        <article v-for="card in cards" :key="card.id" class="governance-panel__card" :data-testid="`probe-${card.id}`">
          <div class="governance-panel__card-header">
            <div>
              <h4 class="governance-panel__card-title">{{ card.title }}</h4>
              <p class="governance-panel__card-desc">{{ card.description }}</p>
            </div>
            <span
              :class="[
                'governance-panel__badge',
                !card.available ? 'is-muted' : card.error ? 'is-error' : card.result?.process.success ? 'is-ok' : '',
              ]"
            >
              {{ !card.available ? '未启用' : card.loading ? '加载中' : card.error ? '失败' : card.result ? '已刷新' : '待刷新' }}
            </span>
          </div>

          <p v-if="!card.available" class="governance-panel__message">当前构建未启用该能力。</p>
          <p v-else-if="card.error" class="governance-panel__message governance-panel__message--error">{{ card.error }}</p>
          <p v-else-if="card.loading" class="governance-panel__message">正在获取快照...</p>
          <p v-else-if="!card.result" class="governance-panel__message">点击“刷新快照”后展示最新结果。</p>
          <template v-else>
            <div class="governance-panel__meta">
              <span>{{ card.result.action }}</span>
              <span>{{ card.result.process.duration_ms }} ms</span>
            </div>
            <pre class="governance-panel__output">{{ card.result.process.command_line }}

{{ formatProcessOutput(card.result) }}</pre>
          </template>
        </article>
      </div>
    </template>
  </section>
</template>

<style scoped>
.governance-panel {
  border: var(--card-border);
  border-radius: var(--card-radius);
  background: var(--surface-card);
  box-shadow: var(--card-shadow);
  padding: var(--card-padding);
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}

.governance-panel__header,
.governance-panel__card-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: var(--space-3);
}

.governance-panel__title,
.governance-panel__card-title {
  font: var(--type-title-sm);
  color: var(--text-primary);
}

.governance-panel__desc,
.governance-panel__card-desc,
.governance-panel__message,
.governance-panel__path-label,
.governance-panel__meta {
  color: var(--text-secondary);
  font: var(--type-body-sm);
}

.governance-panel__placeholder,
.governance-panel__path {
  padding: var(--space-3);
  border: var(--border);
  border-radius: var(--radius-md);
  background: var(--surface-panel);
}

.governance-panel__path {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
}

.governance-panel__path-value {
  color: var(--text-primary);
  font: var(--type-body-sm);
  font-family: var(--font-family-mono);
  word-break: break-all;
}

.governance-panel__grid {
  display: grid;
  gap: var(--space-3);
}

.governance-panel__card {
  border: var(--border);
  border-radius: var(--radius-md);
  background: var(--surface-panel);
  padding: var(--space-3);
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
}

.governance-panel__badge {
  display: inline-flex;
  align-items: center;
  padding: 2px var(--space-3);
  border-radius: var(--radius-full);
  background: var(--ds-background-2);
  color: var(--text-secondary);
  font: var(--type-caption);
}

.governance-panel__badge.is-ok {
  background: var(--color-success-bg);
  color: var(--color-success);
}

.governance-panel__badge.is-error {
  background: var(--color-danger-bg);
  color: var(--color-danger);
}

.governance-panel__badge.is-muted {
  opacity: 0.75;
}

.governance-panel__message--error {
  color: var(--color-danger);
}

.governance-panel__meta {
  display: flex;
  justify-content: space-between;
  gap: var(--space-3);
}

.governance-panel__output {
  border: var(--border);
  border-radius: var(--radius-md);
  background: var(--ds-background-2);
  padding: var(--space-4);
  white-space: pre-wrap;
  word-break: break-word;
  color: var(--text-primary);
}
</style>
