use super::*;

use std::collections::{BTreeMap, VecDeque};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const GUARDED_TASK_TTL_SECS: u64 = 300;
const TASK_HISTORY_LIMIT: usize = 64;

pub(in crate::commands::dashboard) trait TaskRunner:
    Send + Sync
{
    fn run(&self, args: &[String]) -> TaskProcessOutput;
}

#[derive(Clone, Default)]
pub(in crate::commands::dashboard) struct CurrentProcessTaskRunner;

impl TaskRunner for CurrentProcessTaskRunner {
    fn run(&self, args: &[String]) -> TaskProcessOutput {
        let exe = match std::env::current_exe() {
            Ok(v) => v,
            Err(err) => {
                return TaskProcessOutput {
                    command_line: String::new(),
                    exit_code: None,
                    success: false,
                    stdout: String::new(),
                    stderr: format!("resolve current exe failed: {err}"),
                    duration_ms: 0,
                };
            }
        };

        let mut full_args = vec!["--no-color".to_string(), "--non-interactive".to_string()];
        full_args.extend(args.iter().cloned());
        let command_line = format_command_line(&exe, &full_args);
        let started = Instant::now();
        let output = Command::new(&exe).args(&full_args).output();
        let duration_ms = started.elapsed().as_millis() as u64;

        match output {
            Ok(output) => TaskProcessOutput {
                command_line,
                exit_code: output.status.code(),
                success: output.status.success(),
                stdout: String::from_utf8_lossy(&output.stdout).trim().to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
                duration_ms,
            },
            Err(err) => TaskProcessOutput {
                command_line,
                exit_code: None,
                success: false,
                stdout: String::new(),
                stderr: format!("spawn command failed: {err}"),
                duration_ms,
            },
        }
    }
}

#[derive(Clone)]
pub(in crate::commands::dashboard) struct GuardedTaskService {
    inner: Arc<GuardedTaskServiceInner>,
}

struct GuardedTaskServiceInner {
    runner: Arc<dyn TaskRunner>,
    pending: Mutex<HashMap<String, PendingGuardedTask>>,
    history: Mutex<VecDeque<RecentTaskRecord>>,
    seq: AtomicU64,
    ttl: Duration,
}

#[derive(Clone)]
struct PendingGuardedTask {
    workspace: String,
    action: String,
    target: String,
    summary: String,
    preview_args: Vec<String>,
    execute_args: Vec<String>,
    expires_at: Instant,
}

