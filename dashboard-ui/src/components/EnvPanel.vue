<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import {
  addEnvSchemaEnum,
  addEnvSchemaRegex,
  addEnvSchemaRequired,
  addEnvPath,
  applyEnvProfile,
  captureEnvProfile,
  connectEnvWs,
  createEnvSnapshot,
  deleteEnvAnnotation,
  deleteEnvProfile,
  deleteEnvVar,
  expandEnvTemplate,
  exportEnv,
  exportEnvBundle,
  exportEnvLive,
  fetchEnvAnnotations,
  fetchEnvAudit,
  fetchEnvDiff,
  fetchEnvGraph,
  fetchEnvProfileDiff,
  fetchEnvProfiles,
  fetchEnvSchema,
  fetchEnvStatus,
  fetchEnvVarHistory,
  fetchEnvSnapshots,
  fetchEnvVars,
  fixEnvDoctor,
  pruneEnvSnapshots,
  importEnvContent,
  removeEnvPath,
  removeEnvSchemaRule,
  resetEnvSchema,
  restoreEnvSnapshot,
  runEnvCommand,
  runEnvValidate,
  runEnvDoctor,
  setEnvAnnotation,
  setEnvVar,
} from '../api'
import type {
  EnvAnnotationEntry,
  EnvDiffResult,
  EnvDepTree,
  EnvDoctorFixResult,
  EnvDoctorReport,
  EnvLiveExportFormat,
  EnvAuditEntry,
  EnvProfileMeta,
  EnvRunResult,
  EnvSchema,
  EnvScope,
  EnvSnapshotMeta,
  EnvStatusSummary,
  EnvTemplateResult,
  EnvValidationReport,
  EnvVar,
} from '../types'
import EnvAnnotationsPanel from './EnvAnnotationsPanel.vue'
import EnvAuditPanel from './EnvAuditPanel.vue'
import EnvDoctorPanel from './EnvDoctorPanel.vue'
import EnvDiffPanel from './EnvDiffPanel.vue'
import EnvGraphPanel from './EnvGraphPanel.vue'
import EnvImportExportPanel from './EnvImportExportPanel.vue'
import EnvPathEditor from './EnvPathEditor.vue'
import EnvProfilesPanel from './EnvProfilesPanel.vue'
import EnvSchemaPanel from './EnvSchemaPanel.vue'
import EnvSnapshotsPanel from './EnvSnapshotsPanel.vue'
import EnvTemplateRunPanel from './EnvTemplateRunPanel.vue'
import EnvVarHistoryDrawer from './EnvVarHistoryDrawer.vue'
import EnvVarsTable from './EnvVarsTable.vue'

const scope = ref<EnvScope>('user')
const loading = ref(false)
const vars = ref<EnvVar[]>([])
const snapshots = ref<EnvSnapshotMeta[]>([])
const doctorReport = ref<EnvDoctorReport | null>(null)
const doctorFixResult = ref<EnvDoctorFixResult | null>(null)
const diff = ref<EnvDiffResult | null>(null)
const depTree = ref<EnvDepTree | null>(null)
const diffSnapshotId = ref<string | null>(null)
const diffSince = ref<string | null>(null)
const auditEntries = ref<EnvAuditEntry[]>([])
const profiles = ref<EnvProfileMeta[]>([])
const schema = ref<EnvSchema | null>(null)
const validation = ref<EnvValidationReport | null>(null)
const profileDiff = ref<EnvDiffResult | null>(null)
const annotations = ref<EnvAnnotationEntry[]>([])
const templateResult = ref<EnvTemplateResult | null>(null)
const runResult = ref<EnvRunResult | null>(null)
const statusSummary = ref<EnvStatusSummary | null>(null)
const wsConnected = ref(false)
const historyVar = ref<string | null>(null)
const historyEntries = ref<EnvAuditEntry[]>([])
const historyLoading = ref(false)
let stopWs: (() => void) | null = null

function statText(v: number | null | undefined): string {
  return v == null ? 'N/A' : String(v)
}

