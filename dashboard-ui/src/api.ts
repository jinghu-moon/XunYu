import type {
  Bookmark,
  PortsResponse,
  ProxyConfig,
  ProxyItem,
  ProxyTestItem,
  RedirectConfig,
  RedirectProfile,
  RedirectDryRunResponse,
  AuditResponse,
  GlobalConfig,
  DiffApiRequest,
  DiffResult,
  FileEntry,
  FileSearchEntry,
  DiffFileInfo,
  DiffFileContent,
  ConvertFileRequest,
  ConvertFileResponse,
  ValidateFileRequest,
  ValidateFileResponse,
  DiffWsEvent,
  EnvDiffResult,
  EnvDoctorFixResult,
  EnvDepTree,
  EnvDoctorReport,
  EnvAuditEntry,
  EnvAnnotationEntry,
  EnvImportResult,
  EnvLiveExportFormat,
  EnvProfileMeta,
  EnvSchema,
  EnvRunResult,
  EnvStatusSummary,
  EnvTemplateResult,
  EnvValidationReport,
  EnvScope,
  EnvSnapshotPruneResult,
  EnvSnapshotMeta,
  EnvVar,
  EnvWsEvent,
  GuardedTaskExecuteRequest,
  GuardedTaskPreviewRequest,
  GuardedTaskPreviewResponse,
  GuardedTaskReceipt,
  WorkspaceCapabilities,
  WorkspaceOverviewSummary,
  WorkspaceTaskRunRequest,
  WorkspaceTaskRunResponse,
} from './types'
import { beginLoading, endLoading, notifyError } from './ui/feedback'

const BASE = '/api'

type HttpError = Error & { status?: number; statusText?: string; detail?: string }

function formatRequestLabel(input: RequestInfo | URL, init?: RequestInit): string {
  const method = (init?.method || 'GET').toUpperCase()
  let url = ''
  if (typeof input === 'string') url = input
  else if (input instanceof Request) url = input.url
  else url = String(input)
  return `${method} ${url}`
}

function parseErrorText(text: string): string {
  const raw = text.trim()
  if (!raw) return ''
  try {
    const data = JSON.parse(raw)
    const msg = data?.message || data?.error || data?.reason || data?.detail || data?.msg
    if (msg) return String(msg)
  } catch {}
  return raw
}

async function buildHttpError(r: Response): Promise<HttpError> {
  let detail = ''
  try {
    const t = await r.text()
    detail = parseErrorText(t)
  } catch {}
  const msg = detail ? `${r.status} ${r.statusText}: ${detail}` : `${r.status} ${r.statusText}`
  const err = new Error(msg) as HttpError
  err.status = r.status
  err.statusText = r.statusText
  err.detail = detail || undefined
  return err
}

async function request(input: RequestInfo | URL, init?: RequestInit): Promise<Response> {
  const label = formatRequestLabel(input, init)
  beginLoading()
  try {
    const r = await fetch(input, init)
    if (!r.ok) throw await buildHttpError(r)
    return r
  } catch (err) {
    notifyError(err, label)
    throw err
  } finally {
    endLoading()
  }
}

export async function fetchBookmarks(): Promise<Bookmark[]> {
  const r = await request(`${BASE}/bookmarks`)
  return r.json()
}

export async function upsertBookmark(name: string, path: string, tags: string[]): Promise<void> {
  await request(`${BASE}/bookmarks/${encodeURIComponent(name)}`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ path, tags }),
  })
}

export async function deleteBookmark(name: string): Promise<void> {
  await request(`${BASE}/bookmarks/${encodeURIComponent(name)}`, { method: 'DELETE' })
}

export async function renameBookmark(name: string, newName: string): Promise<Bookmark> {
  const r = await request(`${BASE}/bookmarks/${encodeURIComponent(name)}/rename`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ newName }),
  })
  return r.json()
}

export async function fetchPorts(): Promise<PortsResponse> {
  const r = await request(`${BASE}/ports`)
  return r.json()
}

export async function killPort(port: number): Promise<void> {
  await request(`${BASE}/ports/kill/${port}`, { method: 'POST' })
}

