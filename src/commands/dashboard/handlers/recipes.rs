use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use super::{
    GuardedTaskExecuteRequest, GuardedTaskPreviewRequest, GuardedTaskService, TaskProcessOutput,
    WorkspaceTaskRunRequest,
};

const RECIPE_PREVIEW_TTL_SECS: u64 = 300;
const MAX_RECIPE_STEPS: usize = 12;

#[derive(Clone)]
pub(in crate::commands::dashboard) struct RecipeService {
    inner: Arc<RecipeServiceInner>,
}

struct RecipeServiceInner {
    store_path: PathBuf,
    custom: Mutex<BTreeMap<String, RecipeDefinition>>,
    pending: Mutex<HashMap<String, PendingRecipePreview>>,
    seq: AtomicU64,
    ttl: Duration,
}

#[derive(Clone, Debug)]
struct PendingRecipePreview {
    recipe_id: String,
    recipe_name: String,
    steps: Vec<ResolvedRecipeStep>,
    guarded_tokens: BTreeMap<String, String>,
    expires_at: Instant,
}

#[derive(Clone, Debug)]
enum ResolvedRecipeStep {
    Run {
        id: String,
        title: String,
        workspace: String,
        action: String,
        target: String,
        summary: String,
        dry_run_request: WorkspaceTaskRunRequest,
        execute_request: WorkspaceTaskRunRequest,
    },
    Guarded {
        id: String,
        title: String,
        workspace: String,
        action: String,
        target: String,
        summary: String,
        preview_request: GuardedTaskPreviewRequest,
    },
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(in crate::commands::dashboard) enum RecipeSource {
    Builtin,
    Custom,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub(in crate::commands::dashboard) struct RecipeParamDefinition {
    key: String,
    label: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    default_value: String,
    #[serde(default)]
    required: bool,
    #[serde(default)]
    placeholder: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(in crate::commands::dashboard) enum RecipeStepDefinition {
    Run {
        id: String,
        title: String,
        workspace: String,
        action: String,
        target: String,
        summary: String,
        run_args: Vec<String>,
        #[serde(default)]
        dry_run_args: Vec<String>,
    },
    Guarded {
        id: String,
        title: String,
        workspace: String,
        action: String,
        target: String,
        summary: String,
        preview_args: Vec<String>,
        execute_args: Vec<String>,
        #[serde(default)]
        preview_summary: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub(in crate::commands::dashboard) struct RecipeDefinition {
    id: String,
    name: String,
    description: String,
    category: String,
    source: RecipeSource,
    supports_dry_run: bool,
    #[serde(default)]
    params: Vec<RecipeParamDefinition>,
    steps: Vec<RecipeStepDefinition>,
}

#[derive(Serialize, Clone, Debug)]
pub(in crate::commands::dashboard) struct RecipeListResponse {
    recipes: Vec<RecipeDefinition>,
}

#[derive(Deserialize, Clone, Debug)]
pub(in crate::commands::dashboard) struct RecipeUpsertRequest {
    recipe: RecipeDefinition,
}

#[derive(Deserialize, Clone, Debug, Default)]
pub(in crate::commands::dashboard) struct RecipePreviewRequest {
    recipe_id: String,
    #[serde(default)]
    values: BTreeMap<String, String>,
}

#[derive(Serialize, Clone, Debug)]
pub(in crate::commands::dashboard) struct RecipePreviewStepResult {
    id: String,
    title: String,
    workspace: String,
    action: String,
    target: String,
    status: String,
    guarded: bool,
    dry_run: bool,
    summary: String,
    process: TaskProcessOutput,
}

#[derive(Serialize, Clone, Debug)]
pub(in crate::commands::dashboard) struct RecipePreviewResponse {
    token: String,
    recipe_id: String,
    recipe_name: String,
    status: String,
    guarded: bool,
    dry_run: bool,
    ready_to_execute: bool,
    summary: String,
    total_steps: usize,
    expires_in_secs: u64,
    steps: Vec<RecipePreviewStepResult>,
}

#[derive(Deserialize, Clone, Debug)]
pub(in crate::commands::dashboard) struct RecipeExecuteRequest {
    token: String,
    #[serde(default)]
    confirm: bool,
}

#[derive(Serialize, Clone, Debug)]
pub(in crate::commands::dashboard) struct RecipeExecutionStepReceipt {
    id: String,
    title: String,
    workspace: String,
    action: String,
    target: String,
    status: String,
    guarded: bool,
    dry_run: bool,
    summary: String,
    audit_action: Option<String>,
    process: TaskProcessOutput,
}

#[derive(Serialize, Clone, Debug)]
pub(in crate::commands::dashboard) struct RecipeExecutionReceipt {
    token: String,
    recipe_id: String,
    recipe_name: String,
    status: String,
    guarded: bool,
    dry_run: bool,
    summary: String,
    total_steps: usize,
    completed_steps: usize,
    failed_step_id: Option<String>,
    audited_at: u64,
    steps: Vec<RecipeExecutionStepReceipt>,
}

impl RecipeService {
    pub(in crate::commands::dashboard) fn new() -> Self {
        Self::with_store_path(recipe_store_path())
    }

    #[cfg(test)]
    pub(in crate::commands::dashboard) fn for_tests() -> Self {
        static TEST_SEQ: AtomicU64 = AtomicU64::new(1);
        let seq = TEST_SEQ.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "xun-dashboard-recipes-{}-{}.json",
            std::process::id(),
            seq
        ));
        let _ = fs::remove_file(&path);
        Self::with_store_path(path)
    }

    pub(in crate::commands::dashboard) fn with_store_path(path: PathBuf) -> Self {
        Self {
            inner: Arc::new(RecipeServiceInner {
                custom: Mutex::new(load_custom_recipes(&path)),
                store_path: path,
                pending: Mutex::new(HashMap::new()),
                seq: AtomicU64::new(1),
                ttl: Duration::from_secs(RECIPE_PREVIEW_TTL_SECS),
            }),
        }
    }

    pub(in crate::commands::dashboard) fn list_recipes(&self) -> RecipeListResponse {
        let custom = self.inner.custom.lock().unwrap_or_else(|e| e.into_inner());
        let mut recipes = BTreeMap::new();
        for recipe in builtin_recipes() {
            recipes.insert(recipe.id.clone(), recipe);
        }
        for (id, recipe) in custom.iter() {
            recipes.insert(id.clone(), recipe.clone());
        }
        let mut entries = recipes.into_values().collect::<Vec<_>>();
        entries.sort_by(|left, right| {
            left.category
                .cmp(&right.category)
                .then(left.name.cmp(&right.name))
        });
        RecipeListResponse { recipes: entries }
    }

    pub(in crate::commands::dashboard) fn upsert_recipe(
        &self,
        mut recipe: RecipeDefinition,
    ) -> Result<RecipeDefinition, (StatusCode, String)> {
        recipe.source = RecipeSource::Custom;
        validate_recipe_definition(&recipe)?;
        {
            let mut custom = self.inner.custom.lock().unwrap_or_else(|e| e.into_inner());
            custom.insert(recipe.id.clone(), recipe.clone());
            save_custom_recipes(&self.inner.store_path, &custom)?;
        }
        Ok(recipe)
    }

    pub(in crate::commands::dashboard) fn preview_recipe(
        &self,
        req: RecipePreviewRequest,
        guarded_tasks: GuardedTaskService,
    ) -> Result<RecipePreviewResponse, (StatusCode, String)> {
        let recipe = self.find_recipe(&req.recipe_id)?;
        if !recipe.supports_dry_run {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("recipe {} does not support dry-run", recipe.id),
            ));
        }
        let values = resolve_recipe_values(&recipe, &req.values)?;
        let mut steps = Vec::with_capacity(recipe.steps.len());
        let mut preview_results = Vec::with_capacity(recipe.steps.len());
        let mut guarded_tokens = BTreeMap::new();

        for step in &recipe.steps {
            let resolved = resolve_recipe_step(step, &values)?;
            match &resolved {
                ResolvedRecipeStep::Run {
                    id,
                    title,
                    workspace,
                    action,
                    target,
                    summary,
                    dry_run_request,
                    ..
                } => {
                    let preview = guarded_tasks
                        .preview_run(dry_run_request.clone(), Some(summary.clone()))?;
                    preview_results.push(RecipePreviewStepResult {
                        id: id.clone(),
                        title: title.clone(),
                        workspace: workspace.clone(),
                        action: action.clone(),
                        target: target.clone(),
                        status: "previewed".to_string(),
                        guarded: false,
                        dry_run: true,
                        summary: summary.clone(),
                        process: preview.process,
                    });
                }
                ResolvedRecipeStep::Guarded {
                    id,
                    title,
                    workspace,
                    action,
                    target,
                    summary,
                    preview_request,
                } => {
                    let preview = guarded_tasks.preview(preview_request.clone())?;
                    guarded_tokens.insert(id.clone(), preview.token.clone());
                    preview_results.push(RecipePreviewStepResult {
                        id: id.clone(),
                        title: title.clone(),
                        workspace: workspace.clone(),
                        action: action.clone(),
                        target: target.clone(),
                        status: preview.status,
                        guarded: true,
                        dry_run: true,
                        summary: summary.clone(),
                        process: preview.process,
                    });
                }
            }
            steps.push(resolved);
        }

        self.evict_expired();
        let token = self.next_token();
        {
            let mut pending = self.inner.pending.lock().unwrap_or_else(|e| e.into_inner());
            pending.insert(
                token.clone(),
                PendingRecipePreview {
                    recipe_id: recipe.id.clone(),
                    recipe_name: recipe.name.clone(),
                    steps,
                    guarded_tokens,
                    expires_at: Instant::now() + self.inner.ttl,
                },
            );
        }

        Ok(RecipePreviewResponse {
            token,
            recipe_id: recipe.id,
            recipe_name: recipe.name,
            status: "previewed".to_string(),
            guarded: preview_results.iter().any(|step| step.guarded),
            dry_run: true,
            ready_to_execute: true,
            summary: format!("已预演，共 {} 步，可确认执行。", preview_results.len()),
            total_steps: preview_results.len(),
            expires_in_secs: self.inner.ttl.as_secs(),
            steps: preview_results,
        })
    }

    pub(in crate::commands::dashboard) fn execute_recipe(
        &self,
        req: RecipeExecuteRequest,
        guarded_tasks: GuardedTaskService,
    ) -> Result<RecipeExecutionReceipt, (StatusCode, String)> {
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
                "recipe preview token not found or expired".to_string(),
            ));
        };

        let total_steps = pending.steps.len();
        let guarded = pending.steps.iter().any(ResolvedRecipeStep::is_guarded);
        let mut completed_steps = 0usize;
        let mut failed_step_id = None;
        let mut receipts = Vec::with_capacity(total_steps);

        for step in pending.steps {
            match step {
                ResolvedRecipeStep::Run {
                    id,
                    title,
                    summary,
                    execute_request,
                    ..
                } => {
                    let response = guarded_tasks.run(execute_request)?;
                    let succeeded = response.process.success;
                    receipts.push(RecipeExecutionStepReceipt {
                        id: id.clone(),
                        title,
                        workspace: response.workspace,
                        action: response.action,
                        target: response.target,
                        status: if succeeded {
                            "succeeded".to_string()
                        } else {
                            "failed".to_string()
                        },
                        guarded: false,
                        dry_run: false,
                        summary,
                        audit_action: None,
                        process: response.process,
                    });
                    if succeeded {
                        completed_steps += 1;
                    } else {
                        failed_step_id = Some(id);
                        break;
                    }
                }
                ResolvedRecipeStep::Guarded {
                    id, title, summary, ..
                } => {
                    let Some(step_token) = pending.guarded_tokens.get(&id).cloned() else {
                        return Err((
                            StatusCode::BAD_REQUEST,
                            format!("missing guarded preview token for step {id}"),
                        ));
                    };
                    let receipt = guarded_tasks.execute(GuardedTaskExecuteRequest {
                        token: step_token,
                        confirm: true,
                    })?;
                    let succeeded = receipt.process.success;
                    receipts.push(RecipeExecutionStepReceipt {
                        id: id.clone(),
                        title,
                        workspace: receipt.workspace,
                        action: receipt.action,
                        target: receipt.target,
                        status: receipt.status,
                        guarded: true,
                        dry_run: false,
                        summary,
                        audit_action: Some(receipt.audit_action),
                        process: receipt.process,
                    });
                    if succeeded {
                        completed_steps += 1;
                    } else {
                        failed_step_id = Some(id);
                        break;
                    }
                }
            }
        }

        let status = if failed_step_id.is_some() {
            "failed"
        } else {
            "succeeded"
        };

        Ok(RecipeExecutionReceipt {
            token: req.token,
            recipe_id: pending.recipe_id,
            recipe_name: pending.recipe_name.clone(),
            status: status.to_string(),
            guarded,
            dry_run: false,
            summary: if status == "succeeded" {
                format!("{} 执行完成。", pending.recipe_name)
            } else {
                format!("{} 在中途失败，已停止后续步骤。", pending.recipe_name)
            },
            total_steps,
            completed_steps,
            failed_step_id,
            audited_at: now_unix_secs_local(),
            steps: receipts,
        })
    }

    fn find_recipe(&self, recipe_id: &str) -> Result<RecipeDefinition, (StatusCode, String)> {
        if recipe_id.trim().is_empty() {
            return Err((StatusCode::BAD_REQUEST, "recipe_id is empty".to_string()));
        }
        let custom = self.inner.custom.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(recipe) = custom.get(recipe_id) {
            return Ok(recipe.clone());
        }
        builtin_recipes()
            .into_iter()
            .find(|recipe| recipe.id == recipe_id)
            .ok_or_else(|| {
                (
                    StatusCode::NOT_FOUND,
                    format!("recipe {recipe_id} not found"),
                )
            })
    }

    fn evict_expired(&self) {
        let now = Instant::now();
        let mut pending = self.inner.pending.lock().unwrap_or_else(|e| e.into_inner());
        pending.retain(|_, preview| preview.expires_at > now);
    }

    fn next_token(&self) -> String {
        let seq = self.inner.seq.fetch_add(1, Ordering::Relaxed);
        format!("recipe-{}-{}", now_unix_secs_local(), seq)
    }
}

