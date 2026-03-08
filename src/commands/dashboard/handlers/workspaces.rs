use super::*;

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const GUARDED_TASK_TTL_SECS: u64 = 300;

pub(in crate::commands::dashboard) trait TaskRunner: Send + Sync {
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
    seq: AtomicU64,
    ttl: Duration,
}

#[derive(Clone)]
struct PendingGuardedTask {
    workspace: String,
    action: String,
    target: String,
    execute_args: Vec<String>,
    expires_at: Instant,
}

#[derive(Serialize, Clone, Debug)]
pub(in crate::commands::dashboard) struct TaskProcessOutput {
    command_line: String,
    exit_code: Option<i32>,
    success: bool,
    stdout: String,
    stderr: String,
    duration_ms: u64,
}

#[derive(Deserialize, Clone, Debug)]
pub(in crate::commands::dashboard) struct WorkspaceTaskRunRequest {
    workspace: String,
    action: String,
    #[serde(default)]
    target: String,
    args: Vec<String>,
}

#[derive(Serialize, Clone, Debug)]
pub(in crate::commands::dashboard) struct WorkspaceTaskRunResponse {
    workspace: String,
    action: String,
    target: String,
    process: TaskProcessOutput,
}

#[derive(Deserialize, Clone, Debug)]
pub(in crate::commands::dashboard) struct GuardedTaskPreviewRequest {
    workspace: String,
    action: String,
    target: String,
    preview_args: Vec<String>,
    execute_args: Vec<String>,
    #[serde(default)]
    preview_summary: String,
}

#[derive(Serialize, Clone, Debug)]
pub(in crate::commands::dashboard) struct GuardedTaskPreviewResponse {
    token: String,
    workspace: String,
    action: String,
    target: String,
    preview_summary: String,
    process: TaskProcessOutput,
    expires_in_secs: u64,
}

#[derive(Deserialize, Clone, Debug)]
pub(in crate::commands::dashboard) struct GuardedTaskExecuteRequest {
    token: String,
    #[serde(default)]
    confirm: bool,
}

#[derive(Serialize, Clone, Debug)]
pub(in crate::commands::dashboard) struct GuardedTaskReceipt {
    token: String,
    workspace: String,
    action: String,
    target: String,
    audit_action: String,
    audited_at: u64,
    process: TaskProcessOutput,
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
                seq: AtomicU64::new(1),
                ttl: Duration::from_secs(GUARDED_TASK_TTL_SECS),
            }),
        }
    }

    fn run(&self, req: WorkspaceTaskRunRequest) -> Result<WorkspaceTaskRunResponse, (StatusCode, String)> {
        validate_workspace_action(&req.workspace, &req.action)?;
        if req.args.is_empty() {
            return Err((StatusCode::BAD_REQUEST, "args is empty".to_string()));
        }
        let process = self.inner.runner.run(&req.args);
        Ok(WorkspaceTaskRunResponse {
            workspace: req.workspace,
            action: req.action,
            target: req.target,
            process,
        })
    }

    fn preview(
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
        let preview_summary = if req.preview_summary.trim().is_empty() {
            format!("{} / {} / {}", req.workspace, req.action, req.target)
        } else {
            req.preview_summary.clone()
        };

        {
            let mut pending = self.inner.pending.lock().unwrap_or_else(|e| e.into_inner());
            pending.insert(
                token.clone(),
                PendingGuardedTask {
                    workspace: req.workspace.clone(),
                    action: req.action.clone(),
                    target: req.target.clone(),
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

        Ok(GuardedTaskPreviewResponse {
            token,
            workspace: req.workspace,
            action: req.action,
            target: req.target,
            preview_summary,
            process,
            expires_in_secs: self.inner.ttl.as_secs(),
        })
    }

    fn execute(
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
            return Err((StatusCode::NOT_FOUND, "preview token not found or expired".to_string()));
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

        Ok(GuardedTaskReceipt {
            token: req.token,
            workspace: pending.workspace,
            action: pending.action,
            target: pending.target,
            audit_action,
            audited_at: now_unix_secs(),
            process,
        })
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
    content.lines().filter(|line| !line.trim().is_empty()).count()
}

fn proxy_enabled_count() -> usize {
    let mut count = 0usize;
    if std::env::var("HTTP_PROXY").is_ok() || std::env::var("http_proxy").is_ok() {
        count += 1;
    }
    let cfg = config::load_config();
    if cfg.proxy.default_url.as_deref().is_some_and(|v| !v.trim().is_empty()) {
        count += 1;
    }
    count
}

pub(in crate::commands::dashboard) async fn workspace_capabilities() -> Json<WorkspaceCapabilities> {
    Json(capabilities())
}

pub(in crate::commands::dashboard) async fn workspace_overview_summary() -> Json<WorkspaceOverviewSummary> {
    let bookmarks = store::load(&store::db_path()).len();
    let tcp = ports::list_tcp_listeners();
    let udp = ports::list_udp_endpoints();
    let env_status = crate::env_core::EnvManager::new()
        .status_overview(crate::env_core::types::EnvScope::All)
        .ok();

    Json(WorkspaceOverviewSummary {
        bookmarks,
        tcp_ports: tcp.len(),
        udp_ports: udp.len(),
        proxy_enabled: proxy_enabled_count(),
        env_total_vars: env_status
            .as_ref()
            .and_then(|status| status.total_vars)
            .unwrap_or(0),
        env_snapshots: env_status.as_ref().map(|status| status.snapshots).unwrap_or(0),
        audit_entries: audit_entry_count(),
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
    use axum::routing::post;
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
            .route("/api/workspaces/run", post(workspace_run_task))
            .route("/api/workspaces/guarded/preview", post(workspace_preview_guarded_task))
            .route("/api/workspaces/guarded/execute", post(workspace_execute_guarded_task))
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
        let fake = Arc::new(FakeRunner::new(vec![ok_output("preview"), ok_output("execute")]));
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
            &to_bytes(preview_resp.into_body(), usize::MAX).await.unwrap(),
        )
        .unwrap();
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
                    .body(Body::from(serde_json::json!({ "token": token }).to_string()))
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

        let second_execute_resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/workspaces/guarded/execute")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({ "token": preview_json["token"], "confirm": true }).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(second_execute_resp.status(), StatusCode::NOT_FOUND);

        assert_eq!(fake.calls().len(), 2, "preview + execute should each run once");
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
        let json: serde_json::Value = serde_json::from_slice(&to_bytes(resp.into_body(), usize::MAX).await.unwrap()).unwrap();
        assert_eq!(json["workspace"], "files-security");
        assert_eq!(json["action"], "tree");
        assert_eq!(json["process"]["success"], true);
    }
}
