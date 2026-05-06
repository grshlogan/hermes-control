use std::collections::BTreeMap;

use hermes_control_types::{
    ActiveRouteStatus, AuditEventSummary, HealthStatus, ModelRuntimeSummary, OperationResponse,
    ProviderConfig, ReadOnlyStatus, Requester, RequesterChannel, RouteRollbackRequest,
    RouteSwitchRequest,
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use thiserror::Error;
use url::Url;

#[derive(Debug, Error)]
pub enum GuiError {
    #[error("missing required environment variable {0}")]
    MissingEnv(&'static str),
    #[error("invalid daemon url: {0}")]
    InvalidDaemonUrl(#[from] url::ParseError),
    #[error("daemon request failed: {0}")]
    Http(#[from] reqwest::Error),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuiBoundary {
    pub channel: RequesterChannel,
    pub raw_process_execution: bool,
}

pub fn gui_boundary() -> GuiBoundary {
    GuiBoundary {
        channel: RequesterChannel::Gui,
        raw_process_execution: false,
    }
}

#[derive(Debug, Clone)]
pub struct GuiConfig {
    daemon_base_url: Url,
    api_token: String,
    operator_id: String,
}

impl GuiConfig {
    pub fn from_env() -> Result<Self, GuiError> {
        Self::from_env_iter(std::env::vars())
    }

    pub fn from_env_iter<I, K, V>(vars: I) -> Result<Self, GuiError>
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        let env = vars
            .into_iter()
            .map(|(key, value)| (key.into(), value.into()))
            .collect::<BTreeMap<_, _>>();

        let daemon_base_url = env
            .get("HERMES_CONTROL_DAEMON_URL")
            .map(String::as_str)
            .unwrap_or("http://127.0.0.1:18787")
            .parse()?;
        let api_token = env
            .get("HERMES_CONTROL_API_TOKEN")
            .cloned()
            .filter(|value| !value.trim().is_empty())
            .ok_or(GuiError::MissingEnv("HERMES_CONTROL_API_TOKEN"))?;
        let operator_id = env
            .get("HERMES_CONTROL_GUI_OPERATOR_ID")
            .cloned()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "local-gui".to_owned());

        Ok(Self {
            daemon_base_url,
            api_token,
            operator_id,
        })
    }

    pub fn daemon_base_url(&self) -> &Url {
        &self.daemon_base_url
    }

    pub fn api_token(&self) -> &str {
        &self.api_token
    }

    pub fn operator_id(&self) -> &str {
        &self.operator_id
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuiDaemonCommand {
    DashboardSnapshot,
    RouteSwitchPreview,
    RouteRollbackPreview,
    LogsTail,
}

impl GuiDaemonCommand {
    pub fn all() -> Vec<Self> {
        vec![
            Self::DashboardSnapshot,
            Self::RouteSwitchPreview,
            Self::RouteRollbackPreview,
            Self::LogsTail,
        ]
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::DashboardSnapshot => "dashboard_snapshot",
            Self::RouteSwitchPreview => "route_switch_preview",
            Self::RouteRollbackPreview => "route_rollback_preview",
            Self::LogsTail => "logs_tail",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuiTauriCapability {
    pub identifier: String,
    pub description: String,
    pub windows: Vec<String>,
    pub permissions: Vec<String>,
}

pub fn gui_tauri_capability() -> GuiTauriCapability {
    GuiTauriCapability {
        identifier: "main".to_owned(),
        description: "Main Hermes Control window with core Tauri IPC only.".to_owned(),
        windows: vec!["main".to_owned()],
        permissions: vec!["core:default".to_owned()],
    }
}

pub fn gui_requester(operator_id: impl Into<String>) -> Requester {
    Requester {
        channel: RequesterChannel::Gui,
        user_id: operator_id.into(),
        chat_id: None,
    }
}

pub fn route_switch_request(
    profile_id: impl Into<String>,
    operator_id: impl Into<String>,
    dry_run: bool,
) -> RouteSwitchRequest {
    let profile_id = profile_id.into();
    RouteSwitchRequest {
        requester: gui_requester(operator_id),
        reason: format!("GUI route switch {profile_id}"),
        profile_id,
        dry_run,
    }
}

pub fn route_rollback_request(
    operator_id: impl Into<String>,
    dry_run: bool,
) -> RouteRollbackRequest {
    RouteRollbackRequest {
        requester: gui_requester(operator_id),
        reason: "GUI route rollback".to_owned(),
        dry_run,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GuiLogTarget {
    Daemon,
    Bot,
    Hermes,
}

impl GuiLogTarget {
    pub fn all() -> Vec<Self> {
        vec![Self::Daemon, Self::Bot, Self::Hermes]
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Daemon => "daemon",
            Self::Bot => "bot",
            Self::Hermes => "hermes",
        }
    }
}

pub fn log_tail_path(target: GuiLogTarget, tail: usize) -> Result<String, GuiError> {
    Ok(format!(
        "/v1/logs/{}?tail={}",
        target.as_str(),
        tail.clamp(1, 1000)
    ))
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GuiDashboardSnapshot {
    pub status: ReadOnlyStatus,
    pub active_route: ActiveRouteStatus,
    pub providers: Vec<ProviderConfig>,
    pub models: Vec<ModelRuntimeSummary>,
    pub audit: Vec<AuditEventSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuiLogTail {
    pub target: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    pub tail: usize,
    pub lines: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Clone)]
pub struct GuiDaemonClient {
    client: reqwest::Client,
    base_url: Url,
    api_token: String,
}

impl GuiDaemonClient {
    pub fn from_config(config: &GuiConfig) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: config.daemon_base_url.clone(),
            api_token: config.api_token.clone(),
        }
    }

    pub async fn dashboard_snapshot(&self) -> Result<GuiDashboardSnapshot, GuiError> {
        let status = self.get_json::<ReadOnlyStatus>("/v1/status").await?;
        let active_route = self
            .get_json::<ActiveRouteStatus>("/v1/route/active")
            .await?;
        let providers = self
            .get_json::<Vec<ProviderConfig>>("/v1/providers")
            .await?;
        let models = self
            .get_json::<Vec<ModelRuntimeSummary>>("/v1/models")
            .await?;
        let audit = self
            .get_json::<Vec<AuditEventSummary>>("/v1/audit?limit=20")
            .await?;

        Ok(GuiDashboardSnapshot {
            status,
            active_route,
            providers,
            models,
            audit,
        })
    }

    pub async fn health(&self) -> Result<HealthStatus, GuiError> {
        self.get_json("/v1/health").await
    }

    pub async fn route_switch_preview(
        &self,
        profile_id: impl Into<String>,
        operator_id: impl Into<String>,
    ) -> Result<OperationResponse, GuiError> {
        self.post_json(
            "/v1/route/switch",
            &route_switch_request(profile_id, operator_id, true),
        )
        .await
    }

    pub async fn route_rollback_preview(
        &self,
        operator_id: impl Into<String>,
    ) -> Result<OperationResponse, GuiError> {
        self.post_json(
            "/v1/route/rollback",
            &route_rollback_request(operator_id, true),
        )
        .await
    }

    pub async fn tail_logs(
        &self,
        target: GuiLogTarget,
        tail: usize,
    ) -> Result<GuiLogTail, GuiError> {
        self.get_json(&log_tail_path(target, tail)?).await
    }

    async fn get_json<T>(&self, path: &str) -> Result<T, GuiError>
    where
        T: DeserializeOwned,
    {
        let url = self.base_url.join(path)?;
        let response = self
            .client
            .get(url)
            .bearer_auth(&self.api_token)
            .send()
            .await?
            .error_for_status()?;
        Ok(response.json::<T>().await?)
    }

    async fn post_json<T, R>(&self, path: &str, body: &T) -> Result<R, GuiError>
    where
        T: Serialize,
        R: DeserializeOwned,
    {
        let url = self.base_url.join(path)?;
        let response = self
            .client
            .post(url)
            .bearer_auth(&self.api_token)
            .json(body)
            .send()
            .await?
            .error_for_status()?;
        Ok(response.json::<R>().await?)
    }
}