impl ResolvedRecipeStep {
    fn is_guarded(&self) -> bool {
        matches!(self, Self::Guarded { .. })
    }
}

pub(in crate::commands::dashboard) async fn list_workspace_recipes(
    State(state): State<crate::commands::dashboard::DashboardState>,
) -> Json<RecipeListResponse> {
    Json(state.recipes().list_recipes())
}

pub(in crate::commands::dashboard) async fn upsert_workspace_recipe(
    State(state): State<crate::commands::dashboard::DashboardState>,
    Json(body): Json<RecipeUpsertRequest>,
) -> Result<Json<RecipeDefinition>, (StatusCode, String)> {
    state.recipes().upsert_recipe(body.recipe).map(Json)
}

pub(in crate::commands::dashboard) async fn preview_workspace_recipe(
    State(state): State<crate::commands::dashboard::DashboardState>,
    Json(body): Json<RecipePreviewRequest>,
) -> Result<Json<RecipePreviewResponse>, (StatusCode, String)> {
    state
        .recipes()
        .preview_recipe(body, state.guarded_tasks())
        .map(Json)
}

pub(in crate::commands::dashboard) async fn execute_workspace_recipe(
    State(state): State<crate::commands::dashboard::DashboardState>,
    Json(body): Json<RecipeExecuteRequest>,
) -> Result<Json<RecipeExecutionReceipt>, (StatusCode, String)> {
    state
        .recipes()
        .execute_recipe(body, state.guarded_tasks())
        .map(Json)
}