const statusKpis = computed(() => {
  const s = statusSummary.value
  if (!s) return []
  return [
    { label: 'Vars', value: statText(s.total_vars) },
    { label: 'Snapshots', value: String(s.snapshots) },
    { label: 'Profiles', value: String(s.profiles) },
    { label: 'Schema', value: String(s.schema_rules) },
    { label: 'Audit', value: String(s.audit_entries) },
  ]
})

const pathEntries = computed(() => {
  const pathVar = vars.value.find((v) => v.scope === scope.value && v.name.toLowerCase() === 'path')
  if (!pathVar) return []
  return pathVar.raw_value
    .split(';')
    .map((v) => v.trim())
    .filter((v) => !!v)
})

async function withLoading<T>(fn: () => Promise<T>): Promise<T> {
  loading.value = true
  try {
    return await fn()
  } finally {
    loading.value = false
  }
}

async function refreshVars() {
  vars.value = await fetchEnvVars(scope.value)
}

async function refreshStatus() {
  statusSummary.value = await fetchEnvStatus(scope.value)
}

async function refreshSnapshots() {
  snapshots.value = await fetchEnvSnapshots()
}

async function refreshDiff() {
  diff.value = await fetchEnvDiff({
    scope: scope.value,
    snapshot: diffSnapshotId.value || undefined,
    since: diffSince.value || undefined,
  })
}

async function refreshAudit() {
  auditEntries.value = await fetchEnvAudit(120)
}

async function refreshProfiles() {
  profiles.value = await fetchEnvProfiles()
}

async function refreshSchema() {
  schema.value = await fetchEnvSchema()
}

async function refreshAnnotations() {
  annotations.value = await fetchEnvAnnotations()
}

async function refreshAll() {
  await withLoading(async () => {
    await Promise.all([
      refreshVars(),
      refreshStatus(),
      refreshSnapshots(),
      refreshDiff(),
      refreshAudit(),
      refreshProfiles(),
      refreshSchema(),
      refreshAnnotations(),
    ])
  })
}

async function onScopeChange(next: EnvScope) {
  scope.value = next
  diffSnapshotId.value = null
  diffSince.value = null
  depTree.value = null
  await refreshAll()
}

async function onSetVar(payload: { name: string; value: string; noSnapshot: boolean }) {
  await withLoading(async () => {
    const targetScope: EnvScope = scope.value === 'all' ? 'user' : scope.value
    await setEnvVar(payload.name, payload.value, targetScope, payload.noSnapshot)
    await refreshVars()
    await refreshDiff()
  })
}

async function onDeleteVar(name: string) {
  await withLoading(async () => {
    const targetScope: EnvScope = scope.value === 'all' ? 'user' : scope.value
    await deleteEnvVar(name, targetScope)
    await refreshVars()
    await refreshDiff()
  })
}

async function onPathAdd(payload: { entry: string; head: boolean }) {
  await withLoading(async () => {
    const targetScope: EnvScope = scope.value === 'all' ? 'user' : scope.value
    await addEnvPath(payload.entry, targetScope, payload.head)
    await refreshVars()
    await refreshDiff()
  })
}

async function onPathRemove(entry: string) {
  await withLoading(async () => {
    const targetScope: EnvScope = scope.value === 'all' ? 'user' : scope.value
    await removeEnvPath(entry, targetScope)
    await refreshVars()
    await refreshDiff()
  })
}

async function onCreateSnapshot(desc?: string) {
  await withLoading(async () => {
    await createEnvSnapshot(desc)
    await refreshSnapshots()
  })
}

async function onPruneSnapshots(payload: { keep: number }) {
  await withLoading(async () => {
    await pruneEnvSnapshots(payload.keep)
    await Promise.all([refreshSnapshots(), refreshStatus()])
  })
}

async function onRestoreSnapshot(payload: { id: string; scope: EnvScope }) {
  await withLoading(async () => {
    await restoreEnvSnapshot(payload)
    await Promise.all([refreshVars(), refreshSnapshots(), refreshDiff()])
  })
}

async function onRunDoctor() {
  await withLoading(async () => {
    doctorReport.value = await runEnvDoctor(scope.value)
  })
}