#[derive(Serialize, Clone, Debug)]
pub(in crate::commands::dashboard) struct TaskProcessOutput {
    pub(in crate::commands::dashboard) command_line: String,
    pub(in crate::commands::dashboard) exit_code: Option<i32>,
    pub(in crate::commands::dashboard) success: bool,
    pub(in crate::commands::dashboard) stdout: String,
    pub(in crate::commands::dashboard) stderr: String,
    pub(in crate::commands::dashboard) duration_ms: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub(in crate::commands::dashboard) struct WorkspaceTaskRunRequest {
    pub(in crate::commands::dashboard) workspace: String,
    pub(in crate::commands::dashboard) action: String,
    #[serde(default)]
    pub(in crate::commands::dashboard) target: String,
    pub(in crate::commands::dashboard) args: Vec<String>,
}

#[derive(Serialize, Clone, Debug)]
pub(in crate::commands::dashboard) struct WorkspaceTaskRunResponse {
    pub(in crate::commands::dashboard) workspace: String,
    pub(in crate::commands::dashboard) action: String,
    pub(in crate::commands::dashboard) target: String,
    pub(in crate::commands::dashboard) process: TaskProcessOutput,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub(in crate::commands::dashboard) struct GuardedTaskPreviewRequest {
    pub(in crate::commands::dashboard) workspace: String,
    pub(in crate::commands::dashboard) action: String,
    pub(in crate::commands::dashboard) target: String,
    pub(in crate::commands::dashboard) preview_args: Vec<String>,
    pub(in crate::commands::dashboard) execute_args: Vec<String>,
    #[serde(default)]
    pub(in crate::commands::dashboard) preview_summary: String,
}

#[derive(Serialize, Clone, Debug)]
pub(in crate::commands::dashboard) struct GuardedTaskPreviewResponse {
    pub(in crate::commands::dashboard) token: String,
    pub(in crate::commands::dashboard) workspace: String,
    pub(in crate::commands::dashboard) action: String,
    pub(in crate::commands::dashboard) target: String,
    pub(in crate::commands::dashboard) phase: String,
    pub(in crate::commands::dashboard) status: String,
    pub(in crate::commands::dashboard) guarded: bool,
    pub(in crate::commands::dashboard) dry_run: bool,
    pub(in crate::commands::dashboard) ready_to_execute: bool,
    pub(in crate::commands::dashboard) summary: String,
    pub(in crate::commands::dashboard) preview_summary: String,
    pub(in crate::commands::dashboard) process: TaskProcessOutput,
    pub(in crate::commands::dashboard) expires_in_secs: u64,
}

#[derive(Deserialize, Clone, Debug)]
pub(in crate::commands::dashboard) struct GuardedTaskExecuteRequest {
    pub(in crate::commands::dashboard) token: String,
    #[serde(default)]
    pub(in crate::commands::dashboard) confirm: bool,
}

#[derive(Serialize, Clone, Debug)]
pub(in crate::commands::dashboard) struct GuardedTaskReceipt {
    pub(in crate::commands::dashboard) token: String,
    pub(in crate::commands::dashboard) workspace: String,
    pub(in crate::commands::dashboard) action: String,
    pub(in crate::commands::dashboard) target: String,
    pub(in crate::commands::dashboard) phase: String,
    pub(in crate::commands::dashboard) status: String,
    pub(in crate::commands::dashboard) guarded: bool,
    pub(in crate::commands::dashboard) dry_run: bool,
    pub(in crate::commands::dashboard) summary: String,
    pub(in crate::commands::dashboard) audit_action: String,
    pub(in crate::commands::dashboard) audited_at: u64,
    pub(in crate::commands::dashboard) process: TaskProcessOutput,
}

#[derive(Serialize, Clone, Debug)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(in crate::commands::dashboard) enum RecentTaskReplay {
    Run { request: WorkspaceTaskRunRequest },
    GuardedPreview { request: GuardedTaskPreviewRequest },
}

#[derive(Serialize, Clone, Debug)]
pub(in crate::commands::dashboard) struct RecentTaskRecord {
    id: String,
    workspace: String,
    action: String,
    target: String,
    mode: String,
    phase: String,
    status: String,
    guarded: bool,
    dry_run: bool,
    summary: String,
    created_at: u64,
    audit_action: Option<String>,
    process: TaskProcessOutput,
    replay: Option<RecentTaskReplay>,
}

#[derive(Serialize, Clone, Debug, Default)]
pub(in crate::commands::dashboard) struct RecentTaskStats {
    total: usize,
    succeeded: usize,
    failed: usize,
    dry_run: usize,
}

#[derive(Serialize, Clone, Debug)]
pub(in crate::commands::dashboard) struct RecentTaskListResponse {
    entries: Vec<RecentTaskRecord>,
    stats: RecentTaskStats,
}

#[derive(Deserialize, Clone, Debug, Default)]
pub(in crate::commands::dashboard) struct RecentTaskQuery {
    limit: Option<usize>,
    workspace: Option<String>,
}

#[derive(Serialize, Clone, Debug)]
pub(in crate::commands::dashboard) struct WorkspaceCapabilities {
    alias: bool,
    batch_rename: bool,
    crypt: bool,
    cstat: bool,
    diff: bool,
    fs: bool,
    img: bool,
    lock: bool,
    protect: bool,
    redirect: bool,
    tui: bool,
}

#[derive(Serialize, Clone, Debug)]
pub(in crate::commands::dashboard) struct WorkspaceOverviewSummary {
    bookmarks: usize,
    tcp_ports: usize,
    udp_ports: usize,
    proxy_enabled: usize,
    env_total_vars: usize,
    env_snapshots: usize,
    audit_entries: usize,
    recent_tasks: usize,
    failed_tasks: usize,
    dry_run_tasks: usize,
    workspaces: Vec<String>,
    capabilities: WorkspaceCapabilities,
}

impl GuardedTaskService {
    pub(in crate::commands::dashboard) fn new() -> Self {
        Self::with_runner(Arc::new(CurrentProcessTaskRunner))
    }

    pub(in crate::commands::dashboard) fn with_runner(runner: Arc<dyn TaskRunner>) -> Self {
        Self {
            inner: Arc::new(GuardedTaskServiceInner {
                runner,
                pending: Mutex::new(HashMap::new()),
                history: Mutex::new(VecDeque::with_capacity(TASK_HISTORY_LIMIT)),
                seq: AtomicU64::new(1),
                ttl: Duration::from_secs(GUARDED_TASK_TTL_SECS),
            }),
        }
    }

    pub(in crate::commands::dashboard) fn run(
        &self,
        req: WorkspaceTaskRunRequest,
    ) -> Result<WorkspaceTaskRunResponse, (StatusCode, String)> {
        validate_workspace_action(&req.workspace, &req.action)?;
        if req.args.is_empty() {
            return Err((StatusCode::BAD_REQUEST, "args is empty".to_string()));
        }
        let process = self.inner.runner.run(&req.args);
        let response = WorkspaceTaskRunResponse {
            workspace: req.workspace.clone(),
            action: req.action.clone(),
            target: req.target.clone(),
            process,
        };
        self.record_history(RecentTaskRecord {
            id: self.next_history_id(),
            workspace: response.workspace.clone(),
            action: response.action.clone(),
            target: response.target.clone(),
            mode: "run".to_string(),
            phase: "run".to_string(),
            status: if response.process.success {
                "succeeded".to_string()
            } else {
                "failed".to_string()
            },
            guarded: false,
            dry_run: false,
            summary: task_summary(&response.workspace, &response.action, &response.target),
            created_at: now_unix_secs(),
            audit_action: None,
            process: response.process.clone(),
            replay: Some(RecentTaskReplay::Run { request: req }),
        });
        Ok(response)
    }