export async function killPid(pid: number): Promise<void> {
  await request(`${BASE}/ports/kill-pid/${pid}`, { method: 'POST' })
}

export async function fetchProxyStatus(): Promise<ProxyItem[]> {
  const r = await request(`${BASE}/proxy/status`)
  return r.json()
}

export async function fetchProxyConfig(): Promise<ProxyConfig> {
  const r = await request(`${BASE}/proxy/config`)
  return r.json()
}

export async function saveProxyConfig(cfg: ProxyConfig): Promise<void> {
  await request(`${BASE}/proxy/config`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(cfg),
  })
}

export async function fetchConfig(): Promise<GlobalConfig> {
  const r = await request(`${BASE}/config`)
  return r.json()
}

export async function patchConfig(patch: Partial<GlobalConfig>): Promise<GlobalConfig> {
  const r = await request(`${BASE}/config`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(patch),
  })
  return r.json()
}

export async function replaceConfig(cfg: GlobalConfig): Promise<GlobalConfig> {
  const r = await request(`${BASE}/config`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(cfg),
  })
  return r.json()
}

export async function proxySet(url: string, noproxy: string, only?: string): Promise<void> {
  await request(`${BASE}/proxy/set`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ url, noproxy, only }),
  })
}

export async function proxyDel(only?: string): Promise<void> {
  await request(`${BASE}/proxy/del`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ only }),
  })
}

export async function proxyTest(
  url: string,
  targets?: string,
  options?: { timeoutMs?: number; jobs?: number },
): Promise<ProxyTestItem[]> {
  const qs = new URLSearchParams({ url })
  if (targets) qs.set('targets', targets)
  if (options?.timeoutMs != null) qs.set('timeout_ms', String(options.timeoutMs))
  if (options?.jobs != null) qs.set('jobs', String(options.jobs))
  const r = await request(`${BASE}/proxy/test?${qs.toString()}`)
  return r.json()
}

export async function bookmarksBatchDelete(names: string[]): Promise<{ deleted: number }> {
  const r = await request(`${BASE}/bookmarks/batch`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ op: 'delete', names }),
  })
  return r.json()
}

export async function bookmarksBatchAddTags(names: string[], tags: string[]): Promise<{ updated: number }> {
  const r = await request(`${BASE}/bookmarks/batch`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ op: 'add_tags', names, tags }),
  })
  return r.json()
}

export async function bookmarksBatchRemoveTags(names: string[], tags: string[]): Promise<{ updated: number }> {
  const r = await request(`${BASE}/bookmarks/batch`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ op: 'remove_tags', names, tags }),
  })
  return r.json()
}

export async function fetchRedirectProfiles(): Promise<RedirectConfig> {
  const r = await request(`${BASE}/redirect/profiles`)
  return r.json()
}

export async function upsertRedirectProfile(name: string, profile: RedirectProfile): Promise<void> {
  await request(`${BASE}/redirect/profiles/${encodeURIComponent(name)}`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(profile),
  })
}

export async function deleteRedirectProfile(name: string): Promise<void> {
  await request(`${BASE}/redirect/profiles/${encodeURIComponent(name)}`, { method: 'DELETE' })
}

export async function redirectDryRun(payload: {
  source: string
  profile: RedirectProfile
  copy?: boolean
}): Promise<RedirectDryRunResponse> {
  const r = await request(`${BASE}/redirect/dry-run`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(payload),
  })
  return r.json()
}

export async function fetchAudit(params?: {
  limit?: number
  search?: string
  action?: string
  result?: string
  from?: number
  to?: number
  cursor?: number
}): Promise<AuditResponse> {
  const qs = new URLSearchParams()
  if (params?.limit) qs.set('limit', String(params.limit))
  if (params?.search) qs.set('search', params.search)
  if (params?.action) qs.set('action', params.action)
  if (params?.result) qs.set('result', params.result)
  if (params?.from != null) qs.set('from', String(params.from))
  if (params?.to != null) qs.set('to', String(params.to))
  if (params?.cursor != null) qs.set('cursor', String(params.cursor))
  const r = await request(`${BASE}/audit?${qs.toString()}`)
  return r.json()
}

