export interface Bookmark {
  name: string
  path: string
  tags: string[]
  visits: number
  last_visited: number
}

export interface PortInfo {
  port: number
  pid: number
  name: string
  exe_path: string
  cmdline: string
  cwd: string
  protocol: string
}

export interface PortsResponse {
  tcp: PortInfo[]
  udp: PortInfo[]
}

export interface ProxyItem {
  tool: string
  status: string
  address: string
}

export interface ProxyConfig {
  defaultUrl?: string | null
  noproxy?: string | null
}

export interface ProxyTestItem {
  label: string
  ok: boolean
  ms: number
  error: string
}

export interface TreeConfig {
  defaultDepth?: number | null
  excludeNames?: string[]
}

export interface ProtectRule {
  path: string
  deny: string[]
  require: string[]
}

export interface ProtectConfig {
  rules: ProtectRule[]
}

export interface AuditEntry {
  timestamp: number
  action: string
  target: string
  user: string
  params: string
  result: string
  reason: string
}

export interface AuditStats {
  total: number
  by_action: Record<string, number>
  by_result: Record<string, number>
}

export interface AuditResponse {
  entries: AuditEntry[]
  stats: AuditStats
  next_cursor?: string | null
}

export interface RedirectDryRunItem {
  action: string
  src: string
  dst: string
  rule: string
  result: string
  reason: string
}

export interface RedirectDryRunStats {
  total: number
  dry_run: number
  skipped: number
  failed: number
}

export interface RedirectDryRunResponse {
  results: RedirectDryRunItem[]
  stats: RedirectDryRunStats
}

export interface MatchCondition {
  ext: string[]
  glob?: string | null
  regex?: string | null
  size?: string | null
  age?: string | null
}

export interface RedirectRule {
  name: string
  match: MatchCondition
  dest: string
}

export interface RedirectProfile {
  rules: RedirectRule[]
  unmatched: string
  on_conflict: string
  recursive?: boolean
  max_depth?: number
}

export interface RedirectConfig {
  profiles: Record<string, RedirectProfile>
}

export interface GlobalConfig {
  tree: TreeConfig
  proxy: ProxyConfig
  protect?: ProtectConfig
  redirect?: RedirectConfig
}

/* ── Diff ─────────────────────────────────────── */

export interface DiffApiRequest {
  old_path: string
  new_path: string
  mode?: 'auto' | 'line' | 'ast'
  algorithm?: 'histogram' | 'myers' | 'minimal' | 'patience'
  context?: number
  ignore_space_change?: boolean
  ignore_all_space?: boolean
  ignore_blank_lines?: boolean
  strip_trailing_cr?: boolean
  force_text?: boolean
}

export interface DiffStats {
  added: number
  removed: number
  modified: number
  unchanged: number
  unit: 'line' | 'symbol'
}

export interface DiffLine {
  tag: 'context' | 'add' | 'remove'
  content: string
}

export interface DiffHunk {
  kind: 'added' | 'removed' | 'modified' | 'unchanged'
  symbol?: string
  symbol_type?: string
  section?: string
  old_start: number
  old_count: number
  new_start: number
  new_count: number
  lines: DiffLine[]
}

export interface DiffResult {
  kind: 'identical' | 'line' | 'ast' | 'binary'
  stats: DiffStats
  hunks: DiffHunk[]
  actual_algorithm: string
  identical_with_filters: boolean
}

export type ConfigDiffStatus = 'added' | 'removed' | 'modified' | 'unchanged'
export type ConfigDiffNodeKind = 'object' | 'array' | 'value'

export interface ConfigDiffNode {
  key: string
  path: string
  kind: ConfigDiffNodeKind
  status: ConfigDiffStatus
  oldValue?: unknown
  newValue?: unknown
  children?: ConfigDiffNode[]
}

export interface ConfigDiffStats {
  added: number
  removed: number
  modified: number
  unchanged: number
}

export interface FileEntry {
  name: string
  is_dir: boolean
  size?: number
}

export interface FileSearchEntry {
  path: string
  name: string
  is_dir: boolean
  size?: number
}

export interface DiffFileInfo {
  path: string
  name: string
  size: number
  line_count: number | null
  language: string
  file_class: 'config' | 'code' | 'unknown'
  modified: number | null
}

export interface DiffFileContent {
  path: string
  offset: number
  limit: number
  total_lines: number
  truncated: boolean
  is_binary: boolean
  lines: string[]
}

export interface ConvertFileRequest {
  path: string
  to_format: 'json' | 'json5' | 'yaml' | 'toml'
  preview?: boolean
}

export interface ConvertFileResponse {
  from_format: string
  to_format: string
  content: string
  written_path?: string
}

export interface ValidateErrorItem {
  line?: number
  col?: number
  message: string
}

export interface ValidateFileRequest {
  path?: string
  content?: string
  format?: 'json' | 'json5' | 'yaml' | 'toml'
}

export interface ValidateFileResponse {
  valid: boolean
  errors: ValidateErrorItem[]
  format?: string
}

export interface DiffWsEvent {
  type: 'connected' | 'refresh' | 'file_changed'
  path?: string
}

/* ── Env ─────────────────────────────────────── */

export type EnvScope = 'user' | 'system' | 'all'
export type EnvVarKind =
  | 'url'
  | 'path'
  | 'path_list'
  | 'boolean'
  | 'secret'
  | 'json'
  | 'email'
  | 'version'
  | 'integer'
  | 'float'

