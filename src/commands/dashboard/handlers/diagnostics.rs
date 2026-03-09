use axum::Json;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

use crate::env_core::EnvManager;
use crate::env_core::types::{DoctorIssue, DoctorReport, EnvScope};

use super::{AuditEntry, RecentTaskRecord, audit_entry_count, latest_audit_entries};

#[derive(Deserialize, Clone, Debug, Default)]
pub(in crate::commands::dashboard) struct DiagnosticsQuery {
    scope: Option<String>,
    audit_limit: Option<usize>,
    task_limit: Option<usize>,
}

#[derive(Serialize, Clone, Debug)]
pub(in crate::commands::dashboard) struct DiagnosticsDoctorSummary {
    scope: EnvScope,
    issues: Vec<DoctorIssue>,
    errors: usize,
    warnings: usize,
    fixable: usize,
    load_error: Option<String>,
}

#[derive(Serialize, Clone, Debug)]
pub(in crate::commands::dashboard) struct DiagnosticsOverview {
    doctor_issues: usize,
    doctor_errors: usize,
    doctor_warnings: usize,
    doctor_fixable: usize,
    recent_failed_tasks: usize,
    recent_guarded_receipts: usize,
    recent_governance_alerts: usize,
    audit_entries: usize,
    urgent_items: usize,
}

#[derive(Serialize, Clone, Debug)]
pub(in crate::commands::dashboard) struct DiagnosticsSummaryResponse {
    generated_at: u64,
    overview: DiagnosticsOverview,
    doctor: DiagnosticsDoctorSummary,
    audit_timeline: Vec<AuditEntry>,
    failed_tasks: Vec<RecentTaskRecord>,
    guarded_receipts: Vec<RecentTaskRecord>,
    governance_alerts: Vec<RecentTaskRecord>,
}

pub(in crate::commands::dashboard) async fn workspace_diagnostics_summary(
    State(state): State<crate::commands::dashboard::DashboardState>,
    Query(query): Query<DiagnosticsQuery>,
) -> Result<Json<DiagnosticsSummaryResponse>, (StatusCode, String)> {
    let scope = match query.scope.as_deref() {
        Some(raw) => EnvScope::from_str(raw)
            .map_err(|err| (StatusCode::BAD_REQUEST, format!("invalid scope: {err}")))?,
        None => EnvScope::All,
    };
    let audit_limit = query.audit_limit.unwrap_or(10).clamp(1, 100);
    let task_limit = query.task_limit.unwrap_or(8).clamp(1, 50);

    let doctor = summarize_doctor(scope);
    let audit_timeline = latest_audit_entries(audit_limit);
    let failed_tasks = state.guarded_tasks().failed_tasks(task_limit);
    let guarded_receipts = state.guarded_tasks().guarded_receipts(task_limit);
    let governance_alerts = state.guarded_tasks().governance_alerts(task_limit);
    let overview = DiagnosticsOverview {
        doctor_issues: doctor.issues.len(),
        doctor_errors: doctor.errors,
        doctor_warnings: doctor.warnings,
        doctor_fixable: doctor.fixable,
        recent_failed_tasks: failed_tasks.len(),
        recent_guarded_receipts: guarded_receipts.len(),
        recent_governance_alerts: governance_alerts.len(),
        audit_entries: audit_entry_count(),
        urgent_items: doctor.errors + failed_tasks.len(),
    };

    Ok(Json(DiagnosticsSummaryResponse {
        generated_at: now_unix_secs_local(),
        overview,
        doctor,
        audit_timeline,
        failed_tasks,
        guarded_receipts,
        governance_alerts,
    }))
}

fn summarize_doctor(scope: EnvScope) -> DiagnosticsDoctorSummary {
    match EnvManager::new().doctor_run(scope) {
        Ok(DoctorReport {
            scope,
            issues,
            errors,
            warnings,
            fixable,
        }) => DiagnosticsDoctorSummary {
            scope,
            issues,
            errors,
            warnings,
            fixable,
            load_error: None,
        },
        Err(err) => DiagnosticsDoctorSummary {
            scope,
            issues: Vec::new(),
            errors: 0,
            warnings: 0,
            fixable: 0,
            load_error: Some(err.to_string()),
        },
    }
}