export async function exportAuditCsv(params?: {
  limit?: number
  search?: string
  action?: string
  result?: string
  from?: number
  to?: number
  cursor?: number
}): Promise<string> {
  const qs = new URLSearchParams()
  qs.set('format', 'csv')
  if (params?.limit) qs.set('limit', String(params.limit))
  if (params?.search) qs.set('search', params.search)
  if (params?.action) qs.set('action', params.action)
  if (params?.result) qs.set('result', params.result)
  if (params?.from != null) qs.set('from', String(params.from))
  if (params?.to != null) qs.set('to', String(params.to))
  if (params?.cursor != null) qs.set('cursor', String(params.cursor))
  const r = await request(`${BASE}/audit?${qs.toString()}`)
  return r.text()
}

/* ── Diff ─────────────────────────────────────── */

export async function fetchDiff(req: DiffApiRequest): Promise<DiffResult> {
  const r = await request(`${BASE}/diff`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(req),
  })
  return r.json()
}

export async function fetchFiles(path: string): Promise<FileEntry[]> {
  const qs = new URLSearchParams({ path })
  const r = await request(`${BASE}/files?${qs.toString()}`)
  return r.json()
}

export async function fetchFileSearch(params: {
  root: string
  query: string
  limit?: number
}): Promise<FileSearchEntry[]> {
  const qs = new URLSearchParams({ root: params.root, query: params.query })
  if (params.limit != null) qs.set('limit', String(params.limit))
  const r = await request(`${BASE}/files/search?${qs.toString()}`)
  return r.json()
}

export async function fetchFileInfo(path: string): Promise<DiffFileInfo> {
  const qs = new URLSearchParams({ path })
  const r = await request(`${BASE}/info?${qs.toString()}`)
  return r.json()
}

export async function fetchFileContent(params: {
  path: string
  offset?: number
  limit?: number
}): Promise<DiffFileContent> {
  const qs = new URLSearchParams({ path: params.path })
  if (params.offset != null) qs.set('offset', String(params.offset))
  if (params.limit != null) qs.set('limit', String(params.limit))
  const r = await request(`${BASE}/content?${qs.toString()}`)
  return r.json()
}

export async function fetchConvertFile(reqBody: ConvertFileRequest): Promise<ConvertFileResponse> {
  const r = await request(`${BASE}/convert`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(reqBody),
  })
  return r.json()
}

export async function fetchValidateFile(reqBody: ValidateFileRequest): Promise<ValidateFileResponse> {
  const r = await request(`${BASE}/validate`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(reqBody),
  })
  return r.json()
}

export function connectDiffWs(
  onEvent: (event: DiffWsEvent) => void,
  onClose?: (reason: string) => void,
): () => void {
  const proto = window.location.protocol === 'https:' ? 'wss' : 'ws'
  const wsUrl = `${proto}://${window.location.host}/ws`
  const ws = new WebSocket(wsUrl)

  ws.addEventListener('message', (evt) => {
    try {
      const payload = JSON.parse(String(evt.data))
      if (!payload || typeof payload.type !== 'string') return
      if (payload.type !== 'connected' && payload.type !== 'refresh' && payload.type !== 'file_changed') return
      onEvent(payload as DiffWsEvent)
    } catch {
      // ignore malformed frames
    }
  })
  ws.addEventListener('close', () => {
    onClose?.('closed')
  })
  ws.addEventListener('error', () => {
    onClose?.('error')
  })

  return () => {
    if (ws.readyState === WebSocket.OPEN || ws.readyState === WebSocket.CONNECTING) {
      ws.close()
    }
  }
}

/* ── Env ─────────────────────────────────────── */

type ApiEnvelope<T> = {
  ok: boolean
  data: T
}

async function envData<T>(input: RequestInfo | URL, init?: RequestInit): Promise<T> {
  const r = await request(input, init)
  const body = (await r.json()) as ApiEnvelope<T>
  return body.data
}

export async function fetchEnvVars(scope: EnvScope): Promise<EnvVar[]> {
  const qs = new URLSearchParams({ scope })
  const payload = await envData<{ scope: EnvScope; vars: EnvVar[] }>(`${BASE}/env/vars?${qs.toString()}`)
  return payload.vars
}

