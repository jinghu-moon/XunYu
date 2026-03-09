<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { IconRefresh, IconSearch } from '@tabler/icons-vue'
import { Button } from './button'
import type { AuditEntry, AuditFocusRequest, AuditResponse, StatisticsWorkspaceLinkPayload } from '../types'
import { fetchAudit } from '../api'
import SkeletonTable from './SkeletonTable.vue'
import { pushToast } from '../ui/feedback'
import { downloadCsv, downloadJson } from '../ui/export'
import { resolveDiagnosticsCenterFocusFromAuditEntry } from './statistics-diagnostics-focus'

const emit = defineEmits<{
  (event: 'link-panel', payload: StatisticsWorkspaceLinkPayload): void
}>()

const props = withDefaults(
  defineProps<{
    focusRequest?: AuditFocusRequest | null
  }>(),
  {
    focusRequest: null,
  },
)

const search = ref('')
const action = ref('')
const result = ref('')
const busy = ref(false)
const resp = ref<AuditResponse>({ entries: [], stats: { total: 0, by_action: {}, by_result: {} } })
const page = ref(1)
const pageSize = ref(50)
const detailEntry = ref<AuditEntry | null>(null)

const entries = computed<AuditEntry[]>(() => resp.value.entries)
const actionItems = computed(() => Object.keys(resp.value.stats.by_action).sort())
const resultItems = computed(() => Object.keys(resp.value.stats.by_result).sort())
const activeFilterItems = computed(() => {
  const items: Array<{ key: string; label: string; value: string }> = []
  const searchKeyword = search.value.trim()

  if (searchKeyword) items.push({ key: 'search', label: '关键词', value: searchKeyword })
  if (action.value) items.push({ key: 'action', label: '动作', value: action.value })
  if (result.value) items.push({ key: 'result', label: '结果', value: result.value })

  return items
})
const pageCount = computed(() => Math.max(1, Math.ceil(entries.value.length / pageSize.value)))
const pageStart = computed(() => (page.value - 1) * pageSize.value)
const pageEnd = computed(() => Math.min(entries.value.length, pageStart.value + pageSize.value))
const pagedEntries = computed(() => entries.value.slice(pageStart.value, pageEnd.value))
const isLoading = computed(() => busy.value && !entries.value.length)

async function load() {
  busy.value = true
  try {
    resp.value = await fetchAudit({
      limit: 400,
      search: search.value.trim() || undefined,
      action: action.value || undefined,
      result: result.value || undefined,
    })
    page.value = 1
  } finally {
    busy.value = false
  }
}

function applyFocusRequest(request: AuditFocusRequest | null | undefined) {
  if (!request) return
  search.value = request.search ?? ''
  action.value = request.action ?? ''
  result.value = request.result ?? ''
  detailEntry.value = null
}

async function clearFilters() {
  search.value = ''
  action.value = ''
  result.value = ''
  detailEntry.value = null
  await load()
}

function fmtTs(ts: number): string {
  if (!ts) return '-'
  const d = new Date(ts * 1000)
  return d.toLocaleString()
}

function nextPage() {
  page.value = Math.min(pageCount.value, page.value + 1)
}

function prevPage() {
  page.value = Math.max(1, page.value - 1)
}

function openDetail(e: AuditEntry) {
  detailEntry.value = e
}

function closeDetail() {
  detailEntry.value = null
}

function openDiagnostics(entry: AuditEntry) {
  emit('link-panel', {
    panel: 'diagnostics-center',
    request: resolveDiagnosticsCenterFocusFromAuditEntry(entry),
  })
}

function formatPayload(raw: string): string {
  const text = raw?.trim() || ''
  if (!text) return '-'
  if (text.startsWith('{') || text.startsWith('[')) {
    try {
      return JSON.stringify(JSON.parse(text), null, 2)
    } catch {}
  }
  return text
}

function exportAudit(format: 'csv' | 'json') {
  const items = entries.value.map(e => ({
    timestamp: e.timestamp,
    time: fmtTs(e.timestamp),
    action: e.action,
    target: e.target,
    result: e.result,
    reason: e.reason,
    user: e.user || '',
    params: e.params,
  }))
  if (!items.length) {
    pushToast({ level: 'warning', title: '没有可导出的审计记录' })
    return
  }
  if (format === 'json') {
    downloadJson('audit', items)
  } else {
    const rows = items.map(e => [e.timestamp, e.time, e.action, e.target, e.result, e.reason, e.user, e.params])
    downloadCsv('audit', ['timestamp', 'time', 'action', 'target', 'result', 'reason', 'user', 'params'], rows)
  }
  pushToast({ level: 'success', title: '已导出审计记录', detail: `${items.length} 条` })
}