async function onFixDoctor() {
  await withLoading(async () => {
    doctorFixResult.value = await fixEnvDoctor(scope.value)
    doctorReport.value = await runEnvDoctor(scope.value)
    await refreshVars()
    await refreshDiff()
  })
}

async function onExport(payload: { scope: EnvScope; format: 'json' | 'env' | 'reg' | 'csv' }) {
  await withLoading(async () => {
    const data = await exportEnv(payload.scope, payload.format)
    const blob = new Blob([data], { type: 'text/plain;charset=utf-8' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = `xun-env-${payload.scope}.${payload.format}`
    a.click()
    URL.revokeObjectURL(url)
  })
}

async function onExportBundle(payload: { scope: EnvScope }) {
  await withLoading(async () => {
    const blob = await exportEnvBundle(payload.scope)
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = `xun-env-${payload.scope}.zip`
    a.click()
    URL.revokeObjectURL(url)
  })
}

async function onImport(payload: {
  scope: EnvScope
  content: string
  mode: 'merge' | 'overwrite'
  dry_run: boolean
}) {
  await withLoading(async () => {
    await importEnvContent(payload)
    if (!payload.dry_run) {
      await Promise.all([refreshVars(), refreshDiff()])
    }
  })
}

async function onCaptureProfile(payload: { name: string; scope: EnvScope }) {
  await withLoading(async () => {
    await captureEnvProfile(payload.name, payload.scope)
    await refreshProfiles()
  })
}

async function onApplyProfile(payload: { name: string; scope: EnvScope }) {
  await withLoading(async () => {
    await applyEnvProfile(payload.name, payload.scope)
    await Promise.all([refreshVars(), refreshDiff(), refreshProfiles()])
  })
}

async function onDeleteProfile(name: string) {
  await withLoading(async () => {
    await deleteEnvProfile(name)
    await refreshProfiles()
  })
}

async function onDiffProfile(payload: { name: string; scope: EnvScope }) {
  await withLoading(async () => {
    profileDiff.value = await fetchEnvProfileDiff(payload.name, payload.scope)
  })
}

async function onSchemaAddRequired(payload: { pattern: string; warnOnly: boolean }) {
  await withLoading(async () => {
    schema.value = await addEnvSchemaRequired(payload.pattern, payload.warnOnly)
  })
}

async function onSchemaAddRegex(payload: { pattern: string; regex: string; warnOnly: boolean }) {
  await withLoading(async () => {
    schema.value = await addEnvSchemaRegex(payload.pattern, payload.regex, payload.warnOnly)
  })
}

async function onSchemaAddEnum(payload: { pattern: string; values: string[]; warnOnly: boolean }) {
  await withLoading(async () => {
    schema.value = await addEnvSchemaEnum(payload.pattern, payload.values, payload.warnOnly)
  })
}

async function onSchemaRemove(pattern: string) {
  await withLoading(async () => {
    schema.value = await removeEnvSchemaRule(pattern)
  })
}

async function onSchemaReset() {
  await withLoading(async () => {
    schema.value = await resetEnvSchema()
  })
}

async function onRunValidate(payload: { scope: EnvScope; strict: boolean }) {
  await withLoading(async () => {
    validation.value = await runEnvValidate(payload.scope, payload.strict)
  })
}

async function onSetAnnotation(payload: { name: string; note: string }) {
  await withLoading(async () => {
    await setEnvAnnotation(payload.name, payload.note)
    await refreshAnnotations()
  })
}

async function onDeleteAnnotation(name: string) {
  await withLoading(async () => {
    await deleteEnvAnnotation(name)
    await refreshAnnotations()
  })
}

async function onTemplateExpand(payload: { template: string; scope: EnvScope; validate_only: boolean }) {
  await withLoading(async () => {
    templateResult.value = await expandEnvTemplate(payload)
  })
}

async function onExportLive(payload: { scope: EnvScope; format: EnvLiveExportFormat }) {
  await withLoading(async () => {
    const data = await exportEnvLive(payload.scope, payload.format)
    const blob = new Blob([data], { type: 'text/plain;charset=utf-8' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = `xun-env-live-${payload.scope}.${payload.format}`
    a.click()
    URL.revokeObjectURL(url)
  })
}

async function onRunCommand(payload: {
  cmd: string[]
  scope: EnvScope
  schema_check: boolean
  notify: boolean
  max_output: number
}) {
  await withLoading(async () => {
    runResult.value = await runEnvCommand(payload)
  })
}

async function onShowHistory(name: string) {
  historyVar.value = name
  historyLoading.value = true
  try {
    historyEntries.value = await fetchEnvVarHistory(name, 120)
  } finally {
    historyLoading.value = false
  }
}

function onCloseHistory() {
  historyVar.value = null
  historyEntries.value = []
}

async function onDiffSnapshotChange(next: string | null) {
  diffSnapshotId.value = next
  if (next) {
    diffSince.value = null
  }
  await withLoading(async () => {
    await refreshDiff()
  })
}

async function onDiffSinceChange(next: string | null) {
  diffSince.value = next
  if (next) {
    diffSnapshotId.value = null
  }
  await withLoading(async () => {
    await refreshDiff()
  })
}

async function onRunGraph(payload: { scope: EnvScope; name: string; maxDepth: number }) {
  await withLoading(async () => {
    depTree.value = await fetchEnvGraph({
      scope: payload.scope,
      name: payload.name,
      maxDepth: payload.maxDepth,
    })
  })
}

onMounted(async () => {
  stopWs = connectEnvWs(
    async (evt) => {
      if (evt.type === 'connected') {
        wsConnected.value = true
        return
      }
      await refreshAll()
    },
    () => {
      wsConnected.value = false
    },
  )
  await refreshAll()
})

onBeforeUnmount(() => {
  stopWs?.()
})
</script>

<template>
  <section class="env-panel">
    <header class="env-panel__header">
      <h2>Env Manager</h2>
      <div class="meta">
        <span>Scope: {{ scope }}</span>
        <span>WS: {{ wsConnected ? 'connected' : 'offline' }}</span>
        <button @click="refreshAll" :disabled="loading">Reload</button>
      </div>
    </header>

    <div v-if="statusSummary" class="status-strip">
      <div class="status-kpis">
        <span v-for="kpi in statusKpis" :key="kpi.label" class="status-kpi">
          <strong>{{ kpi.value }}</strong>
          <span>{{ kpi.label }}</span>
        </span>
      </div>
      <p class="status-meta">
        Latest Snapshot: {{ statusSummary.latest_snapshot_id || 'none' }}
        <span class="sep">·</span>
        Last Audit: {{ statusSummary.last_audit_at || 'none' }}
      </p>
      <p v-if="statusSummary.notes.length" class="status-note">
        Notes: {{ statusSummary.notes.join(' | ') }}
      </p>
    </div>

    <EnvVarsTable
      :vars="vars"
      :scope="scope"
      :loading="loading"
      @refresh="refreshVars"
      @scope-change="onScopeChange"
      @set-var="onSetVar"
      @delete-var="onDeleteVar"
      @show-history="onShowHistory"
    />

    <div class="grid">
      <EnvPathEditor
        :entries="pathEntries"
        :scope="scope === 'all' ? 'user' : scope"
        :loading="loading"
        @scope-change="onScopeChange"
        @refresh="refreshVars"
        @add="onPathAdd"
        @remove="onPathRemove"
      />
      <EnvSnapshotsPanel
        :snapshots="snapshots"
        :scope="scope"
        :loading="loading"
        @refresh="refreshSnapshots"
        @create="onCreateSnapshot"
        @prune="onPruneSnapshots"
        @restore="onRestoreSnapshot"
      />
    </div>

    <div class="grid">
      <EnvDoctorPanel
        :scope="scope"
        :report="doctorReport"
        :fix-result="doctorFixResult"
        :loading="loading"
        @run="onRunDoctor"
        @fix="onFixDoctor"
      />
      <EnvDiffPanel
        :diff="diff"
        :scope="scope"
        :snapshots="snapshots"
        :snapshot-id="diffSnapshotId"
        :since="diffSince"
        :loading="loading"
        @refresh="refreshDiff"
        @snapshot-change="onDiffSnapshotChange"
        @since-change="onDiffSinceChange"
      />
    </div>

    <EnvGraphPanel
      :scope="scope"
      :tree="depTree"
      :loading="loading"
      @run="onRunGraph"
    />

    <div class="grid">
      <EnvProfilesPanel
        :profiles="profiles"
        :scope="scope"
        :loading="loading"
        @refresh="refreshProfiles"
        @capture="onCaptureProfile"
        @apply="onApplyProfile"
        @delete="onDeleteProfile"
        @diff="onDiffProfile"
      />

      <EnvSchemaPanel
        :schema="schema"
        :validation="validation"
        :scope="scope"
        :loading="loading"
        @refresh-schema="refreshSchema"
        @add-required="onSchemaAddRequired"
        @add-regex="onSchemaAddRegex"
        @add-enum="onSchemaAddEnum"
        @remove-rule="onSchemaRemove"
        @reset-schema="onSchemaReset"
        @run-validate="onRunValidate"
      />
    </div>

    <p v-if="profileDiff" class="diff-hint">
      Profile Diff: +{{ profileDiff.added.length }} / -{{ profileDiff.removed.length }} / ~{{ profileDiff.changed.length }}
    </p>

    <div class="grid">
      <EnvAnnotationsPanel
        :entries="annotations"
        :loading="loading"
        @refresh="refreshAnnotations"
        @set="onSetAnnotation"
        @delete="onDeleteAnnotation"
      />
      <EnvTemplateRunPanel
        :scope="scope"
        :template-result="templateResult"
        :run-result="runResult"
        :loading="loading"
        @template-expand="onTemplateExpand"
        @export-live="onExportLive"
        @run="onRunCommand"
      />
    </div>

    <EnvImportExportPanel
      :scope="scope"
      :loading="loading"
      @export="onExport"
      @export-all="onExportBundle"
      @import="onImport"
    />

    <EnvAuditPanel :entries="auditEntries" :loading="loading" @refresh="refreshAudit" />

    <EnvVarHistoryDrawer
      :var-name="historyVar"
      :entries="historyEntries"
      :loading="historyLoading"
      @close="onCloseHistory"
    />
  </section>
</template>

<style scoped>
.env-panel {
  display: grid;
  gap: var(--space-4);
  width: 100%;
  min-width: 0;
}

.env-panel > * {
  min-width: 0;
}

.env-panel__header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.env-panel__header h2 {
  font: var(--type-title-md);
}

.meta {
  display: inline-flex;
  align-items: center;
  gap: var(--space-2);
  color: var(--text-secondary);
}

.status-strip {
  display: grid;
  gap: var(--space-2);
  padding: var(--space-3);
  border: var(--border);
  border-radius: var(--radius-md);
  background: var(--surface-card);
}

.status-kpis {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
}

.status-kpi {
  display: inline-flex;
  align-items: baseline;
  gap: var(--space-1);
  padding: 2px var(--space-2);
  border-radius: var(--radius-full);
  border: var(--border);
  color: var(--text-secondary);
}

.status-kpi strong {
  color: var(--text-primary);
  font-weight: var(--weight-semibold);
}

.status-meta,
.status-note {
  color: var(--text-secondary);
  font: var(--type-caption);
}

.sep {
  padding: 0 var(--space-1);
}

.grid {
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
  gap: var(--space-4);
  min-width: 0;
}

.grid > * {
  min-width: 0;
}

:deep(.env-card) {
  width: 100%;
  max-width: 100%;
  min-width: 0;
}

:deep(.env-card .table-wrap) {
  max-width: 100%;
  overflow-x: auto;
}

.diff-hint {
  margin-top: calc(var(--space-4) * -0.5);
  color: var(--text-secondary);
}

button {
  padding: var(--comp-padding-sm);
  border: var(--border);
  border-radius: var(--radius-sm);
  background: var(--surface-card);
  color: var(--text-primary);
  cursor: pointer;
}

button:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

@media (max-width: 960px) {
  .grid {
    grid-template-columns: 1fr;
  }

  .env-panel__header {
    flex-direction: column;
    align-items: flex-start;
    gap: var(--space-2);
  }
}
</style>
