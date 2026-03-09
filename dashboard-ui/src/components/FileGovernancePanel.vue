<script setup lang="ts">
import { computed, reactive, watch } from 'vue'
import { runWorkspaceTask } from '../api'
import type { WorkspaceCapabilities, WorkspaceTaskRunResponse } from '../types'
import AclDiffDetails from './AclDiffDetails.vue'
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
    aclReferencePath?: string
    capabilities?: WorkspaceCapabilities | null
  }>(),
  {
    path: '',
    aclReferencePath: '',
    capabilities: null,
  },
)

const probeStates = reactive<Record<ProbeKey, ProbeState>>({
  lock: { loading: false, error: '', result: null },
  protect: { loading: false, error: '', result: null },
  acl: { loading: false, error: '', result: null },
})

const aclDiffState = reactive<ProbeState>({
  loading: false,
  error: '',
  result: null,
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

function normalizeComparablePath(path: string): string {
  return path.trim().replace(/\\/g, '/').replace(/\/+$/, '').toLowerCase()
}

const activePath = computed(() => props.path.trim())
const aclReferencePath = computed(() => props.aclReferencePath.trim())
const canCompareAcl = computed(() => {
  if (!activePath.value || !aclReferencePath.value) return false
  return normalizeComparablePath(activePath.value) !== normalizeComparablePath(aclReferencePath.value)
})

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
const aclDiffHasDetails = computed(
  () => aclDiffState.result?.details?.kind === 'acl_diff' || aclDiffState.result?.details?.kind === 'acl_diff_transition',
)
const aclDiffBadgeText = computed(() => {
  if (!aclReferencePath.value) return '未设置参考'
  if (!canCompareAcl.value) return '需切换目标'
  if (aclDiffState.loading) return '加载中'
  if (aclDiffState.error) return '失败'
  const details = aclDiffState.result?.details
  if (details?.kind === 'acl_diff') {
    return details.diff.has_diff ? '仍有差异' : '已对齐'
  }
  if (details?.kind === 'acl_diff_transition') {
    return details.after.has_diff ? '仍有差异' : '已对齐'
  }
  return aclDiffState.result ? '已刷新' : '待刷新'
})

function resetProbeState(state: ProbeState) {
  state.loading = false
  state.error = ''
  state.result = null
}

function resetStates() {
  for (const key of Object.keys(probeStates) as ProbeKey[]) {
    resetProbeState(probeStates[key])
  }
  resetProbeState(aclDiffState)
}

function errorMessage(err: unknown): string {
  if (err instanceof Error && err.message.trim()) return err.message
  return '请求失败，请检查全局错误提示。'
}

function formatProcessOutput(result: WorkspaceTaskRunResponse | null): string {
  if (!result) return '-'
  const raw = result.process.stdout || result.process.stderr || '暂无执行输出'
  try {
    return JSON.stringify(JSON.parse(raw), null, 2)
  } catch {
    return raw
  }
}

function buildAclDiffRequest(path: string, reference: string) {
  return {
    workspace: 'files-security',
    action: 'acl:diff',
    target: path,
    args: ['acl', 'diff', '-p', path, '-r', reference],
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

  resetProbeState(aclDiffState)
  if (!canCompareAcl.value) return

  aclDiffState.loading = true
  try {
    aclDiffState.result = await runWorkspaceTask(buildAclDiffRequest(path, aclReferencePath.value))
  } catch (err) {
    aclDiffState.error = errorMessage(err)
  } finally {
    aclDiffState.loading = false
  }
}

watch([activePath, aclReferencePath], () => {
  resetStates()
}, { immediate: true })
</script>

<template>
  <section class="governance-panel">
    <header class="governance-panel__header">
      <div>
        <h3 class="governance-panel__title">治理快照</h3>
        <p class="governance-panel__desc">
          聚合当前文件的锁占用、保护规则和 ACL 摘要；如果已设置 ACL 参考路径，还会一并展示结构化 ACL 差异视图。
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
      <div class="governance-panel__paths">
        <div class="governance-panel__path">
          <span class="governance-panel__path-label">治理对象</span>
          <strong class="governance-panel__path-value">{{ activePath }}</strong>
        </div>
        <div class="governance-panel__path" data-testid="acl-reference-path">
          <span class="governance-panel__path-label">ACL 参考</span>
          <strong class="governance-panel__path-value">{{ aclReferencePath || '-' }}</strong>
        </div>
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

        <article class="governance-panel__card governance-panel__card--wide" data-testid="probe-acl-diff">
          <div class="governance-panel__card-header">
            <div>
              <h4 class="governance-panel__card-title">ACL 差异视图</h4>
              <p class="governance-panel__card-desc">对比治理对象与参考路径的 ACL 差异，辅助决定是否执行 `acl:copy` 等高风险动作。</p>
            </div>
            <span
              :class="[
                'governance-panel__badge',
                aclDiffState.error ? 'is-error' : aclDiffHasDetails && aclDiffBadgeText === '已对齐' ? 'is-ok' : '',
              ]"
            >
              {{ aclDiffBadgeText }}
            </span>
          </div>

          <p v-if="!aclReferencePath" class="governance-panel__message">先在工作台里设置 ACL 参考路径，再刷新快照即可看到差异视图。</p>
          <p v-else-if="!canCompareAcl" class="governance-panel__message">当前治理对象与 ACL 参考相同，请切换目标文件后再对比。</p>
          <p v-else-if="aclDiffState.error" class="governance-panel__message governance-panel__message--error">{{ aclDiffState.error }}</p>
          <p v-else-if="aclDiffState.loading" class="governance-panel__message">正在对比 ACL...</p>
          <p v-else-if="!aclDiffState.result" class="governance-panel__message">点击“刷新快照”后展示结构化 ACL 差异视图。</p>
          <template v-else>
            <div class="governance-panel__meta">
              <span>{{ aclDiffState.result.action }}</span>
              <span>{{ aclDiffState.result.process.duration_ms }} ms</span>
            </div>
            <AclDiffDetails v-if="aclDiffState.result.details" :details="aclDiffState.result.details" />
            <pre v-else class="governance-panel__output">{{ aclDiffState.result.process.command_line }}

{{ formatProcessOutput(aclDiffState.result) }}</pre>
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

.governance-panel__paths {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: var(--space-3);
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

.governance-panel__card--wide {
  background: var(--surface-card);
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