function isFailed(e: AuditEntry): boolean {
  return e.result?.toLowerCase() === 'failed'
}

watch([entries, pageSize], () => {
  if (page.value > pageCount.value) page.value = pageCount.value
})

watch(
  () => props.focusRequest?.key,
  async () => {
    if (!props.focusRequest) return
    applyFocusRequest(props.focusRequest)
    await load()
  },
)

onMounted(load)
</script>

<template>
  <div data-testid="audit-panel">
    <div class="toolbar">
      <div style="position:relative;flex:1;display:flex;align-items:center">
        <IconSearch :size="16" style="position:absolute;left:var(--space-2);color:var(--text-tertiary)" />
        <input v-model="search" data-testid="audit-search" placeholder="搜索动作 / 目标 / 参数 / 原因" style="width:100%;padding-left:var(--space-8)" @keydown.enter="load" />
      </div>
      <select v-model="action" data-testid="audit-action" style="max-width:200px" @change="load">
        <option value="">全部动作</option>
        <option v-for="a in actionItems" :key="a" :value="a">{{ a }}</option>
      </select>
      <select v-model="result" data-testid="audit-result" style="max-width:160px" @change="load">
        <option value="">全部结果</option>
        <option v-for="r in resultItems" :key="r" :value="r">{{ r }}</option>
      </select>
      <Button size="sm" preset="secondary" :disabled="busy" style="display:flex;align-items:center;gap:var(--space-1)" @click="load">
        <IconRefresh :size="16" /> 刷新
      </Button>
      <div class="toolbar-group">
        <span class="toolbar-label">导出</span>
        <Button size="sm" preset="secondary" @click="exportAudit('csv')">CSV</Button>
        <Button size="sm" preset="secondary" @click="exportAudit('json')">JSON</Button>
      </div>
    </div>

    <div v-if="activeFilterItems.length" class="audit-focus" data-testid="audit-active-filters">
      <span class="audit-focus__label">当前筛选</span>
      <span v-for="item in activeFilterItems" :key="item.key" class="audit-focus__chip">
        {{ item.label }}：{{ item.value }}
      </span>
      <Button data-testid="clear-audit-filters" size="sm" preset="secondary" :disabled="busy" @click="clearFilters">
        清空筛选
      </Button>
    </div>

    <div class="stats">
      <div class="stat">
        <div class="k">命中条目</div>
        <div class="v">{{ resp.stats.total }}</div>
      </div>
      <div class="stat">
        <div class="k">Redirect 移动</div>
        <div class="v">{{ resp.stats.by_action.redirect_move || 0 }}</div>
      </div>
      <div class="stat">
        <div class="k">Redirect 复制</div>
        <div class="v">{{ resp.stats.by_action.redirect_copy || 0 }}</div>
      </div>
      <div class="stat">
        <div class="k">Redirect 跳过</div>
        <div class="v">{{ resp.stats.by_action.redirect_skip || 0 }}</div>
      </div>
    </div>

    <SkeletonTable v-if="isLoading" :rows="6" :columns="6" />

    <table v-else>
      <thead>
        <tr>
          <th style="width:160px">时间</th>
          <th style="width:140px">动作</th>
          <th>目标</th>
          <th style="width:120px">结果</th>
          <th>原因</th>
          <th style="width:90px"></th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="(e, idx) in pagedEntries" :key="idx" :class="{ 'row-failed': isFailed(e) }">
          <td style="color:var(--text-secondary)">{{ fmtTs(e.timestamp) }}</td>
          <td>{{ e.action }}</td>
          <td style="color:var(--text-secondary)">{{ e.target }}</td>
          <td :style="{ color: e.result === 'success' ? 'var(--color-success)' : (e.result === 'failed' ? 'var(--color-danger)' : 'var(--text-secondary)') }">
            {{ e.result }}
          </td>
          <td style="color:var(--text-tertiary)">{{ e.reason }}</td>
          <td>
            <div class="audit-actions">
              <Button size="sm" preset="secondary" @click="openDetail(e)">详情</Button>
              <Button :data-testid="`audit-link-diagnostics-${e.timestamp}`" size="sm" preset="secondary" @click="openDiagnostics(e)">诊断</Button>
            </div>
          </td>
        </tr>
        <tr v-if="!entries.length">
          <td colspan="6" style="text-align:center;color:var(--text-tertiary)">暂无审计记录</td>
        </tr>
      </tbody>
    </table>

    <div class="pager">
      <div class="pager-info">显示 {{ pageStart + 1 }}-{{ pageEnd }} / {{ entries.length }}</div>
      <div style="flex:1" />
      <div class="pager-controls">
        <span class="pager-label">每页</span>
        <select v-model.number="pageSize">
          <option :value="20">20</option>
          <option :value="50">50</option>
          <option :value="100">100</option>
        </select>
        <Button size="sm" preset="secondary" :disabled="page <= 1" @click="prevPage">Prev</Button>
        <div class="pager-page">{{ page }}/{{ pageCount }}</div>
        <Button size="sm" preset="secondary" :disabled="page >= pageCount" @click="nextPage">Next</Button>
      </div>
    </div>

    <div v-if="detailEntry" class="modal-backdrop" @click.self="closeDetail">
      <div class="modal">
        <div class="modal-header">
          <div class="modal-title">审计详情</div>
          <Button size="sm" preset="secondary" @click="closeDetail">关闭</Button>
        </div>
        <div class="modal-body">
          <div class="detail-grid">
            <div class="detail-item">
              <div class="detail-k">时间</div>
              <div class="detail-v">{{ fmtTs(detailEntry.timestamp) }}</div>
            </div>
            <div class="detail-item">
              <div class="detail-k">动作</div>
              <div class="detail-v">{{ detailEntry.action }}</div>
            </div>
            <div class="detail-item">
              <div class="detail-k">结果</div>
              <div class="detail-v">{{ detailEntry.result }}</div>
            </div>
            <div class="detail-item">
              <div class="detail-k">目标</div>
              <div class="detail-v">{{ detailEntry.target }}</div>
            </div>
            <div class="detail-item">
              <div class="detail-k">用户</div>
              <div class="detail-v">{{ detailEntry.user || '-' }}</div>
            </div>
            <div class="detail-item">
              <div class="detail-k">原因</div>
              <div class="detail-v">{{ detailEntry.reason || '-' }}</div>
            </div>
          </div>
          <div class="detail-block">
            <div class="detail-k">参数</div>
            <pre class="payload">{{ formatPayload(detailEntry.params) }}</pre>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.stats {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: var(--space-3);
  margin-bottom: var(--space-4);
}
.stat {
  border: var(--border);
  border-radius: var(--radius-md);
  padding: var(--space-3) var(--space-4);
  background: var(--ds-background-1);
}
.stat .k {
  font-size: var(--text-xs);
  color: var(--text-secondary);
  margin-bottom: var(--space-1);
}
.stat .v {
  font-size: var(--text-lg);
  font-weight: var(--weight-semibold);
}
.toolbar-group {
  display: inline-flex;
  align-items: center;
  gap: var(--space-2);
}
.audit-actions {
  display: inline-flex;
  gap: var(--space-2);
}
.audit-focus {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: var(--space-2);
  margin-bottom: var(--space-4);
}
.audit-focus__label {
  font-size: var(--text-xs);
  color: var(--text-secondary);
}
.audit-focus__chip {
  display: inline-flex;
  align-items: center;
  padding: 2px var(--space-3);
  border-radius: var(--radius-full);
  background: var(--color-info-bg);
  color: var(--color-info);
  font: var(--type-caption);
}
.toolbar-label {
  font-size: var(--text-xs);
  color: var(--text-secondary);
}