fn recipe_store_path() -> PathBuf {
    let mut path = crate::store::db_path();
    path.set_file_name("recipes.json");
    path
}

fn load_custom_recipes(path: &Path) -> BTreeMap<String, RecipeDefinition> {
    fs::read_to_string(path)
        .ok()
        .and_then(|content| serde_json::from_str::<Vec<RecipeDefinition>>(&content).ok())
        .unwrap_or_default()
        .into_iter()
        .map(|recipe| (recipe.id.clone(), recipe))
        .collect()
}

fn save_custom_recipes(
    path: &Path,
    recipes: &BTreeMap<String, RecipeDefinition>,
) -> Result<(), (StatusCode, String)> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(io_error)?;
    }
    let tmp = path.with_extension("tmp");
    let payload = recipes.values().cloned().collect::<Vec<_>>();
    let json = serde_json::to_string_pretty(&payload).map_err(|err| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("serialize recipes failed: {err}"),
        )
    })?;
    fs::write(&tmp, json).map_err(io_error)?;
    fs::rename(&tmp, path).map_err(io_error)?;
    Ok(())
}

fn io_error(err: std::io::Error) -> (StatusCode, String) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        format!("recipe store I/O failed: {err}"),
    )
}

fn now_unix_secs_local() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn validate_recipe_definition(recipe: &RecipeDefinition) -> Result<(), (StatusCode, String)> {
    if recipe.id.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "recipe.id is empty".to_string()));
    }
    if recipe.name.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "recipe.name is empty".to_string()));
    }
    if recipe.steps.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "recipe.steps is empty".to_string()));
    }
    if recipe.steps.len() > MAX_RECIPE_STEPS {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("recipe.steps exceeds max limit {MAX_RECIPE_STEPS}"),
        ));
    }

    let mut param_keys = BTreeMap::new();
    for param in &recipe.params {
        if param.key.trim().is_empty() {
            return Err((
                StatusCode::BAD_REQUEST,
                "recipe param key is empty".to_string(),
            ));
        }
        if param.label.trim().is_empty() {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("recipe param {} label is empty", param.key),
            ));
        }
        if param_keys.insert(param.key.clone(), true).is_some() {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("duplicate recipe param key {}", param.key),
            ));
        }
    }

    let mut step_ids = BTreeMap::new();
    for step in &recipe.steps {
        match step {
            RecipeStepDefinition::Run {
                id,
                title,
                workspace,
                action,
                run_args,
                dry_run_args,
                ..
            } => {
                validate_step_identity(id, title, workspace, action, &mut step_ids)?;
                if run_args.is_empty() {
                    return Err((
                        StatusCode::BAD_REQUEST,
                        format!("run step {id} has empty run_args"),
                    ));
                }
                if recipe.supports_dry_run && dry_run_args.is_empty() {
                    return Err((
                        StatusCode::BAD_REQUEST,
                        format!("run step {id} is missing dry_run_args"),
                    ));
                }
            }
            RecipeStepDefinition::Guarded {
                id,
                title,
                workspace,
                action,
                preview_args,
                execute_args,
                ..
            } => {
                validate_step_identity(id, title, workspace, action, &mut step_ids)?;
                if preview_args.is_empty() {
                    return Err((
                        StatusCode::BAD_REQUEST,
                        format!("guarded step {id} has empty preview_args"),
                    ));
                }
                if execute_args.is_empty() {
                    return Err((
                        StatusCode::BAD_REQUEST,
                        format!("guarded step {id} has empty execute_args"),
                    ));
                }
                if preview_args == execute_args {
                    return Err((
                        StatusCode::BAD_REQUEST,
                        format!("guarded step {id} preview_args must differ from execute_args"),
                    ));
                }
            }
        }
    }

    Ok(())
}

fn validate_step_identity(
    id: &str,
    title: &str,
    workspace: &str,
    action: &str,
    step_ids: &mut BTreeMap<String, bool>,
) -> Result<(), (StatusCode, String)> {
    if id.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "recipe step id is empty".to_string(),
        ));
    }
    if title.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("recipe step {id} title is empty"),
        ));
    }
    if workspace.trim().is_empty() || action.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("recipe step {id} workspace/action is empty"),
        ));
    }
    if step_ids.insert(id.to_string(), true).is_some() {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("duplicate recipe step id {id}"),
        ));
    }
    Ok(())
}

fn resolve_recipe_values(
    recipe: &RecipeDefinition,
    input: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, String>, (StatusCode, String)> {
    let mut values = BTreeMap::new();
    for param in &recipe.params {
        let value = input
            .get(&param.key)
            .cloned()
            .unwrap_or_else(|| param.default_value.clone())
            .trim()
            .to_string();
        if param.required && value.is_empty() {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("recipe param {} is required", param.key),
            ));
        }
        values.insert(param.key.clone(), value);
    }
    Ok(values)
}

fn resolve_recipe_step(
    step: &RecipeStepDefinition,
    values: &BTreeMap<String, String>,
) -> Result<ResolvedRecipeStep, (StatusCode, String)> {
    match step {
        RecipeStepDefinition::Run {
            id,
            title,
            workspace,
            action,
            target,
            summary,
            run_args,
            dry_run_args,
        } => {
            let rendered_target = render_template(target, values)?;
            let rendered_summary = render_template(summary, values)?;
            Ok(ResolvedRecipeStep::Run {
                id: id.clone(),
                title: title.clone(),
                workspace: workspace.clone(),
                action: action.clone(),
                target: rendered_target.clone(),
                summary: rendered_summary,
                dry_run_request: WorkspaceTaskRunRequest {
                    workspace: workspace.clone(),
                    action: action.clone(),
                    target: rendered_target.clone(),
                    args: render_args(dry_run_args, values)?,
                },
                execute_request: WorkspaceTaskRunRequest {
                    workspace: workspace.clone(),
                    action: action.clone(),
                    target: rendered_target,
                    args: render_args(run_args, values)?,
                },
            })
        }
        RecipeStepDefinition::Guarded {
            id,
            title,
            workspace,
            action,
            target,
            summary,
            preview_args,
            execute_args,
            preview_summary,
        } => {
            let rendered_target = render_template(target, values)?;
            let rendered_summary = render_template(summary, values)?;
            Ok(ResolvedRecipeStep::Guarded {
                id: id.clone(),
                title: title.clone(),
                workspace: workspace.clone(),
                action: action.clone(),
                target: rendered_target.clone(),
                summary: rendered_summary,
                preview_request: GuardedTaskPreviewRequest {
                    workspace: workspace.clone(),
                    action: action.clone(),
                    target: rendered_target,
                    preview_args: render_args(preview_args, values)?,
                    execute_args: render_args(execute_args, values)?,
                    preview_summary: render_template(
                        if preview_summary.trim().is_empty() {
                            summary
                        } else {
                            preview_summary
                        },
                        values,
                    )?,
                },
            })
        }
    }
}