fn now_unix_secs_local() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    use axum::Router;
    use axum::body::{Body, to_bytes};
    use axum::http::Request;
    use axum::routing::get;
    use serial_test::serial;
    use std::sync::Arc;
    use tower::ServiceExt;

    use crate::commands::dashboard::handlers::TaskProcessOutput;

    #[derive(Default)]
    struct FakeRunner {
        outputs: std::sync::Mutex<Vec<TaskProcessOutput>>,
    }

    impl FakeRunner {
        fn new(outputs: Vec<TaskProcessOutput>) -> Self {
            Self {
                outputs: std::sync::Mutex::new(outputs),
            }
        }
    }

    impl crate::commands::dashboard::handlers::TaskRunner for FakeRunner {
        fn run(&self, args: &[String]) -> TaskProcessOutput {
            let mut outputs = self.outputs.lock().unwrap();
            if outputs.is_empty() {
                return TaskProcessOutput {
                    command_line: args.join(" "),
                    exit_code: Some(0),
                    success: true,
                    stdout: "ok".to_string(),
                    stderr: String::new(),
                    duration_ms: 1,
                };
            }
            outputs.remove(0)
        }
    }

    fn ok_output(text: &str) -> TaskProcessOutput {
        TaskProcessOutput {
            command_line: text.to_string(),
            exit_code: Some(0),
            success: true,
            stdout: text.to_string(),
            stderr: String::new(),
            duration_ms: 1,
        }
    }

    fn fail_output(text: &str) -> TaskProcessOutput {
        TaskProcessOutput {
            command_line: text.to_string(),
            exit_code: Some(1),
            success: false,
            stdout: String::new(),
            stderr: text.to_string(),
            duration_ms: 1,
        }
    }

    fn test_router(runner: Arc<dyn crate::commands::dashboard::handlers::TaskRunner>) -> Router {
        let state = crate::commands::dashboard::DashboardState::for_tests(runner);
        Router::new()
            .route(
                "/api/workspaces/diagnostics/summary",
                get(workspace_diagnostics_summary),
            )
            .route(
                "/api/workspaces/guarded/preview",
                axum::routing::post(super::super::workspace_preview_guarded_task),
            )
            .route(
                "/api/workspaces/guarded/execute",
                axum::routing::post(super::super::workspace_execute_guarded_task),
            )
            .route(
                "/api/workspaces/run",
                axum::routing::post(super::super::workspace_run_task),
            )
            .with_state(state)
    }

    #[tokio::test]
    #[serial]
    async fn diagnostics_summary_collects_failed_tasks_guarded_receipts_and_audit() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test.json");
        let audit_path = dir.path().join("audit.jsonl");
        std::fs::write(
            &audit_path,
            concat!(
                "{\"timestamp\":1700000000,\"action\":\"dashboard.task.preview\",\"target\":\"D:/tmp/demo.txt\",\"user\":\"dashboard\",\"params\":{},\"result\":\"dry_run\",\"reason\":\"\"}\n",
                "{\"timestamp\":1700000001,\"action\":\"dashboard.task.execute.rm\",\"target\":\"D:/tmp/demo.txt\",\"user\":\"dashboard\",\"params\":{},\"result\":\"success\",\"reason\":\"\"}\n"
            ),
        )
        .unwrap();

        unsafe {
            std::env::set_var("XUN_DB", &db_path);
        }

        let runner = Arc::new(FakeRunner::new(vec![
            fail_output("run failed"),
            ok_output("preview ok"),
            ok_output("guarded execute"),
        ]));
        let app = test_router(runner);

        let run_req = serde_json::json!({
            "workspace": "statistics-diagnostics",
            "action": "recent",
            "target": ".",
            "args": ["recent"]
        });
        let run_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/workspaces/run")
                    .header("content-type", "application/json")
                    .body(Body::from(run_req.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(run_resp.status(), StatusCode::OK);

        let preview_req = serde_json::json!({
            "workspace": "files-security",
            "action": "rm",
            "target": "D:/tmp/demo.txt",
            "preview_args": ["rm", "--dry-run", "D:/tmp/demo.txt"],
            "execute_args": ["rm", "-y", "D:/tmp/demo.txt"],
            "preview_summary": "閸掔娀娅?D:/tmp/demo.txt"
        });
        let preview_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/workspaces/guarded/preview")
                    .header("content-type", "application/json")
                    .body(Body::from(preview_req.to_string()))
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
        let token = preview_json["token"].as_str().unwrap().to_string();

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

        let summary_resp = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/workspaces/diagnostics/summary?task_limit=5&audit_limit=100")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(summary_resp.status(), StatusCode::OK);
        let summary_json: serde_json::Value = serde_json::from_slice(
            &to_bytes(summary_resp.into_body(), usize::MAX)
                .await
                .unwrap(),
        )
        .unwrap();
        assert!(summary_json["doctor"].is_object());
        assert!(summary_json["doctor"]["issues"].is_array());
        assert_eq!(summary_json["overview"]["recent_failed_tasks"], 1);
        assert_eq!(summary_json["overview"]["recent_guarded_receipts"], 1);
        assert_eq!(summary_json["overview"]["recent_governance_alerts"], 1);
        assert!(summary_json["overview"]["audit_entries"].as_u64().unwrap() >= 2);
        assert_eq!(summary_json["failed_tasks"].as_array().unwrap().len(), 1);
        assert_eq!(
            summary_json["guarded_receipts"].as_array().unwrap().len(),
            1
        );
        assert_eq!(
            summary_json["governance_alerts"].as_array().unwrap().len(),
            1
        );
        assert_eq!(
            summary_json["governance_alerts"][0]["workspace"],
            "files-security"
        );
        let audit_timeline = summary_json["audit_timeline"].as_array().unwrap();
        assert!(audit_timeline.len() >= 2);
        assert!(
            audit_timeline
                .iter()
                .any(|entry| entry["target"] == "D:/tmp/demo.txt")
        );

        unsafe {
            std::env::remove_var("XUN_DB");
        }
    }
}