    pub(in crate::commands::dashboard) fn preview_run(
        &self,
        req: WorkspaceTaskRunRequest,
        summary: Option<String>,
    ) -> Result<WorkspaceTaskRunResponse, (StatusCode, String)> {
        validate_workspace_action(&req.workspace, &req.action)?;
        if req.args.is_empty() {
            return Err((StatusCode::BAD_REQUEST, "args is empty".to_string()));
        }
        let process = self.inner.runner.run(&req.args);
        let response = WorkspaceTaskRunResponse {
            workspace: req.workspace.clone(),
            action: req.action.clone(),
            target: req.target.clone(),
            process,
        };
        self.record_history(RecentTaskRecord {
            id: self.next_history_id(),
            workspace: response.workspace.clone(),
            action: response.action.clone(),
            target: response.target.clone(),
            mode: "run".to_string(),
            phase: "preview".to_string(),
            status: if response.process.success {
                "previewed".to_string()
            } else {
                "failed".to_string()
            },
            guarded: false,
            dry_run: true,
            summary: summary.unwrap_or_else(|| {
                task_summary(&response.workspace, &response.action, &response.target)
            }),
            created_at: now_unix_secs(),
            audit_action: Some("dashboard.task.preview.run".to_string()),
            process: response.process.clone(),
            replay: None,
        });
        if !response.process.success {
            return Err((
                StatusCode::BAD_REQUEST,
                preview_failure_reason(&response.process),
            ));
        }
        Ok(response)
    }

    pub(in crate::commands::dashboard) fn preview(
        &self,
        req: GuardedTaskPreviewRequest,
    ) -> Result<GuardedTaskPreviewResponse, (StatusCode, String)> {
        validate_workspace_action(&req.workspace, &req.action)?;
        if req.target.trim().is_empty() {
            return Err((StatusCode::BAD_REQUEST, "target is empty".to_string()));
        }
        if req.preview_args.is_empty() {
            return Err((StatusCode::BAD_REQUEST, "preview_args is empty".to_string()));
        }
        if req.execute_args.is_empty() {
            return Err((StatusCode::BAD_REQUEST, "execute_args is empty".to_string()));
        }
        if req.preview_args == req.execute_args {
            return Err((
                StatusCode::BAD_REQUEST,
                "preview_args must differ from execute_args for guarded actions".to_string(),
            ));
        }

        self.evict_expired();
        let process = self.inner.runner.run(&req.preview_args);
        if !process.success {
            audit_guarded_event(
                "dashboard.task.preview",
                &req.target,
                &req.workspace,
                &req.action,
                &req.preview_args,
                "failed",
                &process.stderr,
            );
            return Err((StatusCode::BAD_REQUEST, preview_failure_reason(&process)));
        }

        let token = self.next_token();
        let summary = guarded_summary(
            &req.preview_summary,
            &req.workspace,
            &req.action,
            &req.target,
        );

        {
            let mut pending = self.inner.pending.lock().unwrap_or_else(|e| e.into_inner());
            pending.insert(
                token.clone(),
                PendingGuardedTask {
                    workspace: req.workspace.clone(),
                    action: req.action.clone(),
                    target: req.target.clone(),
                    summary: summary.clone(),
                    preview_args: req.preview_args.clone(),
                    execute_args: req.execute_args.clone(),
                    expires_at: Instant::now() + self.inner.ttl,
                },
            );
        }

        audit_guarded_event(
            "dashboard.task.preview",
            &req.target,
            &req.workspace,
            &req.action,
            &req.preview_args,
            "dry_run",
            "",
        );

        let response = GuardedTaskPreviewResponse {
            token,
            workspace: req.workspace.clone(),
            action: req.action.clone(),
            target: req.target.clone(),
            phase: "preview".to_string(),
            status: "previewed".to_string(),
            guarded: true,
            dry_run: true,
            ready_to_execute: true,
            summary: summary.clone(),
            preview_summary: summary,
            process,
            expires_in_secs: self.inner.ttl.as_secs(),
        };
        self.record_history(RecentTaskRecord {
            id: self.next_history_id(),
            workspace: response.workspace.clone(),
            action: response.action.clone(),
            target: response.target.clone(),
            mode: "guarded".to_string(),
            phase: response.phase.clone(),
            status: response.status.clone(),
            guarded: response.guarded,
            dry_run: response.dry_run,
            summary: response.summary.clone(),
            created_at: now_unix_secs(),
            audit_action: Some("dashboard.task.preview".to_string()),
            process: response.process.clone(),
            replay: Some(RecentTaskReplay::GuardedPreview { request: req }),
        });
        Ok(response)
    }

