<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from 'vue'
import {
  type DiagnosticsCenterFocusRequest,
  type DiagnosticsCenterPanelId,
  type DiagnosticsGovernanceFamilyFilter,
  type DiagnosticsGovernanceStatusFilter,
  type DiagnosticsSummaryResponse,
  type EnvScope,
  type RecentTaskDryRunFilter,
  type RecentTaskRecord,
  type StatisticsWorkspaceLinkPayload,
} from '../types'
import { fetchWorkspaceDiagnosticsSummary } from '../api'
import { Button } from './button'
import FileGovernanceSummary from './FileGovernanceSummary.vue'
import { resolveRecentTaskGovernanceContext } from './recent-task-governance'
import {
  buildDiagnosticsAuditEntryKey,
  resolveDiagnosticsGovernanceFamilyFromAction,
} from './statistics-diagnostics-focus'

type DiagnosticsPanelId = DiagnosticsCenterPanelId
type GovernanceAlertFamily = DiagnosticsGovernanceFamilyFilter
type GovernanceAlertGroupKey = Exclude<GovernanceAlertFamily, 'all'>
type GovernanceAlertStatusFilter = DiagnosticsGovernanceStatusFilter
interface GovernanceAlertEntry {
  record: RecentTaskRecord
  context: ReturnType<typeof resolveRecentTaskGovernanceContext>
  family: GovernanceAlertGroupKey
  familyLabel: string
}

interface GovernanceAlertGroup {
  family: GovernanceAlertGroupKey
  label: string
  items: GovernanceAlertEntry[]
}

const emit = defineEmits<{
  (event: 'link-panel', payload: StatisticsWorkspaceLinkPayload): void
}>()

const governanceGroupOrder: GovernanceAlertGroupKey[] = ['acl', 'protect', 'crypt', 'other']
const diagnosticsPanelLinks: ReadonlyArray<{ id: DiagnosticsPanelId; label: string }> = [
  { id: 'doctor', label: 'Doctor' },
  { id: 'governance', label: '????' },
  { id: 'failed', label: '????' },
  { id: 'guarded', label: '?????' },
  { id: 'audit', label: '?????' },
]

const props = withDefaults(
  defineProps<{
    title?: string
    description?: string
    focusRequest?: DiagnosticsCenterFocusRequest | null
  }>(),
  {
    title: '????',
    description: '???? Doctor?????????????????',
    focusRequest: null,
  },
)

const scope = ref<EnvScope>('all')
const loading = ref(false)
const error = ref('')
const summary = ref<DiagnosticsSummaryResponse | null>(null)
const activePanel = ref<DiagnosticsPanelId>('doctor')
const governanceFamily = ref<GovernanceAlertFamily>('all')
const governanceStatus = ref<GovernanceAlertStatusFilter>('all')
const focusedTaskId = ref('')
const focusedTarget = ref('')
const focusedAuditAction = ref('')
const focusedAuditKey = ref('')

function errorMessage(err: unknown): string {
  if (err instanceof Error && err.message.trim()) return err.message
  return '???????????????'
}

function formatTime(ts: number) {
  return new Date(ts * 1000).toLocaleString()
}

function formatOutput(stdout: string, stderr: string) {
  return stderr || stdout || '?????'
}

function resolveGovernanceAlertFamily(action: string): GovernanceAlertGroupKey {
  return resolveDiagnosticsGovernanceFamilyFromAction(action) ?? 'other'
}

function resolveGovernanceAlertFamilyLabel(family: GovernanceAlertGroupKey): string {
  switch (family) {
    case 'acl':
      return 'ACL'
    case 'protect':
      return 'Protect'
    case 'crypt':
      return '???'
    default:
      return '??'
  }
}

function resolveRecentTaskDryRunFilter(record: RecentTaskRecord): RecentTaskDryRunFilter {
  return record.dry_run ? 'dry-run' : 'executed'
}

function resolveAuditResult(record: RecentTaskRecord): string {
  switch (record.status) {
    case 'failed':
      return 'failed'
    case 'previewed':
      return 'dry_run'
    default:
      return 'success'
  }
}

function isActivePanel(panelId: DiagnosticsPanelId): boolean {
  return activePanel.value === panelId
}

