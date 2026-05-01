use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use axum::{
    Json, Router,
    extract::{Query, State},
    http::{HeaderMap, StatusCode, header::AUTHORIZATION},
    routing::get,
};
use hermes_control_core::{ConfigError, collect_read_only_status, load_config_dir};
use hermes_control_types::{
    ActiveRouteStatus, AuditEventSummary, HealthStatus, ModelRuntimeSummary, ProviderConfig,
    ReadOnlyStatus,
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
}

#[derive(Debug, Clone)]
struct AppState {
    config_dir: Arc<PathBuf>,
    api_token: Arc<str>,
    store: DaemonStateStore,
}

#[derive(Debug, Clone)]
struct DaemonStateStore {
    state_db: Arc<PathBuf>,
    audit_db: Arc<PathBuf>,
}

type ApiResult<T> = Result<Json<T>, StatusCode>;

pub fn build_router(
    config_dir: impl AsRef<Path>,
    api_token: impl Into<String>,
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
    };

    Ok(Router::new()
        .route("/v1/status", get(status))
        .route("/v1/health", get(health))
        .route("/v1/providers", get(providers))
        .route("/v1/models", get(models))
        .route("/v1/route/active", get(active_route))
        .route("/v1/audit", get(audit_events))
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