    pub(in crate::commands::dashboard) fn execute(
        &self,
        req: GuardedTaskExecuteRequest,
    ) -> Result<GuardedTaskReceipt, (StatusCode, String)> {
        if !req.confirm {
            return Err((StatusCode::BAD_REQUEST, "confirm must be true".to_string()));
        }
        self.evict_expired();
        let pending = {
            let mut map = self.inner.pending.lock().unwrap_or_else(|e| e.into_inner());
            map.remove(&req.token)
        };
        let Some(pending) = pending else {
            return Err((
                StatusCode::NOT_FOUND,
                "preview token not found or expired".to_string(),
            ));
        };

        let replay_request = GuardedTaskPreviewRequest {
            workspace: pending.workspace.clone(),
            action: pending.action.clone(),
            target: pending.target.clone(),
            preview_args: pending.preview_args.clone(),
            execute_args: pending.execute_args.clone(),
            preview_summary: pending.summary.clone(),
        };
        let process = self.inner.runner.run(&pending.execute_args);
        let audit_action = format!("dashboard.task.execute.{}", pending.action);
        let result = if process.success { "success" } else { "failed" };
        let reason = if process.success {
            String::new()
        } else {
            preview_failure_reason(&process)
        };
        audit_guarded_event(
            &audit_action,
            &pending.target,
            &pending.workspace,
            &pending.action,
            &pending.execute_args,
            result,
            &reason,
        );

        let receipt = GuardedTaskReceipt {
            token: req.token,
            workspace: pending.workspace,
            action: pending.action,
            target: pending.target,
            phase: "execute".to_string(),
            status: if process.success {
                "succeeded".to_string()
            } else {
                "failed".to_string()
            },
            guarded: true,
            dry_run: false,
            summary: pending.summary,
            audit_action,
            audited_at: now_unix_secs(),
            process,
        };
        self.record_history(RecentTaskRecord {
            id: self.next_history_id(),
            workspace: receipt.workspace.clone(),
            action: receipt.action.clone(),
            target: receipt.target.clone(),
            mode: "guarded".to_string(),
            phase: receipt.phase.clone(),
            status: receipt.status.clone(),
            guarded: receipt.guarded,
            dry_run: receipt.dry_run,
            summary: receipt.summary.clone(),
            created_at: receipt.audited_at,
            audit_action: Some(receipt.audit_action.clone()),
            process: receipt.process.clone(),
            replay: Some(RecentTaskReplay::GuardedPreview {
                request: replay_request,
            }),
        });
        Ok(receipt)
    }

    pub(in crate::commands::dashboard) fn recent_tasks(
        &self,
        limit: usize,
        workspace: Option<&str>,
    ) -> RecentTaskListResponse {
        let history = self.inner.history.lock().unwrap_or_else(|e| e.into_inner());
        let workspace = workspace
            .map(str::trim)
            .filter(|value| !value.is_empty());
        let filtered = history
            .iter()
            .filter(|entry| workspace.is_none_or(|value| entry.workspace == value))
            .cloned()
            .collect::<Vec<_>>();
        let stats = history_stats(&filtered);
        let entries = filtered
            .into_iter()
            .take(limit.max(1).min(100))
            .collect::<Vec<_>>();
        RecentTaskListResponse { stats, entries }
    }

    pub(in crate::commands::dashboard) fn recent_task_stats(&self) -> RecentTaskStats {
        let mut history = self.inner.history.lock().unwrap_or_else(|e| e.into_inner());
        history_stats(history.make_contiguous())
    }

    pub(in crate::commands::dashboard) fn failed_tasks(
        &self,
        limit: usize,
    ) -> Vec<RecentTaskRecord> {
        let history = self.inner.history.lock().unwrap_or_else(|e| e.into_inner());
        history
            .iter()
            .filter(|entry| entry.status == "failed")
            .take(limit.max(1).min(100))
            .cloned()
            .collect()
    }

    pub(in crate::commands::dashboard) fn guarded_receipts(
        &self,
        limit: usize,
    ) -> Vec<RecentTaskRecord> {
        let history = self.inner.history.lock().unwrap_or_else(|e| e.into_inner());
        history
            .iter()
            .filter(|entry| entry.guarded && entry.phase == "execute")
            .take(limit.max(1).min(100))
            .cloned()
            .collect()
    }

    fn record_history(&self, record: RecentTaskRecord) {
        let mut history = self.inner.history.lock().unwrap_or_else(|e| e.into_inner());
        history.push_front(record);
        while history.len() > TASK_HISTORY_LIMIT {
            history.pop_back();
        }
    }

    fn evict_expired(&self) {
        let now = Instant::now();
        let mut pending = self.inner.pending.lock().unwrap_or_else(|e| e.into_inner());
        pending.retain(|_, task| task.expires_at > now);
    }

    fn next_token(&self) -> String {
        let seq = self.inner.seq.fetch_add(1, Ordering::Relaxed);
        format!("guard-{}-{}", now_unix_secs(), seq)
    }

    fn next_history_id(&self) -> String {
        let seq = self.inner.seq.fetch_add(1, Ordering::Relaxed);
        format!("task-{}-{}", now_unix_secs(), seq)
    }
}

fn validate_workspace_action(workspace: &str, action: &str) -> Result<(), (StatusCode, String)> {
    if workspace.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "workspace is empty".to_string()));
    }
    if action.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "action is empty".to_string()));
    }
    Ok(())
}

