mod handlers;
mod handlers_env;

use axum::Router;
use axum::http::{StatusCode, header};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{delete, get, post};
use rust_embed::Embed;

use crate::cli::ServeCmd;
use crate::env_core::EnvManager;
use crate::output::{CliError, CliResult};

#[cfg(feature = "diff")]
use crossbeam_channel::{Receiver, Sender};
#[cfg(feature = "diff")]
use notify::Watcher;
#[cfg(feature = "diff")]
use serde_json::json;
#[cfg(feature = "diff")]
use std::collections::HashSet;
#[cfg(feature = "diff")]
use std::path::{Path, PathBuf};

#[derive(Embed)]
#[folder = "dashboard-ui/dist/"]
struct Assets;

#[derive(Clone)]
pub(super) struct DashboardState {
    guarded_tasks: handlers::GuardedTaskService,
    recipes: handlers::RecipeService,
    #[cfg(feature = "diff")]
    event_tx: tokio::sync::broadcast::Sender<String>,
    #[cfg(feature = "diff")]
    watch_cmd_tx: Sender<PathBuf>,
}

impl DashboardState {
    fn new() -> Self {
        let guarded_tasks = handlers::GuardedTaskService::new();
        let recipes = handlers::RecipeService::new();
        #[cfg(feature = "diff")]
        {
            let (event_tx, _) = tokio::sync::broadcast::channel::<String>(256);
            let watch_cmd_tx = spawn_watch_thread(event_tx.clone());
            Self {
                guarded_tasks,
                recipes,
                event_tx,
                watch_cmd_tx,
            }
        }

        #[cfg(not(feature = "diff"))]
        {
            Self {
                guarded_tasks,
                recipes,
            }
        }
    }

    pub(in crate::commands::dashboard) fn guarded_tasks(&self) -> handlers::GuardedTaskService {
        self.guarded_tasks.clone()
    }

    pub(in crate::commands::dashboard) fn recipes(&self) -> handlers::RecipeService {
        self.recipes.clone()
    }

    #[cfg(test)]
    pub(in crate::commands::dashboard) fn for_tests(
        runner: std::sync::Arc<dyn handlers::TaskRunner>,
    ) -> Self {
        let guarded_tasks = handlers::GuardedTaskService::with_runner(runner);
        let recipes = handlers::RecipeService::for_tests();
        #[cfg(feature = "diff")]
        {
            let (event_tx, _) = tokio::sync::broadcast::channel::<String>(64);
            let watch_cmd_tx = spawn_watch_thread(event_tx.clone());
            Self {
                guarded_tasks,
                recipes,
                event_tx,
                watch_cmd_tx,
            }
        }

        #[cfg(not(feature = "diff"))]
        {
            Self {
                guarded_tasks,
                recipes,
            }
        }
    }

    #[cfg(feature = "diff")]
    pub(super) fn subscribe_events(&self) -> tokio::sync::broadcast::Receiver<String> {
        self.event_tx.subscribe()
    }

    #[cfg(feature = "diff")]
    pub(super) fn request_watch_path<P: AsRef<Path>>(&self, path: P) {
        if let Some(target) = normalize_watch_target(path.as_ref().to_path_buf()) {
            let _ = self.watch_cmd_tx.send(target);
        }
    }

    #[cfg(feature = "diff")]
    pub(super) fn emit_file_changed<P: AsRef<Path>>(&self, path: P) {
        emit_file_changed(&self.event_tx, path.as_ref());
    }
}

fn static_handler(path: &str) -> Response {
    let path = if path.is_empty() { "index.html" } else { path };
    match Assets::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            ([(header::CONTENT_TYPE, mime.as_ref())], content.data).into_response()
        }
        None => {
            // SPA fallback: serve index.html for non-API routes
            match Assets::get("index.html") {
                Some(content) => Html(
                    std::str::from_utf8(&content.data)
                        .unwrap_or_default()
                        .to_string(),
                )
                .into_response(),
                None => (StatusCode::NOT_FOUND, "not found").into_response(),
            }
        }
    }
}