export interface EnvVar {
  scope: EnvScope
  name: string
  raw_value: string
  reg_type: number
  inferred_kind?: EnvVarKind
}

export interface EnvSnapshotMeta {
  id: string
  description: string
  created_at: string
  path: string
}

export interface EnvSnapshotPruneResult {
  removed: number
  remaining: number
}

export interface EnvDoctorIssue {
  kind: 'path_missing' | 'path_duplicate' | 'user_shadows_system'
  severity: string
  scope: EnvScope
  name: string
  message: string
  fixable: boolean
}

export interface EnvDoctorReport {
  scope: EnvScope
  issues: EnvDoctorIssue[]
  errors: number
  warnings: number
  fixable: number
}

export interface EnvDoctorFixResult {
  scope: EnvScope
  fixed: number
  details: string[]
}

export interface EnvImportResult {
  dry_run: boolean
  added: number
  updated: number
  skipped: number
  changed_names: string[]
}

export interface EnvDiffPathSegment {
  segment: string
  kind: 'added' | 'removed' | 'changed'
}

export interface EnvDiffEntry {
  name: string
  kind: 'added' | 'removed' | 'changed'
  old_value?: string
  new_value?: string
  path_diff: EnvDiffPathSegment[]
}

export interface EnvDiffResult {
  added: EnvDiffEntry[]
  removed: EnvDiffEntry[]
  changed: EnvDiffEntry[]
}

export interface EnvDepTree {
  scope: EnvScope
  root: string
  lines: string[]
  missing: string[]
  cycles: string[]
}

export interface EnvAuditEntry {
  at: string
  action: string
  scope: EnvScope
  result: string
  name?: string
  message?: string
}

export interface EnvProfileMeta {
  name: string
  scope: EnvScope
  created_at: string
  path: string
  var_count: number
}

export interface EnvSchemaRule {
  pattern: string
  required?: boolean
  warn_only?: boolean
  regex?: string
  enum_values?: string[]
}

export interface EnvSchema {
  rules: EnvSchemaRule[]
}

export interface EnvSchemaViolation {
  name?: string
  pattern: string
  kind: string
  message: string
  severity: 'warning' | 'error'
}

export interface EnvValidationReport {
  scope: EnvScope
  total_vars: number
  violations: EnvSchemaViolation[]
  errors: number
  warnings: number
  passed: boolean
}

export interface EnvAnnotationEntry {
  name: string
  note: string
}

export interface EnvTemplateValidationReport {
  input: string
  references: string[]
  missing: string[]
  cycles: string[][]
  valid: boolean
}

export interface EnvTemplateResult {
  output?: string | null
  report: EnvTemplateValidationReport
}

export type EnvLiveExportFormat = 'dotenv' | 'sh' | 'json' | 'reg'

export interface EnvRunResult {
  command_line: string
  exit_code: number | null
  success: boolean
  stdout?: string
  stderr?: string
  truncated: boolean
}

export interface EnvStatusSummary {
  scope: EnvScope
  user_vars: number | null
  system_vars: number | null
  total_vars: number | null
  snapshots: number
  latest_snapshot_id: string | null
  latest_snapshot_at: string | null
  profiles: number
  schema_rules: number
  annotations: number
  audit_entries: number
  last_audit_at: string | null
  notes: string[]
}

export interface EnvWsEvent {
  type: 'connected' | 'env.refresh' | 'changed' | 'snapshot' | 'doctor' | 'import' | 'export' | 'diff'
  scope?: EnvScope
  at?: string
  name?: string
  message?: string
}


export type WorkspaceKey =
  | 'overview'
  | 'paths-context'
  | 'network-proxy'
  | 'environment-config'
  | 'files-security'
  | 'integration-automation'
  | 'media-conversion'
  | 'statistics-diagnostics'

export interface TaskProcessOutput {
  command_line: string
  exit_code: number | null
  success: boolean
  stdout: string
  stderr: string
  duration_ms: number
}

export interface WorkspaceTaskRunRequest {
  workspace: string
  action: string
  target?: string
  args: string[]
}

export interface WorkspaceTaskRunResponse {
  workspace: string
  action: string
  target: string
  process: TaskProcessOutput
}

export interface GuardedTaskPreviewRequest {
  workspace: string
  action: string
  target: string
  preview_args: string[]
  execute_args: string[]
  preview_summary?: string
}

export interface GuardedTaskPreviewResponse {
  token: string
  workspace: string
  action: string
  target: string
  preview_summary: string
  process: TaskProcessOutput
  expires_in_secs: number
}

export interface GuardedTaskExecuteRequest {
  token: string
  confirm: boolean
}

export interface GuardedTaskReceipt {
  token: string
  workspace: string
  action: string
  target: string
  audit_action: string
  audited_at: number
  process: TaskProcessOutput
}

export interface WorkspaceCapabilities {
  alias: boolean
  batch_rename: boolean
  crypt: boolean
  cstat: boolean
  diff: boolean
  fs: boolean
  img: boolean
  lock: boolean
  protect: boolean
  redirect: boolean
  tui: boolean
}

export interface WorkspaceOverviewSummary {
  bookmarks: number
  tcp_ports: number
  udp_ports: number
  proxy_enabled: number
  env_total_vars: number
  env_snapshots: number
  audit_entries: number
  workspaces: string[]
  capabilities: WorkspaceCapabilities
}