async function focusPanel(panelId: DiagnosticsPanelId) {
  activePanel.value = panelId
  await nextTick()
  if (typeof document === 'undefined') return
  const panel = document.querySelector<HTMLElement>(`[data-panel-id="${panelId}"]`)
  panel?.scrollIntoView?.({ behavior: 'smooth', block: 'start' })
}

function jumpToPanel(panelId: DiagnosticsPanelId) {
  void focusPanel(panelId)
}

async function applyFocusRequest(request: DiagnosticsCenterFocusRequest | null | undefined) {
  if (!request) return
  governanceFamily.value = request.governance_family ?? 'all'
  governanceStatus.value = request.governance_status ?? 'all'
  focusedTaskId.value = request.task_id ?? ''
  focusedTarget.value = request.target?.trim() ?? ''
  focusedAuditAction.value = request.audit_action?.trim() ?? ''
  focusedAuditKey.value = request.audit_timestamp
    ? buildDiagnosticsAuditEntryKey({
        timestamp: request.audit_timestamp,
        action: request.audit_action ?? '',
        target: request.target,
        result: request.audit_result,
      })
    : ''
  await focusPanel(request.panel)
}

function isFocusedTask(record: RecentTaskRecord) {
  if (focusedTaskId.value) return record.id === focusedTaskId.value
  if (!focusedTarget.value && !focusedAuditAction.value) return false
  if (focusedTarget.value && record.target?.trim() !== focusedTarget.value) return false
  if (focusedAuditAction.value && record.audit_action?.trim() !== focusedAuditAction.value) return false
  return true
}

function isFocusedGovernanceEntry(entry: GovernanceAlertEntry) {
  return isFocusedTask(entry.record)
}

function isFocusedAuditEntry(entry: DiagnosticsSummaryResponse['audit_timeline'][number]) {
  if (focusedAuditKey.value) {
    return (
      buildDiagnosticsAuditEntryKey({
        timestamp: entry.timestamp,
        action: entry.action,
        target: entry.target,
        result: entry.result,
      }) === focusedAuditKey.value
    )
  }
  if (!focusedTarget.value && !focusedAuditAction.value) return false
  if (focusedTarget.value && entry.target?.trim() !== focusedTarget.value) return false
  if (focusedAuditAction.value && entry.action?.trim() !== focusedAuditAction.value) return false
  return true
}

function openRecentTasks(record: RecentTaskRecord) {
  emit('link-panel', {
    panel: 'recent-tasks',
    request: {
      selected_task_id: record.id,
      status: record.status,
      dry_run: resolveRecentTaskDryRunFilter(record),
    },
  })
}

