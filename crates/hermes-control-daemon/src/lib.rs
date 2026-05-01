use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{
    Json, Router,
    extract::{Query, State},
    http::{HeaderMap, StatusCode, header::AUTHORIZATION},
    routing::{get, post},
};
use hermes_control_core::{
    ConfigError, HermesRuntimeController, OperationPlan, WslController, collect_read_only_status,
    load_config_dir,
};
use hermes_control_types::{
    ActionRequest, ActiveRouteStatus, AuditEventSummary, CancelRequest, CommandPreview,
    ConfirmRequest, ConfirmationLifecycleResponse, HealthStatus, HermesAction, ModelRuntimeSummary,
    OperationResponse, ProviderConfig, ReadOnlyStatus, Requester, RequesterChannel, RiskLevel,
    WslAction,
};
use rusqlite::{Connection, OptionalExtension};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DaemonError {
    #[error("daemon API token cannot be empty")]
    EmptyApiToken,
    #[error("config error: {0}")]
    Config(#[from] ConfigError),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Clone)]
struct AppState {
    config_dir: Arc<PathBuf>,
    api_token: Arc<str>,
    store: DaemonStateStore,
    executor: Arc<dyn OperationExecutor>,
}

#[derive(Debug, Clone)]
struct DaemonStateStore {
    state_db: Arc<PathBuf>,
    audit_db: Arc<PathBuf>,
}

type ApiResult<T> = Result<Json<T>, StatusCode>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutableOperation {
    pub id: String,
    pub confirmation_id: String,
    pub action: String,
    pub requester_channel: String,
    pub requester_user_id: String,
    pub summary: String,
    pub commands: Vec<CommandPreview>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionOutcome {
    pub status: String,
    pub summary: String,
}

pub trait OperationExecutor: Send + Sync + 'static {
    fn execute(&self, operation: &ExecutableOperation) -> ExecutionOutcome;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOutput {
    pub status_code: i32,
    pub stdout: String,
    pub stderr: String,
}

pub trait CommandRunner: Send + Sync + 'static {
    fn run(&self, command: &CommandPreview) -> CommandOutput;
}

#[derive(Debug, Default)]
pub struct WindowsProcessRunner;

impl CommandRunner for WindowsProcessRunner {
    fn run(&self, command: &CommandPreview) -> CommandOutput {
        match Command::new(&command.program).args(&command.args).output() {
            Ok(output) => CommandOutput {
                status_code: output.status.code().unwrap_or(-1),
                stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            },
            Err(err) => CommandOutput {
                status_code: -1,
                stdout: String::new(),
                stderr: err.to_string(),
            },
        }
    }
}

#[derive(Clone)]
pub struct WindowsCommandExecutor {
    runner: Arc<dyn CommandRunner>,
}

impl Default for WindowsCommandExecutor {
    fn default() -> Self {
        Self::new(Arc::new(WindowsProcessRunner))
    }
}

impl WindowsCommandExecutor {
    pub fn new(runner: Arc<dyn CommandRunner>) -> Self {
        Self { runner }
    }
}

impl OperationExecutor for WindowsCommandExecutor {
    fn execute(&self, operation: &ExecutableOperation) -> ExecutionOutcome {
        for command in &operation.commands {
            if !is_allowlisted_command(command) {
                return ExecutionOutcome {
                    status: "failed".to_owned(),
                    summary: format!(
                        "Command is not allowlisted for execution: {} {}",
                        command.program,
                        command.args.join(" ")
                    ),
                };
            }
        }

        if operation.commands.is_empty() {
            return ExecutionOutcome {
                status: "completed".to_owned(),
                summary: format!(
                    "Operation {} has no command previews to execute.",
                    operation.id
                ),
            };
        }

        for command in &operation.commands {
            let output = self.runner.run(command);
            if output.status_code != 0 {
                return ExecutionOutcome {
                    status: "failed".to_owned(),
                    summary: format!(
                        "Command failed with status {}: {}",
                        output.status_code,
                        output.stderr.trim()
                    ),
                };
            }
        }

        ExecutionOutcome {
            status: "completed".to_owned(),
            summary: format!(
                "Executed {} allowlisted command(s) for {}.",
                operation.commands.len(),
                operation.id
            ),
        }
    }
}

#[derive(Debug, Default)]
pub struct NoopOperationExecutor;

impl OperationExecutor for NoopOperationExecutor {
    fn execute(&self, operation: &ExecutableOperation) -> ExecutionOutcome {
        ExecutionOutcome {
            status: "completed".to_owned(),
            summary: format!(
                "No-op executor accepted operation {} without running system commands.",
                operation.id
            ),
        }
    }
}

pub fn build_router(
    config_dir: impl AsRef<Path>,
    api_token: impl Into<String>,
) -> Result<Router, DaemonError> {
    build_router_with_executor(config_dir, api_token, Arc::new(NoopOperationExecutor))
}

pub fn build_router_with_executor(
    config_dir: impl AsRef<Path>,
    api_token: impl Into<String>,
    executor: Arc<dyn OperationExecutor>,
) -> Result<Router, DaemonError> {
    let config_dir = config_dir.as_ref().to_path_buf();
    let api_token = api_token.into();
    if api_token.trim().is_empty() {
        return Err(DaemonError::EmptyApiToken);
    }

    let config = load_config_dir(&config_dir)?;
    let project_root = project_root_for_config_dir(&config_dir);
    let store = DaemonStateStore::initialize(
        &project_root,
        &config.control.daemon.state_db,
        &config.control.daemon.audit_db,
    )?;

    let state = AppState {
        config_dir: Arc::new(config_dir),
        api_token: Arc::<str>::from(api_token),
        store,
        executor,
    };

    Ok(Router::new()
        .route("/v1/status", get(status))
        .route("/v1/health", get(health))
        .route("/v1/providers", get(providers))
        .route("/v1/models", get(models))
        .route("/v1/route/active", get(active_route))
        .route("/v1/audit", get(audit_events))
        .route("/v1/wsl/status", get(wsl_status))
        .route("/v1/wsl/action", post(wsl_action))
        .route("/v1/hermes/status", get(hermes_status))
        .route("/v1/hermes/action", post(hermes_action))
        .route("/v1/confirm", post(confirm_action))
        .route("/v1/cancel", post(cancel_action))
        .with_state(state))
}

async fn status(State(state): State<AppState>, headers: HeaderMap) -> ApiResult<ReadOnlyStatus> {
    require_auth(&state, &headers)?;
    collect_read_only_status(&*state.config_dir)
        .await
        .map(Json)
        .map_err(|err| {
            tracing::warn!(error = %err, "failed to collect daemon status");
            StatusCode::INTERNAL_SERVER_ERROR
        })
}

async fn health(State(state): State<AppState>, headers: HeaderMap) -> ApiResult<HealthStatus> {
    require_auth(&state, &headers)?;
    collect_read_only_status(&*state.config_dir)
        .await
        .map(|status| Json(status.overall))
        .map_err(|err| {
            tracing::warn!(error = %err, "failed to collect daemon health");
            StatusCode::INTERNAL_SERVER_ERROR
        })
}

async fn providers(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Vec<ProviderConfig>> {
    require_auth(&state, &headers)?;
    load_config_dir(&*state.config_dir)
        .map(|config| Json(config.providers.providers))
        .map_err(|err| {
            tracing::warn!(error = %err, "failed to load providers");
            StatusCode::INTERNAL_SERVER_ERROR
        })
}

async fn models(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Vec<ModelRuntimeSummary>> {
    require_auth(&state, &headers)?;
    collect_read_only_status(&*state.config_dir)
        .await
        .map(|status| Json(status.models))
        .map_err(|err| {
            tracing::warn!(error = %err, "failed to collect model status");
            StatusCode::INTERNAL_SERVER_ERROR
        })
}

async fn active_route(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<ActiveRouteStatus> {
    require_auth(&state, &headers)?;
    state.store.active_route().map(Json).map_err(|err| {
        tracing::warn!(error = %err, "failed to read active route");
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

async fn audit_events(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<HashMap<String, String>>,
) -> ApiResult<Vec<AuditEventSummary>> {
    require_auth(&state, &headers)?;
    let limit = query
        .get("limit")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(100);

    state.store.audit_events(limit).map(Json).map_err(|err| {
        tracing::warn!(error = %err, "failed to read audit events");
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

async fn wsl_status(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Option<hermes_control_types::WslDistroStatus>> {
    require_auth(&state, &headers)?;
    collect_read_only_status(&*state.config_dir)
        .await
        .map(|status| Json(status.wsl))
        .map_err(|err| {
            tracing::warn!(error = %err, "failed to collect WSL status");
            StatusCode::INTERNAL_SERVER_ERROR
        })
}

async fn hermes_status(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<hermes_control_types::EndpointStatus> {
    require_auth(&state, &headers)?;
    collect_read_only_status(&*state.config_dir)
        .await
        .map(|status| Json(status.hermes))
        .map_err(|err| {
            tracing::warn!(error = %err, "failed to collect Hermes status");
            StatusCode::INTERNAL_SERVER_ERROR
        })
}

async fn wsl_action(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<ActionRequest<WslAction>>,
) -> ApiResult<OperationResponse> {
    require_auth(&state, &headers)?;
    let config = load_config_dir(&*state.config_dir).map_err(|err| {
        tracing::warn!(error = %err, "failed to load WSL action config");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    let controller = WslController::with_default_user(
        config.control.wsl.distro,
        config.control.wsl.default_user,
    );
    let action = format!("wsl::{:?}", request.action);
    let plan = controller.plan(request.action);
    operation_response(&state, request.requester, action, request.dry_run, plan).map(Json)
}

async fn hermes_action(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<ActionRequest<HermesAction>>,
) -> ApiResult<OperationResponse> {
    require_auth(&state, &headers)?;
    let config = load_config_dir(&*state.config_dir).map_err(|err| {
        tracing::warn!(error = %err, "failed to load Hermes action config");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    let controller = HermesRuntimeController::new(
        config.control.hermes.agent_root,
        config.control.hermes.health_url,
    );
    let action = format!("hermes::{:?}", request.action);
    let plan = controller.plan(request.action);
    operation_response(&state, request.requester, action, request.dry_run, plan).map(Json)
}

fn operation_response(
    state: &AppState,
    requester: Requester,
    action: String,
    dry_run: bool,
    plan: OperationPlan,
) -> Result<OperationResponse, StatusCode> {
    if dry_run {
        return Ok(OperationResponse {
            status: "dry_run".to_owned(),
            risk: plan.risk,
            summary: plan.summary,
            dry_run: true,
            commands: plan.commands,
            confirmation_id: None,
            code_hint: None,
            expires_at: None,
        });
    }

    if plan.requires_confirmation
        && matches!(plan.risk, RiskLevel::Destructive | RiskLevel::Experimental)
    {
        if state.store.has_active_operation().map_err(|err| {
            tracing::warn!(error = %err, "failed to check operation lock");
            StatusCode::INTERNAL_SERVER_ERROR
        })? {
            return Err(StatusCode::CONFLICT);
        }

        let confirmation = state
            .store
            .create_confirmation(&requester, &action, &plan)
            .map_err(|err| {
                tracing::warn!(error = %err, "failed to create confirmation");
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
        state
            .store
            .append_audit_event(&requester, &action, &plan)
            .map_err(|err| {
                tracing::warn!(error = %err, "failed to append audit event");
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        return Ok(OperationResponse {
            status: "confirmation_required".to_owned(),
            risk: plan.risk,
            summary: plan.summary,
            dry_run: false,
            commands: plan.commands,
            confirmation_id: Some(confirmation.id),
            code_hint: Some(confirmation.code_hint),
            expires_at: Some(confirmation.expires_at),
        });
    }

    state
        .store
        .append_audit_event(&requester, &action, &plan)
        .map_err(|err| {
            tracing::warn!(error = %err, "failed to append audit event");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    Ok(OperationResponse {
        status: "planned".to_owned(),
        risk: plan.risk,
        summary: plan.summary,
        dry_run: false,
        commands: plan.commands,
        confirmation_id: None,
        code_hint: None,
        expires_at: None,
    })
}

async fn confirm_action(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<ConfirmRequest>,
) -> ApiResult<ConfirmationLifecycleResponse> {
    require_auth(&state, &headers)?;
    let operation = state
        .store
        .confirm_pending(&request.requester, &request.code)
        .map_err(|err| {
            tracing::warn!(error = %err, "failed to confirm operation");
            StatusCode::NOT_FOUND
        })?;
    let outcome = state.executor.execute(&operation);
    state
        .store
        .complete_operation(&operation, &outcome, &request.requester)
        .map_err(|err| {
            tracing::warn!(error = %err, "failed to complete operation");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(ConfirmationLifecycleResponse {
        status: "confirmed".to_owned(),
        confirmation_id: operation.confirmation_id,
        summary: outcome.summary,
    }))
}

async fn cancel_action(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<CancelRequest>,
) -> ApiResult<ConfirmationLifecycleResponse> {
    require_auth(&state, &headers)?;
    state
        .store
        .cancel_pending(&request.requester)
        .map(Json)
        .map_err(|err| {
            tracing::warn!(error = %err, "failed to cancel operation");
            StatusCode::NOT_FOUND
        })
}

fn require_auth(state: &AppState, headers: &HeaderMap) -> Result<(), StatusCode> {
    let Some(value) = headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
    else {
        return Err(StatusCode::UNAUTHORIZED);
    };

    let Some(token) = value.strip_prefix("Bearer ") else {
        return Err(StatusCode::UNAUTHORIZED);
    };

    if token == state.api_token.as_ref() {
        Ok(())
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

impl DaemonStateStore {
    fn initialize(
        project_root: &Path,
        state_db: &str,
        audit_db: &str,
    ) -> Result<Self, DaemonError> {
        let state_db = resolve_project_path(project_root, state_db);
        let audit_db = resolve_project_path(project_root, audit_db);

        if let Some(parent) = state_db.parent() {
            fs::create_dir_all(parent)?;
        }
        if let Some(parent) = audit_db.parent() {
            fs::create_dir_all(parent)?;
        }

        let state_connection = Connection::open(&state_db)?;
        state_connection.execute_batch(
            "
            PRAGMA journal_mode = WAL;
            CREATE TABLE IF NOT EXISTS route_state (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                active_profile_id TEXT,
                updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            INSERT OR IGNORE INTO route_state (id, active_profile_id) VALUES (1, NULL);

            CREATE TABLE IF NOT EXISTS operation_state (
                id TEXT PRIMARY KEY,
                action TEXT NOT NULL,
                status TEXT NOT NULL,
                requester_channel TEXT NOT NULL,
                requester_user_id TEXT NOT NULL,
                summary TEXT NOT NULL DEFAULT '',
                commands_json TEXT NOT NULL DEFAULT '[]',
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS confirmations (
                id TEXT PRIMARY KEY,
                operation_id TEXT,
                requester_channel TEXT NOT NULL,
                requester_user_id TEXT NOT NULL,
                action TEXT NOT NULL,
                risk_level TEXT NOT NULL,
                code_hash TEXT NOT NULL,
                expires_at TEXT NOT NULL,
                status TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            ",
        )?;
        ensure_state_column(
            &state_connection,
            "operation_state",
            "summary",
            "TEXT NOT NULL DEFAULT ''",
        )?;
        ensure_state_column(
            &state_connection,
            "operation_state",
            "commands_json",
            "TEXT NOT NULL DEFAULT '[]'",
        )?;

        let audit_connection = Connection::open(&audit_db)?;
        audit_connection.execute_batch(
            "
            PRAGMA journal_mode = WAL;
            CREATE TABLE IF NOT EXISTS audit_events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                happened_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                requester_channel TEXT NOT NULL,
                requester_user_id TEXT NOT NULL,
                action TEXT NOT NULL,
                risk_level TEXT NOT NULL,
                summary TEXT NOT NULL
            );
            ",
        )?;

        Ok(Self {
            state_db: Arc::new(state_db),
            audit_db: Arc::new(audit_db),
        })
    }

    fn active_route(&self) -> Result<ActiveRouteStatus, DaemonError> {
        let connection = Connection::open(&*self.state_db)?;
        let active_profile_id = connection
            .query_row(
                "SELECT active_profile_id FROM route_state WHERE id = 1",
                [],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()?
            .flatten();

        Ok(ActiveRouteStatus { active_profile_id })
    }

    fn audit_events(&self, limit: usize) -> Result<Vec<AuditEventSummary>, DaemonError> {
        let limit = limit.clamp(1, 500) as i64;
        let connection = Connection::open(&*self.audit_db)?;
        let mut statement = connection.prepare(
            "
            SELECT id, happened_at, requester_channel, requester_user_id, action, risk_level, summary
            FROM audit_events
            ORDER BY id DESC
            LIMIT ?1
            ",
        )?;

        let rows = statement.query_map([limit], |row| {
            Ok(AuditEventSummary {
                id: row.get(0)?,
                happened_at: row.get(1)?,
                requester_channel: row.get(2)?,
                requester_user_id: row.get(3)?,
                action: row.get(4)?,
                risk_level: row.get(5)?,
                summary: row.get(6)?,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    fn create_confirmation(
        &self,
        requester: &Requester,
        action: &str,
        plan: &OperationPlan,
    ) -> Result<ConfirmationPreview, DaemonError> {
        let now = unix_epoch_nanos();
        let operation_id = format!("op_{now}");
        let id = format!("confirm_{now}");
        let code_hint = format!("HERMES-{:04}", now % 10000);
        let expires_at = format!("unix:{}", unix_epoch_seconds() + 300);
        let commands_json = serde_json::to_string(&plan.commands)?;
        let connection = Connection::open(&*self.state_db)?;
        connection.execute(
            "
            INSERT INTO operation_state (
                id, action, status, requester_channel, requester_user_id, summary, commands_json
            )
            VALUES (?1, ?2, 'pending_confirmation', ?3, ?4, ?5, ?6)
            ",
            (
                &operation_id,
                action,
                requester_channel_label(&requester.channel),
                &requester.user_id,
                &plan.summary,
                &commands_json,
            ),
        )?;
        connection.execute(
            "
            INSERT INTO confirmations (
                id, operation_id, requester_channel, requester_user_id, action,
                risk_level, code_hash, expires_at, status
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'pending')
            ",
            (
                &id,
                &operation_id,
                requester_channel_label(&requester.channel),
                &requester.user_id,
                action,
                risk_label(&plan.risk),
                &code_hint,
                &expires_at,
            ),
        )?;

        Ok(ConfirmationPreview {
            id,
            code_hint,
            expires_at,
        })
    }

    fn append_audit_event(
        &self,
        requester: &Requester,
        action: &str,
        plan: &OperationPlan,
    ) -> Result<(), DaemonError> {
        let connection = Connection::open(&*self.audit_db)?;
        connection.execute(
            "
            INSERT INTO audit_events (
                requester_channel, requester_user_id, action, risk_level, summary
            )
            VALUES (?1, ?2, ?3, ?4, ?5)
            ",
            (
                requester_channel_label(&requester.channel),
                &requester.user_id,
                action,
                risk_label(&plan.risk),
                &plan.summary,
            ),
        )?;
        Ok(())
    }

    fn has_active_operation(&self) -> Result<bool, DaemonError> {
        let connection = Connection::open(&*self.state_db)?;
        let count = connection.query_row(
            "
            SELECT COUNT(*)
            FROM operation_state
            WHERE status IN ('pending_confirmation', 'running')
            ",
            [],
            |row| row.get::<_, i64>(0),
        )?;
        Ok(count > 0)
    }

    fn confirm_pending(
        &self,
        requester: &Requester,
        code: &str,
    ) -> Result<ExecutableOperation, DaemonError> {
        let pending = self.find_pending_confirmation(requester, Some(code))?;
        let connection = Connection::open(&*self.state_db)?;
        connection.execute(
            "UPDATE confirmations SET status = 'confirmed' WHERE id = ?1 AND status = 'pending'",
            [&pending.confirmation_id],
        )?;
        connection.execute(
            "UPDATE operation_state SET status = 'running', updated_at = CURRENT_TIMESTAMP WHERE id = ?1",
            [&pending.operation_id],
        )?;
        self.append_audit_summary(
            requester,
            &pending.action,
            "NormalMutating",
            &format!("Confirmed pending operation {}", pending.confirmation_id),
        )?;

        Ok(ExecutableOperation {
            id: pending.operation_id,
            confirmation_id: pending.confirmation_id,
            action: pending.action,
            requester_channel: requester_channel_label(&requester.channel).to_owned(),
            requester_user_id: requester.user_id.clone(),
            summary: pending.summary,
            commands: pending.commands,
        })
    }

    fn complete_operation(
        &self,
        operation: &ExecutableOperation,
        outcome: &ExecutionOutcome,
        requester: &Requester,
    ) -> Result<(), DaemonError> {
        let connection = Connection::open(&*self.state_db)?;
        connection.execute(
            "UPDATE operation_state SET status = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
            (&outcome.status, &operation.id),
        )?;
        self.append_audit_summary(
            requester,
            &operation.action,
            "NormalMutating",
            &outcome.summary,
        )
    }

    fn cancel_pending(
        &self,
        requester: &Requester,
    ) -> Result<ConfirmationLifecycleResponse, DaemonError> {
        let pending = self.find_pending_confirmation(requester, None)?;
        let connection = Connection::open(&*self.state_db)?;
        connection.execute(
            "UPDATE confirmations SET status = 'cancelled' WHERE id = ?1 AND status = 'pending'",
            [&pending.confirmation_id],
        )?;
        connection.execute(
            "UPDATE operation_state SET status = 'cancelled', updated_at = CURRENT_TIMESTAMP WHERE id = ?1",
            [&pending.operation_id],
        )?;
        self.append_audit_summary(
            requester,
            &pending.action,
            "NormalMutating",
            &format!("Cancelled pending operation {}", pending.confirmation_id),
        )?;

        Ok(ConfirmationLifecycleResponse {
            status: "cancelled".to_owned(),
            confirmation_id: pending.confirmation_id,
            summary: "Pending operation cancelled.".to_owned(),
        })
    }

    fn find_pending_confirmation(
        &self,
        requester: &Requester,
        code: Option<&str>,
    ) -> Result<PendingConfirmation, DaemonError> {
        let connection = Connection::open(&*self.state_db)?;
        let mut sql = "
            SELECT c.id, c.operation_id, c.action, o.summary, o.commands_json
            FROM confirmations c
            JOIN operation_state o ON o.id = c.operation_id
            WHERE c.status = 'pending'
              AND c.requester_channel = ?1
              AND c.requester_user_id = ?2
        "
        .to_owned();
        if code.is_some() {
            sql.push_str(" AND code_hash = ?3");
        }
        sql.push_str(" ORDER BY c.created_at DESC, c.id DESC LIMIT 1");

        let channel = requester_channel_label(&requester.channel);
        match code {
            Some(code) => connection.query_row(&sql, (channel, &requester.user_id, code), |row| {
                let commands_json = row.get::<_, String>(4)?;
                Ok(PendingConfirmation {
                    confirmation_id: row.get(0)?,
                    operation_id: row.get(1)?,
                    action: row.get(2)?,
                    summary: row.get(3)?,
                    commands: serde_json::from_str(&commands_json).map_err(|err| {
                        rusqlite::Error::FromSqlConversionFailure(
                            4,
                            rusqlite::types::Type::Text,
                            Box::new(err),
                        )
                    })?,
                })
            }),
            None => connection.query_row(&sql, (channel, &requester.user_id), |row| {
                let commands_json = row.get::<_, String>(4)?;
                Ok(PendingConfirmation {
                    confirmation_id: row.get(0)?,
                    operation_id: row.get(1)?,
                    action: row.get(2)?,
                    summary: row.get(3)?,
                    commands: serde_json::from_str(&commands_json).map_err(|err| {
                        rusqlite::Error::FromSqlConversionFailure(
                            4,
                            rusqlite::types::Type::Text,
                            Box::new(err),
                        )
                    })?,
                })
            }),
        }
        .map_err(Into::into)
    }

    fn append_audit_summary(
        &self,
        requester: &Requester,
        action: &str,
        risk_level: &str,
        summary: &str,
    ) -> Result<(), DaemonError> {
        let connection = Connection::open(&*self.audit_db)?;
        connection.execute(
            "
            INSERT INTO audit_events (
                requester_channel, requester_user_id, action, risk_level, summary
            )
            VALUES (?1, ?2, ?3, ?4, ?5)
            ",
            (
                requester_channel_label(&requester.channel),
                &requester.user_id,
                action,
                risk_level,
                summary,
            ),
        )?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ConfirmationPreview {
    id: String,
    code_hint: String,
    expires_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PendingConfirmation {
    confirmation_id: String,
    operation_id: String,
    action: String,
    summary: String,
    commands: Vec<CommandPreview>,
}

fn unix_epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn ensure_state_column(
    connection: &Connection,
    table: &str,
    column: &str,
    definition: &str,
) -> Result<(), DaemonError> {
    let mut statement = connection.prepare(&format!("PRAGMA table_info({table})"))?;
    let names = statement
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<Result<Vec<_>, _>>()?;
    if !names.iter().any(|name| name == column) {
        connection.execute(
            &format!("ALTER TABLE {table} ADD COLUMN {column} {definition}"),
            [],
        )?;
    }
    Ok(())
}

fn is_allowlisted_command(command: &CommandPreview) -> bool {
    if !command.program.eq_ignore_ascii_case("wsl.exe") {
        return false;
    }

    match command.args.as_slice() {
        [arg] if arg == "--shutdown" => true,
        [terminate, distro] if terminate == "--terminate" => is_safe_identifier(distro),
        [distribution, distro, user_flag, user, exec, command]
            if distribution == "--distribution" && user_flag == "--user" && exec == "--exec" =>
        {
            is_safe_identifier(distro) && is_safe_identifier(user) && command == "true"
        }
        _ => false,
    }
}

fn is_safe_identifier(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
}

fn unix_epoch_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0)
}

fn requester_channel_label(channel: &RequesterChannel) -> &'static str {
    match channel {
        RequesterChannel::Gui => "gui",
        RequesterChannel::Cli => "cli",
        RequesterChannel::Telegram => "telegram",
    }
}

fn risk_label(risk: &RiskLevel) -> &'static str {
    match risk {
        RiskLevel::ReadOnly => "ReadOnly",
        RiskLevel::NormalMutating => "NormalMutating",
        RiskLevel::Destructive => "Destructive",
        RiskLevel::Experimental => "Experimental",
    }
}

fn project_root_for_config_dir(config_dir: &Path) -> PathBuf {
    config_dir
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn resolve_project_path(project_root: &Path, configured_path: &str) -> PathBuf {
    let path = PathBuf::from(configured_path);
    if path.is_absolute() {
        path
    } else {
        project_root.join(path)
    }
}