export async function fetchEnvStatus(scope: EnvScope): Promise<EnvStatusSummary> {
  const qs = new URLSearchParams({ scope })
  const payload = await envData<{ status: EnvStatusSummary }>(`${BASE}/env/status?${qs.toString()}`)
  return payload.status
}

export async function fetchEnvVar(name: string, scope: EnvScope): Promise<EnvVar> {
  const qs = new URLSearchParams({ scope })
  return envData<EnvVar>(`${BASE}/env/vars/${encodeURIComponent(name)}?${qs.toString()}`)
}

export async function setEnvVar(
  name: string,
  value: string,
  scope: EnvScope,
  noSnapshot = false,
): Promise<void> {
  const qs = new URLSearchParams({ scope })
  await envData<unknown>(`${BASE}/env/vars/${encodeURIComponent(name)}?${qs.toString()}`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ value, no_snapshot: noSnapshot }),
  })
}

export async function deleteEnvVar(name: string, scope: EnvScope): Promise<void> {
  const qs = new URLSearchParams({ scope })
  await envData<unknown>(`${BASE}/env/vars/${encodeURIComponent(name)}?${qs.toString()}`, {
    method: 'DELETE',
  })
}

export async function addEnvPath(entry: string, scope: EnvScope, head = false): Promise<void> {
  await envData<unknown>(`${BASE}/env/path/add`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ entry, scope, head }),
  })
}

export async function removeEnvPath(entry: string, scope: EnvScope): Promise<void> {
  await envData<unknown>(`${BASE}/env/path/remove`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ entry, scope }),
  })
}

export async function fetchEnvSnapshots(): Promise<EnvSnapshotMeta[]> {
  const payload = await envData<{ snapshots: EnvSnapshotMeta[] }>(`${BASE}/env/snapshots`)
  return payload.snapshots
}

export async function createEnvSnapshot(desc?: string): Promise<EnvSnapshotMeta> {
  return envData(`${BASE}/env/snapshots`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ desc }),
  })
}

export async function pruneEnvSnapshots(keep: number): Promise<EnvSnapshotPruneResult> {
  const qs = new URLSearchParams({ keep: String(keep) })
  return envData<EnvSnapshotPruneResult>(`${BASE}/env/snapshots?${qs.toString()}`, {
    method: 'DELETE',
  })
}

export async function restoreEnvSnapshot(payload: {
  id?: string
  latest?: boolean
  scope?: EnvScope
}): Promise<EnvSnapshotMeta> {
  return envData(`${BASE}/env/snapshots/restore`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(payload),
  })
}

export async function runEnvDoctor(scope: EnvScope): Promise<EnvDoctorReport> {
  const payload = await envData<{ report: EnvDoctorReport }>(`${BASE}/env/doctor/run`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ scope }),
  })
  return payload.report
}

export async function fixEnvDoctor(scope: EnvScope): Promise<EnvDoctorFixResult> {
  const payload = await envData<{ result: EnvDoctorFixResult }>(`${BASE}/env/doctor/fix`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ scope }),
  })
  return payload.result
}

export async function importEnvContent(payload: {
  content: string
  scope: EnvScope
  mode: 'merge' | 'overwrite'
  dry_run?: boolean
}): Promise<EnvImportResult> {
  const data = await envData<{ result: EnvImportResult }>(`${BASE}/env/import`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(payload),
  })
  return data.result
}

export async function exportEnv(scope: EnvScope, format: 'json' | 'env' | 'reg' | 'csv'): Promise<string> {
  const qs = new URLSearchParams({ scope, format })
  const r = await request(`${BASE}/env/export?${qs.toString()}`)
  return r.text()
}

export async function exportEnvBundle(scope: EnvScope): Promise<Blob> {
  const qs = new URLSearchParams({ scope })
  const r = await request(`${BASE}/env/export-all?${qs.toString()}`)
  return r.blob()
}

export async function fetchEnvDiff(params: {
  scope: EnvScope
  snapshot?: string
  since?: string
}): Promise<EnvDiffResult> {
  const qs = new URLSearchParams({ scope: params.scope })
  if (params.snapshot) qs.set('snapshot', params.snapshot)
  if (params.since) qs.set('since', params.since)
  const data = await envData<{ diff: EnvDiffResult }>(`${BASE}/env/diff-live?${qs.toString()}`)
  return data.diff
}