fn format_command_line(exe: &std::path::Path, args: &[String]) -> String {
    let mut parts = Vec::with_capacity(args.len() + 1);
    parts.push(quote_arg(&exe.to_string_lossy()));
    for arg in args {
        parts.push(quote_arg(arg));
    }
    parts.join(" ")
}

fn quote_arg(value: &str) -> String {
    if value.is_empty() {
        return "\"\"".to_string();
    }
    let needs_quotes = value.chars().any(|ch| ch.is_whitespace() || ch == '"');
    if !needs_quotes {
        return value.to_string();
    }
    format!("\"{}\"", value.replace('"', "\\\""))
}

fn preview_failure_reason(process: &TaskProcessOutput) -> String {
    if !process.stderr.trim().is_empty() {
        return process.stderr.clone();
    }
    if !process.stdout.trim().is_empty() {
        return process.stdout.clone();
    }
    match process.exit_code {
        Some(code) => format!("command exited with code {code}"),
        None => "command failed".to_string(),
    }
}

fn guarded_summary(preview_summary: &str, workspace: &str, action: &str, target: &str) -> String {
    let summary = preview_summary.trim();
    if !summary.is_empty() {
        return summary.to_string();
    }
    task_summary(workspace, action, target)
}

fn task_summary(workspace: &str, action: &str, target: &str) -> String {
    if target.trim().is_empty() {
        return format!("{} / {}", workspace, action);
    }
    format!("{} / {} / {}", workspace, action, target)
}

fn history_stats(entries: &[RecentTaskRecord]) -> RecentTaskStats {
    let mut stats = RecentTaskStats::default();
    stats.total = entries.len();
    for entry in entries {
        if entry.status == "succeeded" {
            stats.succeeded += 1;
        }
        if entry.status == "failed" {
            stats.failed += 1;
        }
        if entry.dry_run {
            stats.dry_run += 1;
        }
    }
    stats
}

fn now_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn audit_guarded_event(
    action: &str,
    target: &str,
    workspace: &str,
    task_action: &str,
    args: &[String],
    result: &str,
    reason: &str,
) {
    let mut params = BTreeMap::new();
    params.insert(
        "workspace".to_string(),
        serde_json::Value::String(workspace.to_string()),
    );
    params.insert(
        "task_action".to_string(),
        serde_json::Value::String(task_action.to_string()),
    );
    params.insert(
        "args".to_string(),
        serde_json::Value::Array(
            args.iter()
                .cloned()
                .map(serde_json::Value::String)
                .collect(),
        ),
    );
    crate::security::audit::audit_log(
        action,
        target,
        "dashboard",
        crate::security::audit::AuditParams::Map(params),
        result,
        reason,
    );
}

fn capabilities() -> WorkspaceCapabilities {
    WorkspaceCapabilities {
        alias: cfg!(feature = "alias"),
        batch_rename: cfg!(feature = "batch_rename"),
        crypt: cfg!(feature = "crypt"),
        cstat: cfg!(feature = "cstat"),
        diff: cfg!(feature = "diff"),
        fs: cfg!(feature = "fs"),
        img: cfg!(feature = "img"),
        lock: cfg!(feature = "lock"),
        protect: cfg!(feature = "protect"),
        redirect: cfg!(feature = "redirect"),
        tui: cfg!(feature = "tui"),
    }
}

fn audit_entry_count() -> usize {
    let mut path = store::db_path();
    path.set_file_name("audit.jsonl");
    let Ok(content) = std::fs::read_to_string(path) else {
        return 0;
    };
    content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .count()
}

fn proxy_enabled_count() -> usize {
    let mut count = 0usize;
    if std::env::var("HTTP_PROXY").is_ok() || std::env::var("http_proxy").is_ok() {
        count += 1;
    }
    let cfg = config::load_config();
    if cfg
        .proxy
        .default_url
        .as_deref()
        .is_some_and(|v| !v.trim().is_empty())
    {
        count += 1;
    }
    count
}

pub(in crate::commands::dashboard) async fn workspace_capabilities() -> Json<WorkspaceCapabilities>
{
    Json(capabilities())
}

pub(in crate::commands::dashboard) async fn workspace_overview_summary(
    State(state): State<super::super::DashboardState>,
) -> Json<WorkspaceOverviewSummary> {
    let bookmarks = store::load(&store::db_path()).len();
    let tcp = ports::list_tcp_listeners();
    let udp = ports::list_udp_endpoints();
    let env_status = crate::env_core::EnvManager::new()
        .status_overview(crate::env_core::types::EnvScope::All)
        .ok();
    let task_stats = state.guarded_tasks().recent_task_stats();

    Json(WorkspaceOverviewSummary {
        bookmarks,
        tcp_ports: tcp.len(),
        udp_ports: udp.len(),
        proxy_enabled: proxy_enabled_count(),
        env_total_vars: env_status
            .as_ref()
            .and_then(|status| status.total_vars)
            .unwrap_or(0),
        env_snapshots: env_status
            .as_ref()
            .map(|status| status.snapshots)
            .unwrap_or(0),
        audit_entries: audit_entry_count(),
        recent_tasks: task_stats.total,
        failed_tasks: task_stats.failed,
        dry_run_tasks: task_stats.dry_run,
        workspaces: vec![
            "overview".to_string(),
            "paths-context".to_string(),
            "network-proxy".to_string(),
            "environment-config".to_string(),
            "files-security".to_string(),
            "integration-automation".to_string(),
            "media-conversion".to_string(),
            "statistics-diagnostics".to_string(),
        ],
        capabilities: capabilities(),
    })
}