fn render_args(
    args: &[String],
    values: &BTreeMap<String, String>,
) -> Result<Vec<String>, (StatusCode, String)> {
    args.iter()
        .map(|arg| render_template(arg, values))
        .collect()
}

fn render_template(
    template: &str,
    values: &BTreeMap<String, String>,
) -> Result<String, (StatusCode, String)> {
    let mut rendered = template.to_string();
    for (key, value) in values {
        rendered = rendered.replace(&format!("{{{{{key}}}}}"), value);
    }
    if rendered.contains("{{") {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("unresolved recipe placeholder in {template}"),
        ));
    }
    Ok(rendered)
}

fn builtin_recipes() -> Vec<RecipeDefinition> {
    vec![
        RecipeDefinition {
            id: "file-cleanup-target".to_string(),
            name: "单目标清理".to_string(),
            description: "先看目录摘要，再对单个目标执行受保护删除。".to_string(),
            category: "files-security".to_string(),
            source: RecipeSource::Builtin,
            supports_dry_run: true,
            params: vec![
                RecipeParamDefinition {
                    key: "scan_root".to_string(),
                    label: "扫描目录".to_string(),
                    description: "用于 tree 预览的目录。".to_string(),
                    default_value: ".".to_string(),
                    required: true,
                    placeholder: ".".to_string(),
                },
                RecipeParamDefinition {
                    key: "target".to_string(),
                    label: "删除目标".to_string(),
                    description: "要删除的文件或目录。".to_string(),
                    default_value: String::new(),
                    required: true,
                    placeholder: "D:/tmp/demo.log".to_string(),
                },
            ],
            steps: vec![
                RecipeStepDefinition::Run {
                    id: "tree-preview".to_string(),
                    title: "目录摘要".to_string(),
                    workspace: "files-security".to_string(),
                    action: "tree".to_string(),
                    target: "{{scan_root}}".to_string(),
                    summary: "查看 {{scan_root}} 目录摘要".to_string(),
                    run_args: vec!["tree", "{{scan_root}}", "--stats-only"]
                        .into_iter()
                        .map(str::to_string)
                        .collect(),
                    dry_run_args: vec!["tree", "{{scan_root}}", "--stats-only"]
                        .into_iter()
                        .map(str::to_string)
                        .collect(),
                },
                RecipeStepDefinition::Guarded {
                    id: "guarded-rm".to_string(),
                    title: "受保护删除".to_string(),
                    workspace: "files-security".to_string(),
                    action: "rm".to_string(),
                    target: "{{target}}".to_string(),
                    summary: "删除 {{target}}".to_string(),
                    preview_args: vec!["rm", "--dry-run", "-f", "json", "{{target}}"]
                        .into_iter()
                        .map(str::to_string)
                        .collect(),
                    execute_args: vec!["rm", "-y", "-f", "json", "{{target}}"]
                        .into_iter()
                        .map(str::to_string)
                        .collect(),
                    preview_summary: "删除 {{target}}".to_string(),
                },
            ],
        },
        RecipeDefinition {
            id: "env-governance-user".to_string(),
            name: "环境治理闭环".to_string(),
            description: "先 doctor 预演，再确认修复，最后查看最近审计。".to_string(),
            category: "environment-config".to_string(),
            source: RecipeSource::Builtin,
            supports_dry_run: true,
            params: vec![RecipeParamDefinition {
                key: "scope".to_string(),
                label: "环境范围".to_string(),
                description: "user / system / all".to_string(),
                default_value: "user".to_string(),
                required: true,
                placeholder: "user".to_string(),
            }],
            steps: vec![
                RecipeStepDefinition::Guarded {
                    id: "env-doctor-fix".to_string(),
                    title: "Doctor 修复".to_string(),
                    workspace: "environment-config".to_string(),
                    action: "env:doctor:fix".to_string(),
                    target: "{{scope}}".to_string(),
                    summary: "修复 {{scope}} 环境问题".to_string(),
                    preview_args: vec!["env", "doctor", "--scope", "{{scope}}", "--format", "json"]
                        .into_iter()
                        .map(str::to_string)
                        .collect(),
                    execute_args: vec![
                        "env",
                        "doctor",
                        "--scope",
                        "{{scope}}",
                        "--fix",
                        "--format",
                        "json",
                    ]
                    .into_iter()
                    .map(str::to_string)
                    .collect(),
                    preview_summary: "修复 {{scope}} 环境问题".to_string(),
                },
                RecipeStepDefinition::Run {
                    id: "env-audit".to_string(),
                    title: "查看环境审计".to_string(),
                    workspace: "environment-config".to_string(),
                    action: "env:audit".to_string(),
                    target: "{{scope}}".to_string(),
                    summary: "查看 {{scope}} 环境审计".to_string(),
                    run_args: vec!["env", "audit", "--limit", "20", "--format", "json"]
                        .into_iter()
                        .map(str::to_string)
                        .collect(),
                    dry_run_args: vec!["env", "audit", "--limit", "20", "--format", "json"]
                        .into_iter()
                        .map(str::to_string)
                        .collect(),
                },
            ],
        },
        RecipeDefinition {
            id: "paths-context-health".to_string(),
            name: "路径健康巡检".to_string(),
            description: "串联上下文、最近访问与书签健康检查，快速完成路径工作台体检。".to_string(),
            category: "paths-context".to_string(),
            source: RecipeSource::Builtin,
            supports_dry_run: true,
            params: vec![
                RecipeParamDefinition {
                    key: "limit".to_string(),
                    label: "最近数量".to_string(),
                    description: "recent 步骤返回的记录数量。".to_string(),
                    default_value: "10".to_string(),
                    required: true,
                    placeholder: "10".to_string(),
                },
                RecipeParamDefinition {
                    key: "days".to_string(),
                    label: "陈旧阈值(天)".to_string(),
                    description: "check 步骤使用的陈旧阈值。".to_string(),
                    default_value: "90".to_string(),
                    required: true,
                    placeholder: "90".to_string(),
                },
            ],
            steps: vec![
                RecipeStepDefinition::Run {
                    id: "ctx-list".to_string(),
                    title: "列出上下文".to_string(),
                    workspace: "paths-context".to_string(),
                    action: "ctx:list".to_string(),
                    target: String::new(),
                    summary: "查看当前可用的上下文配置".to_string(),
                    run_args: vec!["ctx", "list", "-f", "json"]
                        .into_iter()
                        .map(str::to_string)
                        .collect(),
                    dry_run_args: vec!["ctx", "list", "-f", "json"]
                        .into_iter()
                        .map(str::to_string)
                        .collect(),
                },
                RecipeStepDefinition::Run {
                    id: "bookmark-recent".to_string(),
                    title: "查看最近访问".to_string(),
                    workspace: "paths-context".to_string(),
                    action: "recent".to_string(),
                    target: "{{limit}}".to_string(),
                    summary: "查看最近 {{limit}} 条访问记录".to_string(),
                    run_args: vec!["recent", "-n", "{{limit}}", "-f", "json"]
                        .into_iter()
                        .map(str::to_string)
                        .collect(),
                    dry_run_args: vec!["recent", "-n", "{{limit}}", "-f", "json"]
                        .into_iter()
                        .map(str::to_string)
                        .collect(),
                },
                RecipeStepDefinition::Run {
                    id: "bookmark-check".to_string(),
                    title: "执行健康检查".to_string(),
                    workspace: "paths-context".to_string(),
                    action: "check".to_string(),
                    target: "{{days}}".to_string(),
                    summary: "按 {{days}} 天阈值检查缺失路径和陈旧记录".to_string(),
                    run_args: vec!["check", "-d", "{{days}}", "-f", "json"]
                        .into_iter()
                        .map(str::to_string)
                        .collect(),
                    dry_run_args: vec!["check", "-d", "{{days}}", "-f", "json"]
                        .into_iter()
                        .map(str::to_string)
                        .collect(),
                },
            ],
        },
        RecipeDefinition {
            id: "integration-shell-bootstrap".to_string(),
            name: "Shell 安装引导".to_string(),
            description: "依次生成 init 与 completion 输出，用于初始化目标 shell 的集成与补全。"
                .to_string(),
            category: "integration-automation".to_string(),
            source: RecipeSource::Builtin,
            supports_dry_run: true,
            params: vec![RecipeParamDefinition {
                key: "shell".to_string(),
                label: "Shell".to_string(),
                description: "init 与 completion 使用的 shell 类型。".to_string(),
                default_value: "powershell".to_string(),
                required: true,
                placeholder: "powershell".to_string(),
            }],
            steps: vec![
                RecipeStepDefinition::Run {
                    id: "shell-init".to_string(),
                    title: "生成 init 脚本".to_string(),
                    workspace: "integration-automation".to_string(),
                    action: "init".to_string(),
                    target: "{{shell}}".to_string(),
                    summary: "生成 {{shell}} init 输出".to_string(),
                    run_args: vec!["init", "{{shell}}"]
                        .into_iter()
                        .map(str::to_string)
                        .collect(),
                    dry_run_args: vec!["init", "{{shell}}"]
                        .into_iter()
                        .map(str::to_string)
                        .collect(),
                },
                RecipeStepDefinition::Run {
                    id: "shell-completion".to_string(),
                    title: "生成 completion 脚本".to_string(),
                    workspace: "integration-automation".to_string(),
                    action: "completion".to_string(),
                    target: "{{shell}}".to_string(),
                    summary: "生成 {{shell}} completion 输出".to_string(),
                    run_args: vec!["completion", "{{shell}}"]
                        .into_iter()
                        .map(str::to_string)
                        .collect(),
                    dry_run_args: vec!["completion", "{{shell}}"]
                        .into_iter()
                        .map(str::to_string)
                        .collect(),
                },
            ],
        },
        RecipeDefinition {
            id: "media-video-probe-compress".to_string(),
            name: "视频探测与压缩".to_string(),
            description: "先探测视频元数据，再按 mode / engine 执行压缩输出。".to_string(),
            category: "media-conversion".to_string(),
            source: RecipeSource::Builtin,
            supports_dry_run: true,
            params: vec![
                RecipeParamDefinition {
                    key: "input".to_string(),
                    label: "输入文件".to_string(),
                    description: "需要探测和压缩的视频文件。".to_string(),
                    default_value: String::new(),
                    required: true,
                    placeholder: "D:/media/demo.mp4".to_string(),
                },
                RecipeParamDefinition {
                    key: "output".to_string(),
                    label: "输出文件".to_string(),
                    description: "压缩后的输出文件路径。".to_string(),
                    default_value: String::new(),
                    required: true,
                    placeholder: "D:/media/demo.small.mp4".to_string(),
                },
                RecipeParamDefinition {
                    key: "mode".to_string(),
                    label: "压缩模式".to_string(),
                    description: "balanced / fastest / smallest".to_string(),
                    default_value: "balanced".to_string(),
                    required: true,
                    placeholder: "balanced".to_string(),
                },
                RecipeParamDefinition {
                    key: "engine".to_string(),
                    label: "压缩引擎".to_string(),
                    description: "auto / cpu / gpu".to_string(),
                    default_value: "auto".to_string(),
                    required: true,
                    placeholder: "auto".to_string(),
                },
            ],
            steps: vec![
                RecipeStepDefinition::Run {
                    id: "video-probe".to_string(),
                    title: "探测视频".to_string(),
                    workspace: "media-conversion".to_string(),
                    action: "video:probe".to_string(),
                    target: "{{input}}".to_string(),
                    summary: "读取 {{input}} 的媒体信息".to_string(),
                    run_args: vec!["video", "probe", "-i", "{{input}}"]
                        .into_iter()
                        .map(str::to_string)
                        .collect(),
                    dry_run_args: vec!["video", "probe", "-i", "{{input}}"]
                        .into_iter()
                        .map(str::to_string)
                        .collect(),
                },
                RecipeStepDefinition::Run {
                    id: "video-compress".to_string(),
                    title: "压缩视频".to_string(),
                    workspace: "media-conversion".to_string(),
                    action: "video:compress".to_string(),
                    target: "{{input}} -> {{output}}".to_string(),
                    summary: "按 {{mode}} / {{engine}} 将 {{input}} 压缩到 {{output}}".to_string(),
                    run_args: vec![
                        "video",
                        "compress",
                        "-i",
                        "{{input}}",
                        "-o",
                        "{{output}}",
                        "--mode",
                        "{{mode}}",
                        "--engine",
                        "{{engine}}",
                    ]
                    .into_iter()
                    .map(str::to_string)
                    .collect(),
                    dry_run_args: vec![
                        "video",
                        "compress",
                        "-i",
                        "{{input}}",
                        "-o",
                        "{{output}}",
                        "--mode",
                        "{{mode}}",
                        "--engine",
                        "{{engine}}",
                    ]
                    .into_iter()
                    .map(str::to_string)
                    .collect(),
                },
            ],
        },
        RecipeDefinition {
            id: "media-video-remux-validate".to_string(),
            name: "视频无损封装校验".to_string(),
            description: "先探测输入，再执行 remux，最后重新探测输出文件。".to_string(),
            category: "media-conversion".to_string(),
            source: RecipeSource::Builtin,
            supports_dry_run: true,
            params: vec![
                RecipeParamDefinition {
                    key: "input".to_string(),
                    label: "输入文件".to_string(),
                    description: "需要执行 remux 的视频文件。".to_string(),
                    default_value: String::new(),
                    required: true,
                    placeholder: "D:/media/demo.mkv".to_string(),
                },
                RecipeParamDefinition {
                    key: "output".to_string(),
                    label: "输出文件".to_string(),
                    description: "remux 后的输出文件路径。".to_string(),
                    default_value: String::new(),
                    required: true,
                    placeholder: "D:/media/demo.mp4".to_string(),
                },
                RecipeParamDefinition {
                    key: "strict".to_string(),
                    label: "严格模式".to_string(),
                    description: "true 表示流不兼容时直接失败。".to_string(),
                    default_value: "true".to_string(),
                    required: true,
                    placeholder: "true".to_string(),
                },
            ],
            steps: vec![
                RecipeStepDefinition::Run {
                    id: "remux-probe-input".to_string(),
                    title: "探测输入文件".to_string(),
                    workspace: "media-conversion".to_string(),
                    action: "video:probe".to_string(),
                    target: "{{input}}".to_string(),
                    summary: "读取 {{input}} 的输入媒体信息".to_string(),
                    run_args: vec!["video", "probe", "-i", "{{input}}"]
                        .into_iter()
                        .map(str::to_string)
                        .collect(),
                    dry_run_args: vec!["video", "probe", "-i", "{{input}}"]
                        .into_iter()
                        .map(str::to_string)
                        .collect(),
                },
                RecipeStepDefinition::Run {
                    id: "video-remux".to_string(),
                    title: "执行无损封装转换".to_string(),
                    workspace: "media-conversion".to_string(),
                    action: "video:remux".to_string(),
                    target: "{{input}} -> {{output}}".to_string(),
                    summary: "按 strict={{strict}} 将 {{input}} 无损封装到 {{output}}".to_string(),
                    run_args: vec![
                        "video",
                        "remux",
                        "-i",
                        "{{input}}",
                        "-o",
                        "{{output}}",
                        "--strict",
                        "{{strict}}",
                    ]
                    .into_iter()
                    .map(str::to_string)
                    .collect(),
                    dry_run_args: vec![
                        "video",
                        "remux",
                        "-i",
                        "{{input}}",
                        "-o",
                        "{{output}}",
                        "--strict",
                        "{{strict}}",
                    ]
                    .into_iter()
                    .map(str::to_string)
                    .collect(),
                },
                RecipeStepDefinition::Run {
                    id: "remux-probe-output".to_string(),
                    title: "探测输出文件".to_string(),
                    workspace: "media-conversion".to_string(),
                    action: "video:probe".to_string(),
                    target: "{{output}}".to_string(),
                    summary: "读取 {{output}} 的输出媒体信息".to_string(),
                    run_args: vec!["video", "probe", "-i", "{{output}}"]
                        .into_iter()
                        .map(str::to_string)
                        .collect(),
                    dry_run_args: vec!["video", "probe", "-i", "{{output}}"]
                        .into_iter()
                        .map(str::to_string)
                        .collect(),
                },
            ],
        },
        RecipeDefinition {
            id: "media-image-batch-convert".to_string(),
            name: "图片目录批量转换".to_string(),
            description: "将整批图片统一转换到目标格式，并在输出目录上执行统计复盘。".to_string(),
            category: "media-conversion".to_string(),
            source: RecipeSource::Builtin,
            supports_dry_run: true,
            params: vec![
                RecipeParamDefinition {
                    key: "input".to_string(),
                    label: "输入目录".to_string(),
                    description: "待处理图片目录。".to_string(),
                    default_value: String::new(),
                    required: true,
                    placeholder: "D:/media/raw".to_string(),
                },
                RecipeParamDefinition {
                    key: "output".to_string(),
                    label: "输出目录".to_string(),
                    description: "转换后的目标目录。".to_string(),
                    default_value: String::new(),
                    required: true,
                    placeholder: "D:/media/out".to_string(),
                },
                RecipeParamDefinition {
                    key: "format".to_string(),
                    label: "输出格式".to_string(),
                    description: "webp / jpeg / png / avif / svg".to_string(),
                    default_value: "webp".to_string(),
                    required: true,
                    placeholder: "webp".to_string(),
                },
                RecipeParamDefinition {
                    key: "quality".to_string(),
                    label: "质量".to_string(),
                    description: "有损编码时的质量参数。".to_string(),
                    default_value: "80".to_string(),
                    required: true,
                    placeholder: "80".to_string(),
                },
            ],
            steps: vec![
                RecipeStepDefinition::Run {
                    id: "image-batch-convert".to_string(),
                    title: "执行图像批量转换".to_string(),
                    workspace: "media-conversion".to_string(),
                    action: "img".to_string(),
                    target: "{{input}} -> {{output}}".to_string(),
                    summary: "按 {{format}} / q={{quality}} 转换 {{input}} 到 {{output}}"
                        .to_string(),
                    run_args: vec![
                        "img",
                        "-i",
                        "{{input}}",
                        "-o",
                        "{{output}}",
                        "-f",
                        "{{format}}",
                        "-q",
                        "{{quality}}",
                    ]
                    .into_iter()
                    .map(str::to_string)
                    .collect(),
                    dry_run_args: vec![
                        "img",
                        "-i",
                        "{{input}}",
                        "-o",
                        "{{output}}",
                        "-f",
                        "{{format}}",
                        "-q",
                        "{{quality}}",
                    ]
                    .into_iter()
                    .map(str::to_string)
                    .collect(),
                },
                RecipeStepDefinition::Run {
                    id: "image-output-cstat".to_string(),
                    title: "复盘输出目录".to_string(),
                    workspace: "statistics-diagnostics".to_string(),
                    action: "cstat".to_string(),
                    target: "{{output}}".to_string(),
                    summary: "对 {{output}} 执行统计扫描，复盘转换产物".to_string(),
                    run_args: vec!["cstat", "{{output}}", "--all", "-f", "json"]
                        .into_iter()
                        .map(str::to_string)
                        .collect(),
                    dry_run_args: vec!["cstat", "{{output}}", "--all", "-f", "json"]
                        .into_iter()
                        .map(str::to_string)
                        .collect(),
                },
            ],
        },
        RecipeDefinition {
            id: "statistics-cstat-review".to_string(),
            name: "目录统计与复盘".to_string(),
            description: "执行全量 cstat 扫描，并回看最近任务，便于诊断目录治理结果。".to_string(),
            category: "statistics-diagnostics".to_string(),
            source: RecipeSource::Builtin,
            supports_dry_run: true,
            params: vec![
                RecipeParamDefinition {
                    key: "path".to_string(),
                    label: "扫描路径".to_string(),
                    description: "cstat 需要扫描的根路径。".to_string(),
                    default_value: ".".to_string(),
                    required: true,
                    placeholder: ".".to_string(),
                },
                RecipeParamDefinition {
                    key: "output".to_string(),
                    label: "导出 JSON".to_string(),
                    description: "用于留存 cstat 结果的 JSON 文件路径。".to_string(),
                    default_value: "report.json".to_string(),
                    required: true,
                    placeholder: "report.json".to_string(),
                },
                RecipeParamDefinition {
                    key: "limit".to_string(),
                    label: "最近任务数量".to_string(),
                    description: "复盘最近任务时使用的记录数量。".to_string(),
                    default_value: "10".to_string(),
                    required: true,
                    placeholder: "10".to_string(),
                },
            ],
            steps: vec![
                RecipeStepDefinition::Run {
                    id: "cstat-full-scan".to_string(),
                    title: "执行目录统计".to_string(),
                    workspace: "statistics-diagnostics".to_string(),
                    action: "cstat".to_string(),
                    target: "{{path}}".to_string(),
                    summary: "对 {{path}} 执行 --all 扫描并导出到 {{output}}".to_string(),
                    run_args: vec![
                        "cstat",
                        "{{path}}",
                        "--all",
                        "-f",
                        "json",
                        "-o",
                        "{{output}}",
                    ]
                    .into_iter()
                    .map(str::to_string)
                    .collect(),
                    dry_run_args: vec![
                        "cstat",
                        "{{path}}",
                        "--all",
                        "-f",
                        "json",
                        "-o",
                        "{{output}}",
                    ]
                    .into_iter()
                    .map(str::to_string)
                    .collect(),
                },
                RecipeStepDefinition::Run {
                    id: "recent-review".to_string(),
                    title: "回看最近任务".to_string(),
                    workspace: "paths-context".to_string(),
                    action: "recent".to_string(),
                    target: "{{limit}}".to_string(),
                    summary: "回看最近 {{limit}} 条任务上下文，辅助复盘".to_string(),
                    run_args: vec!["recent", "-n", "{{limit}}", "-f", "json"]
                        .into_iter()
                        .map(str::to_string)
                        .collect(),
                    dry_run_args: vec!["recent", "-n", "{{limit}}", "-f", "json"]
                        .into_iter()
                        .map(str::to_string)
                        .collect(),
                },
            ],
        },
        RecipeDefinition {
            id: "proxy-diagnostics".to_string(),
            name: "代理诊断快照".to_string(),
            description: "连续采集代理状态、系统代理探测与端口快照，便于排障留档。".to_string(),
            category: "network-proxy".to_string(),
            source: RecipeSource::Builtin,
            supports_dry_run: true,
            params: vec![],
            steps: vec![
                RecipeStepDefinition::Run {
                    id: "proxy-status".to_string(),
                    title: "查看代理状态".to_string(),
                    workspace: "network-proxy".to_string(),
                    action: "pst".to_string(),
                    target: String::new(),
                    summary: "查看当前代理状态快照".to_string(),
                    run_args: vec!["pst", "-f", "json"]
                        .into_iter()
                        .map(str::to_string)
                        .collect(),
                    dry_run_args: vec!["pst", "-f", "json"]
                        .into_iter()
                        .map(str::to_string)
                        .collect(),
                },
                RecipeStepDefinition::Run {
                    id: "proxy-detect".to_string(),
                    title: "探测系统代理".to_string(),
                    workspace: "network-proxy".to_string(),
                    action: "proxy:detect".to_string(),
                    target: String::new(),
                    summary: "读取系统代理配置".to_string(),
                    run_args: vec!["proxy", "detect", "-f", "json"]
                        .into_iter()
                        .map(str::to_string)
                        .collect(),
                    dry_run_args: vec!["proxy", "detect", "-f", "json"]
                        .into_iter()
                        .map(str::to_string)
                        .collect(),
                },
                RecipeStepDefinition::Run {
                    id: "ports-snapshot".to_string(),
                    title: "查看端口快照".to_string(),
                    workspace: "network-proxy".to_string(),
                    action: "ports".to_string(),
                    target: String::new(),
                    summary: "查看当前端口占用快照".to_string(),
                    run_args: vec!["ports", "-f", "json"]
                        .into_iter()
                        .map(str::to_string)
                        .collect(),
                    dry_run_args: vec!["ports", "-f", "json"]
                        .into_iter()
                        .map(str::to_string)
                        .collect(),
                },
            ],
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    use axum::Router;
    use axum::body::{Body, to_bytes};
    use axum::http::Request;
    use axum::routing::{get, post};
    use tower::ServiceExt;

    #[derive(Default)]
    struct FakeRunner {
        outputs: Mutex<Vec<TaskProcessOutput>>,
        calls: Mutex<Vec<Vec<String>>>,
    }

    impl FakeRunner {
        fn new(outputs: Vec<TaskProcessOutput>) -> Self {
            Self {
                outputs: Mutex::new(outputs),
                calls: Mutex::new(Vec::new()),
            }
        }

        fn calls(&self) -> Vec<Vec<String>> {
            self.calls.lock().unwrap().clone()
        }
    }

    impl crate::commands::dashboard::handlers::TaskRunner for FakeRunner {
        fn run(&self, args: &[String]) -> TaskProcessOutput {
            self.calls.lock().unwrap().push(args.to_vec());
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
            .route("/api/workspaces/recipes", get(list_workspace_recipes))
            .route("/api/workspaces/recipes", post(upsert_workspace_recipe))
            .route(
                "/api/workspaces/recipes/preview",
                post(preview_workspace_recipe),
            )
            .route(
                "/api/workspaces/recipes/execute",
                post(execute_workspace_recipe),
            )
            .route(
                "/api/workspaces/tasks/recent",
                get(super::super::workspace_recent_tasks),
            )
            .with_state(state)
    }

    fn custom_recipe() -> RecipeDefinition {
        RecipeDefinition {
            id: "custom-seq".to_string(),
            name: "Custom Seq".to_string(),
            description: "custom".to_string(),
            category: "statistics-diagnostics".to_string(),
            source: RecipeSource::Custom,
            supports_dry_run: true,
            params: vec![RecipeParamDefinition {
                key: "path".to_string(),
                label: "路径".to_string(),
                description: String::new(),
                default_value: "D:/tmp/demo.txt".to_string(),
                required: true,
                placeholder: String::new(),
            }],
            steps: vec![
                RecipeStepDefinition::Run {
                    id: "run-preview".to_string(),
                    title: "预览".to_string(),
                    workspace: "statistics-diagnostics".to_string(),
                    action: "recent".to_string(),
                    target: "{{path}}".to_string(),
                    summary: "检查 {{path}}".to_string(),
                    run_args: vec!["recent", "-f", "json"]
                        .into_iter()
                        .map(str::to_string)
                        .collect(),
                    dry_run_args: vec!["recent", "-f", "json"]
                        .into_iter()
                        .map(str::to_string)
                        .collect(),
                },
                RecipeStepDefinition::Guarded {
                    id: "guarded-rm".to_string(),
                    title: "删除".to_string(),
                    workspace: "files-security".to_string(),
                    action: "rm".to_string(),
                    target: "{{path}}".to_string(),
                    summary: "删除 {{path}}".to_string(),
                    preview_args: vec!["rm", "--dry-run", "-f", "json", "{{path}}"]
                        .into_iter()
                        .map(str::to_string)
                        .collect(),
                    execute_args: vec!["rm", "-y", "-f", "json", "{{path}}"]
                        .into_iter()
                        .map(str::to_string)
                        .collect(),
                    preview_summary: "删除 {{path}}".to_string(),
                },
            ],
        }
    }

    #[tokio::test]
    async fn builtin_recipe_catalog_covers_phase3_to_phase5_workspaces() {
        let app = test_router(Arc::new(FakeRunner::default()));
        let list_resp = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/workspaces/recipes")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(list_resp.status(), StatusCode::OK);
        let body = to_bytes(list_resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let recipes = json["recipes"].as_array().unwrap();

        assert!(
            recipes
                .iter()
                .any(|entry| entry["id"] == "paths-context-health"
                    && entry["category"] == "paths-context")
        );
        assert!(
            recipes
                .iter()
                .any(|entry| entry["id"] == "integration-shell-bootstrap"
                    && entry["category"] == "integration-automation")
        );
        assert!(
            recipes
                .iter()
                .any(|entry| entry["id"] == "media-video-probe-compress"
                    && entry["category"] == "media-conversion")
        );
    }

    #[test]
    fn builtin_recipe_catalog_uses_readable_chinese_labels() {
        let recipes = builtin_recipes();

        let paths = recipes
            .iter()
            .find(|recipe| recipe.id == "paths-context-health")
            .expect("paths-context-health recipe");
        assert_eq!(paths.name, "路径健康巡检");
        assert_eq!(
            paths.description,
            "串联上下文、最近访问与书签健康检查，快速完成路径工作台体检。"
        );

        let shell = recipes
            .iter()
            .find(|recipe| recipe.id == "integration-shell-bootstrap")
            .expect("integration-shell-bootstrap recipe");
        assert_eq!(shell.name, "Shell 安装引导");
        assert_eq!(
            shell.description,
            "依次生成 init 与 completion 输出，用于初始化目标 shell 的集成与补全。"
        );

        let media = recipes
            .iter()
            .find(|recipe| recipe.id == "media-video-probe-compress")
            .expect("media-video-probe-compress recipe");
        assert_eq!(media.name, "视频探测与压缩");
        assert_eq!(
            media.description,
            "先探测视频元数据，再按 mode / engine 执行压缩输出。"
        );

        let proxy = recipes
            .iter()
            .find(|recipe| recipe.id == "proxy-diagnostics")
            .expect("proxy-diagnostics recipe");
        assert_eq!(proxy.name, "代理诊断快照");
        assert_eq!(
            proxy.description,
            "连续采集代理状态、系统代理探测与端口快照，便于排障留档。"
        );
    }

    #[test]
    fn builtin_recipe_catalog_includes_finalization_recipes() {
        let recipes = builtin_recipes();

        for (id, category) in [
            ("media-video-remux-validate", "media-conversion"),
            ("media-image-batch-convert", "media-conversion"),
            ("statistics-cstat-review", "statistics-diagnostics"),
        ] {
            assert!(
                recipes
                    .iter()
                    .any(|recipe| recipe.id == id && recipe.category == category),
                "missing builtin recipe {id} in {category}"
            );
        }
    }

    #[tokio::test]
    async fn recipe_save_and_list_custom_entries() {
        let app = test_router(Arc::new(FakeRunner::default()));
        let save_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/workspaces/recipes")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({ "recipe": custom_recipe() }).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(save_resp.status(), StatusCode::OK);

        let list_resp = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/workspaces/recipes")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(list_resp.status(), StatusCode::OK);
        let body = to_bytes(list_resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(
            json["recipes"]
                .as_array()
                .unwrap()
                .iter()
                .any(|entry| entry["id"] == "custom-seq" && entry["source"] == "custom")
        );
    }

    #[tokio::test]
    async fn recipe_preview_and_execute_require_confirm_and_preserve_guard_chain() {
        let runner = Arc::new(FakeRunner::new(vec![
            ok_output("recipe preview run"),
            ok_output("guarded preview"),
            ok_output("recipe run execute"),
            ok_output("guarded execute"),
        ]));
        let app = test_router(runner);

        let save_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/workspaces/recipes")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({ "recipe": custom_recipe() }).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(save_resp.status(), StatusCode::OK);

        let preview_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/workspaces/recipes/preview")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({ "recipe_id": "custom-seq" }).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(preview_resp.status(), StatusCode::OK);
        let preview_body = to_bytes(preview_resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let preview_json: serde_json::Value = serde_json::from_slice(&preview_body).unwrap();
        assert_eq!(preview_json["status"], "previewed");
        assert_eq!(preview_json["ready_to_execute"], true);
        assert_eq!(preview_json["total_steps"], 2);
        let token = preview_json["token"].as_str().unwrap().to_string();

        let reject_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/workspaces/recipes/execute")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({ "token": token }).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(reject_resp.status(), StatusCode::BAD_REQUEST);

        let execute_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/workspaces/recipes/execute")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({ "token": token, "confirm": true }).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(execute_resp.status(), StatusCode::OK);
        let execute_body = to_bytes(execute_resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let execute_json: serde_json::Value = serde_json::from_slice(&execute_body).unwrap();
        assert_eq!(execute_json["status"], "succeeded");
        assert_eq!(execute_json["completed_steps"], 2);
        assert_eq!(execute_json["failed_step_id"], serde_json::Value::Null);

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
        let recent_body = to_bytes(recent_resp.into_body(), usize::MAX).await.unwrap();
        let recent_json: serde_json::Value = serde_json::from_slice(&recent_body).unwrap();
        assert!(recent_json["entries"].as_array().unwrap().len() >= 4);
    }

    #[tokio::test]
    async fn recipe_execution_stops_after_first_failed_step() {
        let runner = Arc::new(FakeRunner::new(vec![
            ok_output("preview step 1"),
            ok_output("preview step 2"),
            ok_output("preview step 3"),
            ok_output("execute step 1"),
            fail_output("execute step 2 failed"),
        ]));

        let recipe = RecipeDefinition {
            id: "fail-fast".to_string(),
            name: "Fail Fast".to_string(),
            description: String::new(),
            category: "statistics-diagnostics".to_string(),
            source: RecipeSource::Custom,
            supports_dry_run: true,
            params: vec![],
            steps: vec![
                RecipeStepDefinition::Run {
                    id: "step-1".to_string(),
                    title: "step-1".to_string(),
                    workspace: "statistics-diagnostics".to_string(),
                    action: "recent".to_string(),
                    target: String::new(),
                    summary: "step-1".to_string(),
                    run_args: vec!["recent"].into_iter().map(str::to_string).collect(),
                    dry_run_args: vec!["recent"].into_iter().map(str::to_string).collect(),
                },
                RecipeStepDefinition::Run {
                    id: "step-2".to_string(),
                    title: "step-2".to_string(),
                    workspace: "statistics-diagnostics".to_string(),
                    action: "recent".to_string(),
                    target: String::new(),
                    summary: "step-2".to_string(),
                    run_args: vec!["recent", "-n", "2"]
                        .into_iter()
                        .map(str::to_string)
                        .collect(),
                    dry_run_args: vec!["recent", "-n", "2"]
                        .into_iter()
                        .map(str::to_string)
                        .collect(),
                },
                RecipeStepDefinition::Run {
                    id: "step-3".to_string(),
                    title: "step-3".to_string(),
                    workspace: "statistics-diagnostics".to_string(),
                    action: "recent".to_string(),
                    target: String::new(),
                    summary: "step-3".to_string(),
                    run_args: vec!["recent", "-n", "3"]
                        .into_iter()
                        .map(str::to_string)
                        .collect(),
                    dry_run_args: vec!["recent", "-n", "3"]
                        .into_iter()
                        .map(str::to_string)
                        .collect(),
                },
            ],
        };

        let app = test_router(runner.clone());
        let save_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/workspaces/recipes")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({ "recipe": recipe }).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(save_resp.status(), StatusCode::OK);

        let preview_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/workspaces/recipes/preview")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({ "recipe_id": "fail-fast" }).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(preview_resp.status(), StatusCode::OK);
        let preview_body = to_bytes(preview_resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let preview_json: serde_json::Value = serde_json::from_slice(&preview_body).unwrap();
        let token = preview_json["token"].as_str().unwrap().to_string();

        let execute_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/workspaces/recipes/execute")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({ "token": token, "confirm": true }).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(execute_resp.status(), StatusCode::OK);
        let execute_body = to_bytes(execute_resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let execute_json: serde_json::Value = serde_json::from_slice(&execute_body).unwrap();
        assert_eq!(execute_json["status"], "failed");
        assert_eq!(execute_json["completed_steps"], 1);
        assert_eq!(execute_json["failed_step_id"], "step-2");
        assert_eq!(execute_json["steps"].as_array().unwrap().len(), 2);
        assert_eq!(runner.calls().len(), 5);
    }
}