export async function fetchEnvGraph(params: {
  scope: EnvScope
  name: string
  maxDepth?: number
}): Promise<EnvDepTree> {
  const qs = new URLSearchParams({ scope: params.scope, name: params.name })
  if (params.maxDepth != null) qs.set('max_depth', String(params.maxDepth))
  const data = await envData<{ tree: EnvDepTree }>(`${BASE}/env/graph?${qs.toString()}`)
  return data.tree
}

export async function fetchEnvAudit(limit = 100): Promise<EnvAuditEntry[]> {
  const qs = new URLSearchParams({ limit: String(limit) })
  const data = await envData<{ entries: EnvAuditEntry[] }>(`${BASE}/env/audit?${qs.toString()}`)
  return data.entries
}

export async function fetchEnvVarHistory(name: string, limit = 50): Promise<EnvAuditEntry[]> {
  const qs = new URLSearchParams({ limit: String(limit) })
  const data = await envData<{ name: string; entries: EnvAuditEntry[] }>(
    `${BASE}/env/vars/${encodeURIComponent(name)}/history?${qs.toString()}`,
  )
  return data.entries
}

export async function fetchEnvProfiles(): Promise<EnvProfileMeta[]> {
  const data = await envData<{ profiles: EnvProfileMeta[] }>(`${BASE}/env/profiles`)
  return data.profiles
}

export async function captureEnvProfile(name: string, scope: EnvScope): Promise<EnvProfileMeta> {
  return envData<EnvProfileMeta>(`${BASE}/env/profiles/${encodeURIComponent(name)}/capture`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ scope }),
  })
}

export async function applyEnvProfile(name: string, scope?: EnvScope): Promise<EnvProfileMeta> {
  return envData<EnvProfileMeta>(`${BASE}/env/profiles/${encodeURIComponent(name)}/apply`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ scope }),
  })
}

export async function deleteEnvProfile(name: string): Promise<boolean> {
  const data = await envData<{ name: string; deleted: boolean }>(`${BASE}/env/profiles/${encodeURIComponent(name)}`, {
    method: 'DELETE',
  })
  return data.deleted
}

export async function fetchEnvProfileDiff(name: string, scope?: EnvScope): Promise<EnvDiffResult> {
  const qs = new URLSearchParams()
  if (scope) qs.set('scope', scope)
  const data = await envData<{ diff: EnvDiffResult }>(
    `${BASE}/env/profiles/${encodeURIComponent(name)}/diff?${qs.toString()}`,
  )
  return data.diff
}

export async function fetchEnvSchema(): Promise<EnvSchema> {
  const data = await envData<{ schema: EnvSchema }>(`${BASE}/env/schema`)
  return data.schema
}

export async function addEnvSchemaRequired(pattern: string, warnOnly = false): Promise<EnvSchema> {
  const data = await envData<{ schema: EnvSchema }>(`${BASE}/env/schema/add-required`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ pattern, warn_only: warnOnly }),
  })
  return data.schema
}

export async function addEnvSchemaRegex(pattern: string, regex: string, warnOnly = false): Promise<EnvSchema> {
  const data = await envData<{ schema: EnvSchema }>(`${BASE}/env/schema/add-regex`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ pattern, regex, warn_only: warnOnly }),
  })
  return data.schema
}

export async function addEnvSchemaEnum(pattern: string, values: string[], warnOnly = false): Promise<EnvSchema> {
  const data = await envData<{ schema: EnvSchema }>(`${BASE}/env/schema/add-enum`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ pattern, values, warn_only: warnOnly }),
  })
  return data.schema
}

export async function removeEnvSchemaRule(pattern: string): Promise<EnvSchema> {
  const data = await envData<{ schema: EnvSchema }>(`${BASE}/env/schema/remove`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ pattern }),
  })
  return data.schema
}

export async function resetEnvSchema(): Promise<EnvSchema> {
  const data = await envData<{ schema: EnvSchema }>(`${BASE}/env/schema/reset`, {
    method: 'POST',
  })
  return data.schema
}