.row-failed {
  background: var(--red-100);
}

.pager {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  margin-top: var(--space-4);
}
.pager-info {
  font-size: var(--text-xs);
  color: var(--text-secondary);
}
.pager-controls {
  display: flex;
  align-items: center;
  gap: var(--space-2);
}
.pager-label {
  font-size: var(--text-xs);
  color: var(--text-secondary);
}
.pager-page {
  font-size: var(--text-xs);
  color: var(--text-secondary);
  min-width: 56px;
  text-align: center;
}

.modal-backdrop {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.45);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: var(--z-modal);
}
.modal {
  width: min(760px, 90vw);
  max-height: 85vh;
  overflow: auto;
  background: var(--ds-background-1);
  border: var(--border);
  border-radius: var(--radius-lg);
  box-shadow: var(--shadow-md);
}
.modal-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--space-4);
  border-bottom: var(--border);
}
.modal-title {
  font-size: var(--text-md);
  font-weight: var(--weight-semibold);
}
.modal-body {
  padding: var(--space-4);
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}
.detail-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: var(--space-3);
}
.detail-item {
  border: var(--border);
  border-radius: var(--radius-md);
  padding: var(--space-3);
  background: var(--ds-background-2);
}
.detail-k {
  font-size: var(--text-xs);
  color: var(--text-secondary);
  margin-bottom: var(--space-1);
}
.detail-v {
  font-size: var(--text-sm);
  color: var(--text-primary);
  word-break: break-word;
}
.detail-block {
  border: var(--border);
  border-radius: var(--radius-md);
  padding: var(--space-3);
  background: var(--ds-background-2);
}
.payload {
  margin: 0;
  white-space: pre-wrap;
  font-family: var(--font-family-mono);
  font-size: var(--text-xs);
  color: var(--text-secondary);
}
</style>