fn base_router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    let r = Router::<S>::new()
        .route("/api/bookmarks", get(handlers::list_bookmarks))
        .route("/api/bookmarks/export", get(handlers::export_bookmarks))
        .route("/api/bookmarks/import", post(handlers::import_bookmarks))
        .route("/api/bookmarks/{name}", post(handlers::upsert_bookmark))
        .route("/api/bookmarks/{name}", delete(handlers::delete_bookmark))
        .route(
            "/api/bookmarks/{name}/rename",
            post(handlers::rename_bookmark),
        )
        .route("/api/bookmarks/batch", post(handlers::bookmarks_batch))
        .route("/api/ports", get(handlers::list_ports))
        .route("/api/ports/icon/{pid}", get(handlers::port_icon))
        .route("/api/ports/kill/{port}", post(handlers::kill_port))
        .route("/api/ports/kill-pid/{pid}", post(handlers::kill_pid))
        .route("/api/proxy/status", get(handlers::proxy_status))
        .route(
            "/api/proxy/config",
            get(handlers::get_proxy_config).post(handlers::set_proxy_config),
        )
        .route("/api/proxy/test", get(handlers::proxy_test))
        .route("/api/proxy/set", post(handlers::proxy_set))
        .route("/api/proxy/del", post(handlers::proxy_del))
        .route(
            "/api/config",
            get(handlers::get_config)
                .post(handlers::post_config_patch)
                .put(handlers::put_config_replace),
        )
        .route("/api/audit", get(handlers::get_audit));

    let r = r
        .route("/api/env/ping", get(handlers_env::env_ping))
        .route("/api/env/status", get(handlers_env::env_status))
        .route("/api/env/vars", get(handlers_env::list_vars))
        .route("/api/env/vars/{name}", get(handlers_env::get_var))
        .route("/api/env/vars/{name}", post(handlers_env::set_var))
        .route("/api/env/vars/{name}", delete(handlers_env::delete_var))
        .route(
            "/api/env/vars/{name}/history",
            get(handlers_env::var_history),
        )
        .route("/api/env/path/add", post(handlers_env::path_add))
        .route("/api/env/path/remove", post(handlers_env::path_remove))
        .route("/api/env/snapshots", get(handlers_env::list_snapshots))
        .route("/api/env/snapshots", post(handlers_env::create_snapshot))
        .route("/api/env/snapshots", delete(handlers_env::prune_snapshots))
        .route(
            "/api/env/snapshots/restore",
            post(handlers_env::restore_snapshot),
        )
        .route("/api/env/doctor/run", post(handlers_env::doctor_run))
        .route("/api/env/doctor/fix", post(handlers_env::doctor_fix))
        .route("/api/env/import", post(handlers_env::import_vars))
        .route("/api/env/export", get(handlers_env::export_vars))
        .route("/api/env/export-all", get(handlers_env::export_all))
        .route("/api/env/export-live", get(handlers_env::export_live))
        .route("/api/env/diff-live", get(handlers_env::diff_live))
        .route("/api/env/graph", get(handlers_env::dependency_graph))
        .route("/api/env/audit", get(handlers_env::audit_list))
        .route(
            "/api/env/template/expand",
            post(handlers_env::template_expand),
        )
        .route("/api/env/run", post(handlers_env::run_command))
        .route("/api/env/annotations", get(handlers_env::annotations_list))
        .route(
            "/api/env/annotations/{name}",
            get(handlers_env::annotation_get),
        )
        .route(
            "/api/env/annotations/{name}",
            post(handlers_env::annotation_set),
        )
        .route(
            "/api/env/annotations/{name}",
            delete(handlers_env::annotation_delete),
        )
        .route("/api/env/profiles", get(handlers_env::list_profiles))
        .route(
            "/api/env/profiles/{name}/capture",
            post(handlers_env::capture_profile),
        )
        .route(
            "/api/env/profiles/{name}/apply",
            post(handlers_env::apply_profile),
        )
        .route(
            "/api/env/profiles/{name}/diff",
            get(handlers_env::profile_diff),
        )
        .route(
            "/api/env/profiles/{name}",
            delete(handlers_env::delete_profile),
        )
        .route("/api/env/schema", get(handlers_env::schema_show))
        .route(
            "/api/env/schema/add-required",
            post(handlers_env::schema_add_required),
        )
        .route(
            "/api/env/schema/add-regex",
            post(handlers_env::schema_add_regex),
        )
        .route(
            "/api/env/schema/add-enum",
            post(handlers_env::schema_add_enum),
        )
        .route("/api/env/schema/remove", post(handlers_env::schema_remove))
        .route("/api/env/schema/reset", post(handlers_env::schema_reset))
        .route("/api/env/validate", post(handlers_env::validate))
        .route("/api/env/ws", get(handlers_env::env_ws));

    #[cfg(feature = "redirect")]
    let r = r
        .route(
            "/api/redirect/profiles",
            get(handlers::list_redirect_profiles),
        )
        .route(
            "/api/redirect/profiles/{name}",
            post(handlers::upsert_redirect_profile).delete(handlers::delete_redirect_profile),
        )
        .route("/api/redirect/dry-run", post(handlers::redirect_dry_run));

    r
}