pub(in crate::commands::dashboard) async fn workspace_recent_tasks(
    State(state): State<super::super::DashboardState>,
    Query(query): Query<RecentTaskQuery>,
) -> Json<RecentTaskListResponse> {
    Json(
        state
            .guarded_tasks()
            .recent_tasks(query.limit.unwrap_or(20), query.workspace.as_deref()),
    )
}

pub(in crate::commands::dashboard) async fn workspace_run_task(
    State(state): State<super::super::DashboardState>,
    Json(req): Json<WorkspaceTaskRunRequest>,
) -> Response {
    let service = state.guarded_tasks();
    match tokio::task::spawn_blocking(move || service.run(req)).await {
        Ok(Ok(resp)) => Json(resp).into_response(),
        Ok(Err((status, msg))) => (status, msg).into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("task join failed: {err}"),
        )
            .into_response(),
    }
}

pub(in crate::commands::dashboard) async fn workspace_preview_guarded_task(
    State(state): State<super::super::DashboardState>,
    Json(req): Json<GuardedTaskPreviewRequest>,
) -> Response {
    let service = state.guarded_tasks();
    match tokio::task::spawn_blocking(move || service.preview(req)).await {
        Ok(Ok(resp)) => Json(resp).into_response(),
        Ok(Err((status, msg))) => (status, msg).into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("preview join failed: {err}"),
        )
            .into_response(),
    }
}