export async function runEnvValidate(scope: EnvScope, strict = false): Promise<EnvValidationReport> {
  const data = await envData<{ report: EnvValidationReport }>(`${BASE}/env/validate`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ scope, strict }),
  })
  return data.report
}

export async function fetchEnvAnnotations(): Promise<EnvAnnotationEntry[]> {
  const data = await envData<{ entries: EnvAnnotationEntry[] }>(`${BASE}/env/annotations`)
  return data.entries
}

export async function fetchEnvAnnotation(name: string): Promise<EnvAnnotationEntry> {
  return envData<EnvAnnotationEntry>(`${BASE}/env/annotations/${encodeURIComponent(name)}`)
}

export async function setEnvAnnotation(name: string, note: string): Promise<EnvAnnotationEntry> {
  return envData<EnvAnnotationEntry>(`${BASE}/env/annotations/${encodeURIComponent(name)}`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ note }),
  })
}

export async function deleteEnvAnnotation(name: string): Promise<boolean> {
  const data = await envData<{ name: string; deleted: boolean }>(`${BASE}/env/annotations/${encodeURIComponent(name)}`, {
    method: 'DELETE',
  })
  return data.deleted
}

export async function exportEnvLive(scope: EnvScope, format: EnvLiveExportFormat): Promise<string> {
  const qs = new URLSearchParams({ scope, format })
  const r = await request(`${BASE}/env/export-live?${qs.toString()}`)
  return r.text()
}

export async function expandEnvTemplate(payload: {
  template: string
  scope?: EnvScope
  validate_only?: boolean
}): Promise<EnvTemplateResult> {
  return envData<EnvTemplateResult>(`${BASE}/env/template/expand`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(payload),
  })
}

export async function runEnvCommand(payload: {
  cmd: string[]
  scope?: EnvScope
  env_files?: string[]
  set?: string[]
  schema_check?: boolean
  notify?: boolean
  cwd?: string
  max_output?: number
}): Promise<EnvRunResult> {
  const data = await envData<{ result: EnvRunResult }>(`${BASE}/env/run`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(payload),
  })
  return data.result
}

export function connectEnvWs(
  onEvent: (event: EnvWsEvent) => void,
  onClose?: (reason: string) => void,
): () => void {
  const proto = window.location.protocol === 'https:' ? 'wss' : 'ws'
  const wsUrl = `${proto}://${window.location.host}/api/env/ws`
  const ws = new WebSocket(wsUrl)

  ws.addEventListener('message', (evt) => {
    try {
      const payload = JSON.parse(String(evt.data))
      if (!payload || typeof payload.type !== 'string') return
      onEvent(payload as EnvWsEvent)
    } catch {
      // ignore malformed payload
    }
  })
  ws.addEventListener('close', () => {
    onClose?.('closed')
  })
  ws.addEventListener('error', () => {
    onClose?.('error')
  })

  return () => {
    if (ws.readyState === WebSocket.OPEN || ws.readyState === WebSocket.CONNECTING) {
      ws.close()
    }
  }
}


export async function fetchWorkspaceCapabilities(): Promise<WorkspaceCapabilities> {
  const r = await request(`${BASE}/workspaces/capabilities`)
  return r.json()
}

export async function fetchWorkspaceOverviewSummary(): Promise<WorkspaceOverviewSummary> {
  const r = await request(`${BASE}/workspaces/overview/summary`)
  return r.json()
}

export async function runWorkspaceTask(payload: WorkspaceTaskRunRequest): Promise<WorkspaceTaskRunResponse> {
  const r = await request(`${BASE}/workspaces/run`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(payload),
  })
  return r.json()
}

export async function previewGuardedTask(
  payload: GuardedTaskPreviewRequest,
): Promise<GuardedTaskPreviewResponse> {
  const r = await request(`${BASE}/workspaces/guarded/preview`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(payload),
  })
  return r.json()
}

export async function executeGuardedTask(
  payload: GuardedTaskExecuteRequest,
): Promise<GuardedTaskReceipt> {
  const r = await request(`${BASE}/workspaces/guarded/execute`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(payload),
  })
  return r.json()
}