function openAuditPanel(record: RecentTaskRecord) {
  emit('link-panel', {
    panel: 'audit',
    request: {
      search: record.target?.trim() || undefined,
      action: record.audit_action?.trim() || undefined,
      result: resolveAuditResult(record),
    },
  })
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

const doctorBadgeText = computed(() => {
  if (!summary.value) return '-'
  if (summary.value.doctor.load_error) return 'doctor ??'
  return `${summary.value.doctor.errors} ?? / ${summary.value.doctor.warnings} ??`
})

const governanceAlerts = computed<GovernanceAlertEntry[]>(() =>
  (summary.value?.governance_alerts ?? []).map((record) => {
    const family = resolveGovernanceAlertFamily(record.action)
    return {
      record,
      context: resolveRecentTaskGovernanceContext(record),
      family,
      familyLabel: resolveGovernanceAlertFamilyLabel(family),
    }
  }),
)

const filteredGovernanceAlerts = computed(() =>
  governanceAlerts.value.filter((entry) => {
    if (governanceFamily.value !== 'all' && entry.family !== governanceFamily.value) return false
    if (governanceStatus.value !== 'all' && entry.record.status !== governanceStatus.value) return false
    return true
  }),
)

const governanceAlertGroups = computed<GovernanceAlertGroup[]>(() =>
  governanceGroupOrder
    .map((family) => ({
      family,
      label: resolveGovernanceAlertFamilyLabel(family),
      items: filteredGovernanceAlerts.value.filter((entry) => entry.family === family),
    }))
    .filter((group) => group.items.length > 0),
)

const governanceAlertSummaryText = computed(() => {
  const total = governanceAlerts.value.length
  if (!total) return '???????'
  return `?? ${filteredGovernanceAlerts.value.length} / ${total} ?????`
})

watch(scope, () => {
  void load()
})

watch(
  () => props.focusRequest?.key,
  async () => {
    if (!props.focusRequest) return
    await applyFocusRequest(props.focusRequest)
  },
)

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
          <span>??</span>
          <select v-model="scope" data-testid="diagnostics-scope">
            <option value="all">all</option>
            <option value="user">user</option>
            <option value="system">system</option>
          </select>
        </label>
        <Button data-testid="diagnostics-refresh" preset="secondary" :loading="loading" @click="load">
          ??
        </Button>
      </div>
    </header>

    <p v-if="error" class="diagnostics-center__error">{{ error }}</p>

    <div class="diagnostics-center__summary">
      <div class="diagnostics-center__card">
        <span>???</span>
        <strong>{{ summary?.overview.urgent_items ?? '-' }}</strong>
      </div>
      <div class="diagnostics-center__card">
        <span>Doctor ??</span>
        <strong>{{ summary?.overview.doctor_issues ?? '-' }}</strong>
      </div>
      <div class="diagnostics-center__card">
        <span>????</span>
        <strong>{{ summary?.overview.recent_failed_tasks ?? '-' }}</strong>
      </div>
      <div class="diagnostics-center__card">
        <span>?????</span>
        <strong>{{ summary?.overview.recent_guarded_receipts ?? '-' }}</strong>
      </div>
      <div class="diagnostics-center__card">
        <span>????</span>
        <strong>{{ summary?.overview.recent_governance_alerts ?? '-' }}</strong>
      </div>
      <div class="diagnostics-center__card">
        <span>????</span>
        <strong>{{ summary?.overview.audit_entries ?? '-' }}</strong>
      </div>
    </div>

    <nav v-if="summary" class="diagnostics-center__nav" aria-label="????????">
      <button
        v-for="link in diagnosticsPanelLinks"
        :key="link.id"
        :class="['diagnostics-center__jump', { 'is-active': isActivePanel(link.id) }]"
        :data-testid="`diagnostics-jump-${link.id}`"
        type="button"
        @click="jumpToPanel(link.id)"
      >
        {{ link.label }}
      </button>
    </nav>

    <div v-if="summary" class="diagnostics-center__grid">
      <section
        :class="['diagnostics-center__panel', 'diagnostics-center__panel--doctor', { 'is-active': isActivePanel('doctor') }]"
        data-panel-id="doctor"
      >
        <div class="diagnostics-center__panel-header">
          <h4>?? Doctor</h4>
          <span :class="['diagnostics-center__badge', doctorTone]">
            {{ doctorBadgeText }}
          </span>
        </div>
        <p v-if="summary.doctor.load_error" class="diagnostics-center__muted">
          {{ summary.doctor.load_error }}
        </p>
        <p v-else class="diagnostics-center__muted">
          ????={{ summary.doctor.scope }} / ???={{ summary.doctor.fixable }}
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
        <p v-else class="diagnostics-center__muted">?? Doctor ???</p>
      </section>

      <section
        :class="['diagnostics-center__panel', { 'is-active': isActivePanel('governance') }]"
        data-panel-id="governance"
      >
        <div class="diagnostics-center__panel-header">
          <h4>??????</h4>
          <span class="diagnostics-center__badge is-warn">{{ governanceAlerts.length }}</span>
        </div>
        <div class="diagnostics-center__panel-toolbar">
          <label class="diagnostics-center__filter">
            <span>???</span>
            <select v-model="governanceFamily" data-testid="diagnostics-governance-family">
              <option value="all">all</option>
              <option value="acl">acl</option>
              <option value="protect">protect</option>
              <option value="crypt">crypt</option>
              <option value="other">other</option>
            </select>
          </label>
          <label class="diagnostics-center__filter">
            <span>??</span>
            <select v-model="governanceStatus" data-testid="diagnostics-governance-status">
              <option value="all">all</option>
              <option value="failed">failed</option>
              <option value="succeeded">succeeded</option>
              <option value="previewed">previewed</option>
            </select>
          </label>
          <p class="diagnostics-center__muted diagnostics-center__summary-text">
            {{ governanceAlertSummaryText }}
          </p>
        </div>
        <div v-if="governanceAlertGroups.length" class="diagnostics-center__group-list">
          <article
            v-for="group in governanceAlertGroups"
            :key="group.family"
            class="diagnostics-center__group"
            data-testid="diagnostics-governance-group"
          >
            <div class="diagnostics-center__group-header">
              <strong>{{ group.label }}</strong>
              <span class="diagnostics-center__badge is-warn">{{ group.items.length }}</span>
            </div>
            <div class="diagnostics-center__list">
              <article
                v-for="entry in group.items"
                :key="entry.record.id"
                :class="['diagnostics-center__item', { 'is-active': isFocusedGovernanceEntry(entry) }]"
              >
                <div class="diagnostics-center__item-top">
                  <strong>{{ entry.record.summary }}</strong>
                  <span :class="['diagnostics-center__badge', entry.record.status === 'failed' ? 'is-danger' : 'is-warn']">
                    {{ entry.record.status }}
                  </span>
                </div>
                <p class="diagnostics-center__muted">
                  {{ entry.record.workspace }} / {{ entry.record.action }} / {{ formatTime(entry.record.created_at) }}
                </p>
                <p class="diagnostics-center__muted">????{{ entry.familyLabel }}</p>
                <FileGovernanceSummary
                  v-if="entry.context"
                  :task="entry.context.task"
                  :form="entry.context.form"
                  :phase="entry.context.phase"
                  :process="entry.context.process"
                  :details="entry.context.details"
                />
                <p v-else class="diagnostics-center__muted">????????????????????</p>
                <div class="diagnostics-center__item-actions">
                  <button
                    :data-testid="`diagnostics-link-recent-governance-${entry.record.id}`"
                    class="diagnostics-center__link"
                    type="button"
                    @click="openRecentTasks(entry.record)"
                  >
                    ???????
                  </button>
                  <button
                    :data-testid="`diagnostics-link-audit-governance-${entry.record.id}`"
                    class="diagnostics-center__link"
                    type="button"
                    @click="openAuditPanel(entry.record)"
                  >
                    ??????
                  </button>
                </div>
                <pre class="diagnostics-center__output">{{ entry.record.process.command_line }}

{{ formatOutput(entry.record.process.stdout, entry.record.process.stderr) }}</pre>
              </article>
            </div>
          </article>
        </div>
        <p v-else-if="governanceAlerts.length" class="diagnostics-center__muted">??????????????</p>
        <p v-else class="diagnostics-center__muted">?????????</p>
      </section>

      <section :class="['diagnostics-center__panel', { 'is-active': isActivePanel('failed') }]" data-panel-id="failed">
        <div class="diagnostics-center__panel-header">
          <h4>??????</h4>
          <span class="diagnostics-center__badge is-danger">{{ summary.failed_tasks.length }}</span>
        </div>
        <div v-if="summary.failed_tasks.length" class="diagnostics-center__list">
          <article
            v-for="task in summary.failed_tasks"
            :key="task.id"
            :class="['diagnostics-center__item', { 'is-active': isFocusedTask(task) }]"
          >
            <div class="diagnostics-center__item-top">
              <strong>{{ task.summary }}</strong>
              <span class="diagnostics-center__badge is-danger">{{ task.status }}</span>
            </div>
            <p class="diagnostics-center__muted">{{ task.workspace }} / {{ task.action }} / {{ formatTime(task.created_at) }}</p>
            <div class="diagnostics-center__item-actions">
              <button
                :data-testid="`diagnostics-link-recent-failed-${task.id}`"
                class="diagnostics-center__link"
                type="button"
                @click="openRecentTasks(task)"
              >
                ???????
              </button>
              <button
                :data-testid="`diagnostics-link-audit-failed-${task.id}`"
                class="diagnostics-center__link"
                type="button"
                @click="openAuditPanel(task)"
              >
                ??????
              </button>
            </div>
            <pre class="diagnostics-center__output">{{ task.process.command_line }}

{{ formatOutput(task.process.stdout, task.process.stderr) }}</pre>
          </article>
        </div>
        <p v-else class="diagnostics-center__muted">???????</p>
      </section>

      <section :class="['diagnostics-center__panel', { 'is-active': isActivePanel('guarded') }]" data-panel-id="guarded">
        <div class="diagnostics-center__panel-header">
          <h4>???????</h4>
          <span class="diagnostics-center__badge is-ok">{{ summary.guarded_receipts.length }}</span>
        </div>
        <div v-if="summary.guarded_receipts.length" class="diagnostics-center__list">
          <article
            v-for="task in summary.guarded_receipts"
            :key="task.id"
            :class="['diagnostics-center__item', { 'is-active': isFocusedTask(task) }]"
          >
            <div class="diagnostics-center__item-top">
              <strong>{{ task.summary }}</strong>
              <span :class="['diagnostics-center__badge', task.status === 'failed' ? 'is-danger' : 'is-ok']">
                {{ task.status }}
              </span>
            </div>
            <p class="diagnostics-center__muted">{{ task.audit_action || '-' }} / {{ formatTime(task.created_at) }}</p>
            <div class="diagnostics-center__item-actions">
              <button
                :data-testid="`diagnostics-link-recent-guarded-${task.id}`"
                class="diagnostics-center__link"
                type="button"
                @click="openRecentTasks(task)"
              >
                ???????
              </button>
              <button
                :data-testid="`diagnostics-link-audit-guarded-${task.id}`"
                class="diagnostics-center__link"
                type="button"
                @click="openAuditPanel(task)"
              >
                ??????
              </button>
            </div>
            <pre class="diagnostics-center__output">{{ task.process.command_line }}

{{ formatOutput(task.process.stdout, task.process.stderr) }}</pre>
          </article>
        </div>
        <p v-else class="diagnostics-center__muted">??????????</p>
      </section>

      <section :class="['diagnostics-center__panel', { 'is-active': isActivePanel('audit') }]" data-panel-id="audit">
        <div class="diagnostics-center__panel-header">
          <h4>?????</h4>
          <span class="diagnostics-center__badge">{{ summary.audit_timeline.length }}</span>
        </div>
        <div v-if="summary.audit_timeline.length" class="diagnostics-center__list">
          <article
            v-for="entry in summary.audit_timeline"
            :key="`${entry.timestamp}-${entry.action}-${entry.target}`"
            :class="['diagnostics-center__item', { 'is-active': isFocusedAuditEntry(entry) }]"
          >
            <div class="diagnostics-center__item-top">
              <strong>{{ entry.action }}</strong>
              <span :class="['diagnostics-center__badge', entry.result === 'failed' ? 'is-danger' : 'is-ok']">
                {{ entry.result }}
              </span>
            </div>
            <p class="diagnostics-center__muted">{{ entry.target || '-' }} / {{ formatTime(entry.timestamp) }}</p>
            <p class="diagnostics-center__muted">{{ entry.reason || '?????' }}</p>
          </article>
        </div>
        <p v-else class="diagnostics-center__muted">???????</p>
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
.diagnostics-center__actions,
.diagnostics-center__group-header {
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
  grid-template-columns: repeat(6, minmax(0, 1fr));
  gap: var(--space-3);
}

.diagnostics-center__nav {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
}

.diagnostics-center__jump,
.diagnostics-center__link {
  border: var(--border);
  border-radius: var(--radius-full);
  background: var(--surface-panel);
  color: var(--text-secondary);
  cursor: pointer;
  font: var(--type-caption);
}

.diagnostics-center__jump {
  padding: var(--space-2) var(--space-3);
}

.diagnostics-center__jump.is-active,
.diagnostics-center__panel.is-active {
  border-color: var(--color-primary);
  box-shadow: 0 0 0 1px var(--color-primary);
}

.diagnostics-center__grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: var(--space-4);
}

.diagnostics-center__card,
.diagnostics-center__panel,
.diagnostics-center__group {
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
.diagnostics-center__list,
.diagnostics-center__group-list,
.diagnostics-center__group {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
}

.diagnostics-center__panel-toolbar {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-3);
  align-items: flex-end;
}

.diagnostics-center__summary-text {
  margin-left: auto;
}

.diagnostics-center__item {
  border: var(--border);
  border-radius: var(--radius-md);
  background: var(--surface-panel);
  padding: var(--space-3);
}

.diagnostics-center__item-actions {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
}

.diagnostics-center__link {
  padding: 0;
  border: none;
  background: transparent;
  color: var(--color-primary);
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