fn with_static_fallback<S>(router: Router<S>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    router.fallback(|req: axum::extract::Request| async move {
        let path = req.uri().path().trim_start_matches('/');
        static_handler(path)
    })
}

fn build_router(state: DashboardState) -> Router {
    let r = base_router::<DashboardState>()
        .route(
            "/api/workspaces/capabilities",
            get(handlers::workspace_capabilities),
        )
        .route(
            "/api/workspaces/overview/summary",
            get(handlers::workspace_overview_summary),
        )
        .route(
            "/api/workspaces/diagnostics/summary",
            get(handlers::workspace_diagnostics_summary),
        )
        .route(
            "/api/workspaces/tasks/recent",
            get(handlers::workspace_recent_tasks),
        )
        .route(
            "/api/workspaces/recipes",
            get(handlers::list_workspace_recipes),
        )
        .route(
            "/api/workspaces/recipes",
            post(handlers::upsert_workspace_recipe),
        )
        .route(
            "/api/workspaces/recipes/preview",
            post(handlers::preview_workspace_recipe),
        )
        .route(
            "/api/workspaces/recipes/execute",
            post(handlers::execute_workspace_recipe),
        )
        .route("/api/workspaces/run", post(handlers::workspace_run_task))
        .route(
            "/api/workspaces/guarded/preview",
            post(handlers::workspace_preview_guarded_task),
        )
        .route(
            "/api/workspaces/guarded/execute",
            post(handlers::workspace_execute_guarded_task),
        );

    #[cfg(feature = "diff")]
    let r = r
        .route("/api/files", get(handlers::list_files))
        .route("/api/files/search", get(handlers::search_files))
        .route("/api/info", get(handlers::get_file_info))
        .route("/api/content", get(handlers::get_file_content))
        .route("/api/diff", post(handlers::diff_handler))
        .route("/api/convert", post(handlers::convert_file))
        .route("/api/validate", post(handlers::validate_file))
        .route("/ws", get(handlers::ws_handler));

    with_static_fallback(r).with_state(state)
}

pub(crate) fn cmd_serve(args: ServeCmd) -> CliResult {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| CliError::new(1, format!("Failed to create tokio runtime: {e}")))?;

    rt.block_on(async move {
        let app = {
            let state = DashboardState::new();
            build_router(state)
        };

        spawn_env_snapshot_scheduler();

        let addr = std::net::SocketAddr::from(([127, 0, 0, 1], args.port));
        ui_println!("Dashboard: http://localhost:{}", args.port);
        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .map_err(|e| CliError::new(1, format!("Failed to bind: {e}")))?;
        axum::serve(listener, app)
            .await
            .map_err(|e| CliError::new(1, format!("Server error: {e}")))?;
        Ok::<(), CliError>(())
    })?;
    Ok(())
}