pub(in crate::commands::dashboard) async fn workspace_execute_guarded_task(
    State(state): State<super::super::DashboardState>,
    Json(req): Json<GuardedTaskExecuteRequest>,
) -> Response {
    let service = state.guarded_tasks();
    match tokio::task::spawn_blocking(move || service.execute(req)).await {
        Ok(Ok(resp)) => Json(resp).into_response(),
        Ok(Err((status, msg))) => (status, msg).into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("execute join failed: {err}"),
        )
            .into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Router;
    use axum::body::{Body, to_bytes};
    use axum::http::{Request, StatusCode};
    use axum::routing::{get, post};
    use tower::util::ServiceExt;

    #[derive(Clone)]
    struct FakeRunner {
        outputs: Arc<Mutex<Vec<TaskProcessOutput>>>,
        calls: Arc<Mutex<Vec<Vec<String>>>>,
    }

    impl FakeRunner {
        fn new(outputs: Vec<TaskProcessOutput>) -> Self {
            Self {
                outputs: Arc::new(Mutex::new(outputs)),
                calls: Arc::new(Mutex::new(Vec::new())),
            }
        }

        fn calls(&self) -> Vec<Vec<String>> {
            self.calls.lock().unwrap().clone()
        }
    }

    impl TaskRunner for FakeRunner {
        fn run(&self, args: &[String]) -> TaskProcessOutput {
            self.calls.lock().unwrap().push(args.to_vec());
            self.outputs.lock().unwrap().remove(0)
        }
    }

    fn ok_output(label: &str) -> TaskProcessOutput {
        TaskProcessOutput {
            command_line: label.to_string(),
            exit_code: Some(0),
            success: true,
            stdout: format!("ok:{label}"),
            stderr: String::new(),
            duration_ms: 1,
        }
    }

    fn fail_output(label: &str) -> TaskProcessOutput {
        TaskProcessOutput {
            command_line: label.to_string(),
            exit_code: Some(2),
            success: false,
            stdout: String::new(),
            stderr: format!("fail:{label}"),
            duration_ms: 1,
        }
    }

    fn test_router(runner: Arc<dyn TaskRunner>) -> Router {
        let state = crate::commands::dashboard::DashboardState::for_tests(runner);
        Router::new()
            .route("/api/workspaces/tasks/recent", get(workspace_recent_tasks))
            .route("/api/workspaces/run", post(workspace_run_task))
            .route(
                "/api/workspaces/guarded/preview",
                post(workspace_preview_guarded_task),
            )
            .route(
                "/api/workspaces/guarded/execute",
                post(workspace_execute_guarded_task),
            )
            .with_state(state)
    }

    #[tokio::test]
    async fn guarded_preview_rejects_identical_args() {
        let runner = Arc::new(FakeRunner::new(vec![]));
        let app = test_router(runner);
        let body = serde_json::json!({
            "workspace": "files-security",
            "action": "rm",
            "target": "C:/tmp/demo.txt",
            "preview_args": ["rm", "C:/tmp/demo.txt"],
            "execute_args": ["rm", "C:/tmp/demo.txt"]
        });

        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/workspaces/guarded/preview")
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn guarded_execute_requires_confirm_and_valid_token() {
        let fake = Arc::new(FakeRunner::new(vec![
            ok_output("preview"),
            ok_output("execute"),
        ]));
        let app = test_router(fake.clone());
        let preview_body = serde_json::json!({
            "workspace": "files-security",
            "action": "rm",
            "target": "C:/tmp/demo.txt",
            "preview_args": ["rm", "--what-if", "C:/tmp/demo.txt"],
            "execute_args": ["rm", "C:/tmp/demo.txt"],
            "preview_summary": "Delete C:/tmp/demo.txt"
        });

        let preview_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/workspaces/guarded/preview")
                    .header("content-type", "application/json")
                    .body(Body::from(preview_body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(preview_resp.status(), StatusCode::OK);
        let preview_json: serde_json::Value = serde_json::from_slice(
            &to_bytes(preview_resp.into_body(), usize::MAX)
                .await
                .unwrap(),
        )
        .unwrap();
        assert_eq!(preview_json["phase"], "preview");
        assert_eq!(preview_json["status"], "previewed");
        assert_eq!(preview_json["guarded"], true);
        assert_eq!(preview_json["dry_run"], true);
        assert_eq!(preview_json["ready_to_execute"], true);
        assert_eq!(preview_json["summary"], "Delete C:/tmp/demo.txt");
        assert_eq!(preview_json["preview_summary"], "Delete C:/tmp/demo.txt");
        let token = preview_json
            .get("token")
            .and_then(|value| value.as_str())
            .unwrap()
            .to_string();

        let missing_confirm_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/workspaces/guarded/execute")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({ "token": token }).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(missing_confirm_resp.status(), StatusCode::BAD_REQUEST);

        let execute_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/workspaces/guarded/execute")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({ "token": token, "confirm": true }).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(execute_resp.status(), StatusCode::OK);
        let execute_json: serde_json::Value = serde_json::from_slice(
            &to_bytes(execute_resp.into_body(), usize::MAX)
                .await
                .unwrap(),
        )
        .unwrap();
        assert_eq!(execute_json["phase"], "execute");
        assert_eq!(execute_json["status"], "succeeded");
        assert_eq!(execute_json["guarded"], true);
        assert_eq!(execute_json["dry_run"], false);
        assert_eq!(execute_json["summary"], "Delete C:/tmp/demo.txt");
        assert_eq!(execute_json["process"]["success"], true);

        let second_execute_resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/workspaces/guarded/execute")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({ "token": preview_json["token"], "confirm": true })
                            .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(second_execute_resp.status(), StatusCode::NOT_FOUND);

        assert_eq!(
            fake.calls().len(),
            2,
            "preview + execute should each run once"
        );
    }

    #[tokio::test]
    async fn guarded_preview_refuses_failed_dry_run() {
        let runner = Arc::new(FakeRunner::new(vec![fail_output("preview")])) as Arc<dyn TaskRunner>;
        let app = test_router(runner);
        let body = serde_json::json!({
            "workspace": "files-security",
            "action": "rm",
            "target": "C:/tmp/demo.txt",
            "preview_args": ["rm", "--what-if", "C:/tmp/demo.txt"],
            "execute_args": ["rm", "C:/tmp/demo.txt"]
        });

        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/workspaces/guarded/preview")
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn guarded_execute_returns_failed_receipt_when_command_fails() {
        let fake = Arc::new(FakeRunner::new(vec![
            ok_output("preview"),
            fail_output("execute"),
        ])) as Arc<dyn TaskRunner>;
        let app = test_router(fake);
        let preview_body = serde_json::json!({
            "workspace": "files-security",
            "action": "rm",
            "target": "C:/tmp/demo.txt",
            "preview_args": ["rm", "--what-if", "C:/tmp/demo.txt"],
            "execute_args": ["rm", "C:/tmp/demo.txt"],
            "preview_summary": "Delete C:/tmp/demo.txt"
        });

        let preview_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/workspaces/guarded/preview")
                    .header("content-type", "application/json")
                    .body(Body::from(preview_body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(preview_resp.status(), StatusCode::OK);
        let preview_json: serde_json::Value = serde_json::from_slice(
            &to_bytes(preview_resp.into_body(), usize::MAX)
                .await
                .unwrap(),
        )
        .unwrap();

        let execute_resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/workspaces/guarded/execute")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({ "token": preview_json["token"], "confirm": true })
                            .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(execute_resp.status(), StatusCode::OK);

        let execute_json: serde_json::Value = serde_json::from_slice(
            &to_bytes(execute_resp.into_body(), usize::MAX)
                .await
                .unwrap(),
        )
        .unwrap();
        assert_eq!(execute_json["status"], "failed");
        assert_eq!(execute_json["summary"], "Delete C:/tmp/demo.txt");
        assert_eq!(execute_json["process"]["success"], false);
    }

    #[tokio::test]
    async fn run_task_returns_process_payload() {
        let fake = Arc::new(FakeRunner::new(vec![ok_output("tree")])) as Arc<dyn TaskRunner>;
        let app = test_router(fake);
        let body = serde_json::json!({
            "workspace": "files-security",
            "action": "tree",
            "target": "C:/tmp",
            "args": ["tree", "C:/tmp"]
        });

        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/workspaces/run")
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json: serde_json::Value =
            serde_json::from_slice(&to_bytes(resp.into_body(), usize::MAX).await.unwrap()).unwrap();
        assert_eq!(json["workspace"], "files-security");
        assert_eq!(json["action"], "tree");
        assert_eq!(json["process"]["success"], true);
    }

    #[tokio::test]
    async fn recent_tasks_endpoint_returns_latest_entries_and_stats() {
        let fake = Arc::new(FakeRunner::new(vec![
            ok_output("tree"),
            ok_output("preview"),
            ok_output("execute"),
        ])) as Arc<dyn TaskRunner>;
        let app = test_router(fake);

        let run_body = serde_json::json!({
            "workspace": "files-security",
            "action": "tree",
            "target": "C:/tmp",
            "args": ["tree", "C:/tmp"]
        });
        let run_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/workspaces/run")
                    .header("content-type", "application/json")
                    .body(Body::from(run_body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(run_resp.status(), StatusCode::OK);

        let preview_body = serde_json::json!({
            "workspace": "files-security",
            "action": "rm",
            "target": "C:/tmp/demo.txt",
            "preview_args": ["rm", "--what-if", "C:/tmp/demo.txt"],
            "execute_args": ["rm", "C:/tmp/demo.txt"],
            "preview_summary": "Delete C:/tmp/demo.txt"
        });
        let preview_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/workspaces/guarded/preview")
                    .header("content-type", "application/json")
                    .body(Body::from(preview_body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(preview_resp.status(), StatusCode::OK);
        let preview_json: serde_json::Value = serde_json::from_slice(
            &to_bytes(preview_resp.into_body(), usize::MAX)
                .await
                .unwrap(),
        )
        .unwrap();

        let execute_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/workspaces/guarded/execute")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({ "token": preview_json["token"], "confirm": true })
                            .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(execute_resp.status(), StatusCode::OK);

        let recent_resp = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/workspaces/tasks/recent?limit=10")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(recent_resp.status(), StatusCode::OK);
        let recent_json: serde_json::Value =
            serde_json::from_slice(&to_bytes(recent_resp.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        let entries = recent_json["entries"].as_array().unwrap();
        assert_eq!(entries.len(), 3);
        assert_eq!(recent_json["stats"]["total"], 3);
        assert_eq!(recent_json["stats"]["dry_run"], 1);
        assert_eq!(entries[0]["phase"], "execute");
        assert_eq!(entries[1]["phase"], "preview");
        assert_eq!(entries[2]["phase"], "run");
        assert_eq!(entries[0]["replay"]["kind"], "guarded_preview");
        assert_eq!(entries[2]["replay"]["kind"], "run");
    }

    #[tokio::test]
    async fn recent_tasks_endpoint_supports_workspace_filter() {
        let fake = Arc::new(FakeRunner::new(vec![ok_output("tree"), ok_output("recent")]))
            as Arc<dyn TaskRunner>;
        let app = test_router(fake);

        let files_body = serde_json::json!({
            "workspace": "files-security",
            "action": "tree",
            "target": "C:/tmp",
            "args": ["tree", "C:/tmp"]
        });
        let files_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/workspaces/run")
                    .header("content-type", "application/json")
                    .body(Body::from(files_body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(files_resp.status(), StatusCode::OK);

        let paths_body = serde_json::json!({
            "workspace": "paths-context",
            "action": "recent",
            "target": "",
            "args": ["recent"]
        });
        let paths_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/workspaces/run")
                    .header("content-type", "application/json")
                    .body(Body::from(paths_body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(paths_resp.status(), StatusCode::OK);

        let recent_resp = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/workspaces/tasks/recent?workspace=files-security&limit=10")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(recent_resp.status(), StatusCode::OK);
        let recent_json: serde_json::Value =
            serde_json::from_slice(&to_bytes(recent_resp.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(recent_json["stats"]["total"], 1);
        assert_eq!(recent_json["entries"].as_array().unwrap().len(), 1);
        assert_eq!(recent_json["entries"][0]["workspace"], "files-security");
    }


    #[tokio::test]
    async fn recent_tasks_endpoint_captures_failed_run_status() {
        let fake = Arc::new(FakeRunner::new(vec![fail_output("run")])) as Arc<dyn TaskRunner>;
        let app = test_router(fake);
        let body = serde_json::json!({
            "workspace": "files-security",
            "action": "tree",
            "target": "C:/tmp",
            "args": ["tree", "C:/tmp"]
        });

        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/workspaces/run")
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let recent_resp = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/workspaces/tasks/recent")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(recent_resp.status(), StatusCode::OK);
        let recent_json: serde_json::Value =
            serde_json::from_slice(&to_bytes(recent_resp.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(recent_json["stats"]["failed"], 1);
        assert_eq!(recent_json["entries"][0]["status"], "failed");
        assert_eq!(recent_json["entries"][0]["replay"]["kind"], "run");
    }
}