fn spawn_env_snapshot_scheduler() {
    let interval_secs = EnvManager::new().config().snapshot_every_secs;
    if interval_secs == 0 {
        return;
    }
    ui_println!(
        "Env auto snapshot scheduler enabled: every {}s (key: snapshot_every_secs)",
        interval_secs
    );
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(std::time::Duration::from_secs(interval_secs));
        // Skip the immediate first tick; only snapshot after one full interval.
        ticker.tick().await;
        loop {
            ticker.tick().await;
            let outcome = tokio::task::spawn_blocking(|| {
                let manager = EnvManager::new();
                manager.snapshot_create(Some("auto-snapshot"))
            })
            .await;
            match outcome {
                Ok(Ok(meta)) => ui_println!("Env auto snapshot created: {}", meta.id),
                Ok(Err(err)) => eprintln!("env auto snapshot failed: {}", err),
                Err(join_err) => eprintln!("env auto snapshot task join failed: {}", join_err),
            }
        }
    });
}

#[cfg(feature = "diff")]
fn spawn_watch_thread(event_tx: tokio::sync::broadcast::Sender<String>) -> Sender<PathBuf> {
    let (cmd_tx, cmd_rx) = crossbeam_channel::unbounded::<PathBuf>();

    let _ = std::thread::Builder::new()
        .name("xun-dashboard-fs-watch".to_string())
        .spawn(move || run_watch_loop(event_tx, cmd_rx));

    cmd_tx
}

#[cfg(feature = "diff")]
fn run_watch_loop(event_tx: tokio::sync::broadcast::Sender<String>, cmd_rx: Receiver<PathBuf>) {
    let (raw_event_tx, raw_event_rx) =
        crossbeam_channel::unbounded::<notify::Result<notify::Event>>();
    let mut watcher = match notify::recommended_watcher(move |res| {
        let _ = raw_event_tx.send(res);
    }) {
        Ok(v) => v,
        Err(_) => return,
    };

    let mut watched_dirs: HashSet<PathBuf> = HashSet::new();

    loop {
        crossbeam_channel::select! {
            recv(cmd_rx) -> cmd => {
                match cmd {
                    Ok(path) => {
                        if !watched_dirs.insert(path.clone()) {
                            continue;
                        }
                        if watcher.watch(&path, notify::RecursiveMode::Recursive).is_err() {
                            watched_dirs.remove(&path);
                        }
                    }
                    Err(_) => break,
                }
            }
            recv(raw_event_rx) -> evt => {
                match evt {
                    Ok(Ok(event)) => {
                        if !matches!(
                            event.kind,
                            notify::EventKind::Any
                                | notify::EventKind::Create(_)
                                | notify::EventKind::Modify(_)
                                | notify::EventKind::Remove(_)
                        ) {
                            continue;
                        }
                        if event.paths.is_empty() {
                            emit_refresh(&event_tx);
                            continue;
                        }
                        for path in event.paths {
                            emit_file_changed(&event_tx, &path);
                        }
                    }
                    Ok(Err(_)) => {
                        emit_refresh(&event_tx);
                    }
                    Err(_) => break,
                }
            }
        }
    }
}

#[cfg(feature = "diff")]
fn normalize_watch_target(path: PathBuf) -> Option<PathBuf> {
    let resolved = std::fs::canonicalize(&path).unwrap_or(path);
    if resolved.is_dir() {
        Some(resolved)
    } else if resolved.is_file() {
        resolved.parent().map(|p| p.to_path_buf())
    } else {
        None
    }
}

#[cfg(feature = "diff")]
fn emit_refresh(event_tx: &tokio::sync::broadcast::Sender<String>) {
    let _ = event_tx.send(r#"{"type":"refresh"}"#.to_string());
}

#[cfg(feature = "diff")]
fn emit_file_changed(event_tx: &tokio::sync::broadcast::Sender<String>, path: &Path) {
    let payload = json!({
        "type": "file_changed",
        "path": path.to_string_lossy().to_string(),
    });
    let _ = event_tx.send(payload.to_string());
}
