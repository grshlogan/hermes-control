use std::collections::BTreeMap;

use hermes_control_types::{
    ActionRequest, ActiveRouteStatus, AuditEventSummary, CancelRequest, ConfirmRequest,
    ConfirmationLifecycleResponse, HealthStatus, HermesAction, ModelAction, ModelRuntimeSummary,
    OpenWebUiAction, OperationResponse, ProviderConfig, ReadOnlyStatus, Requester,
    RequesterChannel, RouteRollbackRequest, RouteSwitchRequest, WslAction,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuiConnectionSummary {
    pub daemon_url: String,
    pub operator_id: String,
    pub token_configured: bool,
    pub token_label: String,
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

pub fn gui_connection_summary_from_env() -> Result<GuiConnectionSummary, GuiError> {
    gui_connection_summary_from_env_iter(std::env::vars())
}

pub fn gui_connection_summary_from_env_iter<I, K, V>(
    vars: I,
) -> Result<GuiConnectionSummary, GuiError>
where
    I: IntoIterator<Item = (K, V)>,
    K: Into<String>,
    V: Into<String>,
{
    let env = vars
        .into_iter()
        .map(|(key, value)| (key.into(), value.into()))
        .collect::<BTreeMap<_, _>>();

    let daemon_base_url: Url = env
        .get("HERMES_CONTROL_DAEMON_URL")
        .map(String::as_str)
        .unwrap_or("http://127.0.0.1:18787")
        .parse()?;
    let api_token = env
        .get("HERMES_CONTROL_API_TOKEN")
        .cloned()
        .unwrap_or_default();
    let token = api_token.trim();
    let operator_id = env
        .get("HERMES_CONTROL_GUI_OPERATOR_ID")
        .cloned()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "local-gui".to_owned());

    Ok(GuiConnectionSummary {
        daemon_url: daemon_base_url.to_string(),
        operator_id,
        token_configured: !token.is_empty(),
        token_label: if token.is_empty() {
            "not set".to_owned()
        } else {
            redact_token(token)
        },
    })
}

fn redact_token(token: &str) -> String {
    let suffix = token
        .chars()
        .rev()
        .take(4)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<String>();
    format!("****{suffix}")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuiDaemonCommand {
    DashboardSnapshot,
    RouteSwitchPreview,
    RouteSwitchExecute,
    RouteRollbackPreview,
    RouteRollbackExecute,
    ConfirmOperation,
    CancelOperation,
    ModelActionPreview,
    ModelActionExecute,
    WslActionPreview,
    WslActionExecute,
    HermesActionPreview,
    HermesActionExecute,
    OpenWebUiActionPreview,
    OpenWebUiActionExecute,
    LogsTail,
}

impl GuiDaemonCommand {
    pub fn all() -> Vec<Self> {
        vec![
            Self::DashboardSnapshot,
            Self::RouteSwitchPreview,
            Self::RouteSwitchExecute,
            Self::RouteRollbackPreview,
            Self::RouteRollbackExecute,
            Self::ConfirmOperation,
            Self::CancelOperation,
            Self::ModelActionPreview,
            Self::ModelActionExecute,
            Self::WslActionPreview,
            Self::WslActionExecute,
            Self::HermesActionPreview,
            Self::HermesActionExecute,
            Self::OpenWebUiActionPreview,
            Self::OpenWebUiActionExecute,
            Self::LogsTail,
        ]
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::DashboardSnapshot => "dashboard_snapshot",
            Self::RouteSwitchPreview => "route_switch_preview",
            Self::RouteSwitchExecute => "route_switch_execute",
            Self::RouteRollbackPreview => "route_rollback_preview",
            Self::RouteRollbackExecute => "route_rollback_execute",
            Self::ConfirmOperation => "confirm_operation",
            Self::CancelOperation => "cancel_operation",
            Self::ModelActionPreview => "model_action_preview",
            Self::ModelActionExecute => "model_action_execute",
            Self::WslActionPreview => "wsl_action_preview",
            Self::WslActionExecute => "wsl_action_execute",
            Self::HermesActionPreview => "hermes_action_preview",
            Self::HermesActionExecute => "hermes_action_execute",
            Self::OpenWebUiActionPreview => "openwebui_action_preview",
            Self::OpenWebUiActionExecute => "openwebui_action_execute",
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

pub fn operation_confirm_request(
    code: impl Into<String>,
    operator_id: impl Into<String>,
) -> ConfirmRequest {
    ConfirmRequest {
        requester: gui_requester(operator_id),
        code: code.into(),
    }
}

pub fn operation_cancel_request(operator_id: impl Into<String>) -> CancelRequest {
    CancelRequest {
        requester: gui_requester(operator_id),
    }
}

pub fn model_action_request(
    action: ModelAction,
    model_id: impl Into<String>,
    operator_id: impl Into<String>,
    dry_run: bool,
) -> ActionRequest<ModelAction> {
    let model_id = model_id.into();
    let action_label = match action {
        ModelAction::Install => "install",
        ModelAction::Start => "start",
        ModelAction::Stop => "stop",
        ModelAction::Restart => "restart",
        ModelAction::Health => "health",
        ModelAction::Logs => "logs",
        ModelAction::Benchmark => "benchmark",
    };

    ActionRequest {
        requester: gui_requester(operator_id),
        action,
        reason: format!("GUI model {action_label} {model_id}"),
        dry_run,
    }
}

pub fn wsl_action_request(
    action: WslAction,
    operator_id: impl Into<String>,
    dry_run: bool,
) -> ActionRequest<WslAction> {
    let action_label = match action {
        WslAction::Wake => "wake",
        WslAction::StopDistro => "stop distro",
        WslAction::RestartDistro => "restart distro",
        WslAction::ShutdownAll => "shutdown all",
    };

    ActionRequest {
        requester: gui_requester(operator_id),
        action,
        reason: format!("GUI WSL {action_label}"),
        dry_run,
    }
}

pub fn hermes_action_request(
    action: HermesAction,
    operator_id: impl Into<String>,
    dry_run: bool,
) -> ActionRequest<HermesAction> {
    let action_label = match action {
        HermesAction::Wake => "wake",
        HermesAction::Stop => "stop",
        HermesAction::Restart => "restart",
        HermesAction::Kill => "kill",
    };

    ActionRequest {
        requester: gui_requester(operator_id),
        action,
        reason: format!("GUI Hermes {action_label}"),
        dry_run,
    }
}

pub fn openwebui_action_request(
    action: OpenWebUiAction,
    operator_id: impl Into<String>,
    dry_run: bool,
) -> ActionRequest<OpenWebUiAction> {
    let action_label = match action {
        OpenWebUiAction::Wake => "wake",
        OpenWebUiAction::Stop => "stop",
        OpenWebUiAction::Restart => "restart",
        OpenWebUiAction::Status => "status",
    };

    ActionRequest {
        requester: gui_requester(operator_id),
        action,
        reason: format!("GUI Open WebUI {action_label}"),
        dry_run,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GuiLogTarget {
    Daemon,
    Bot,
    Hermes,
    Vllm,
}

impl GuiLogTarget {
    pub fn all() -> Vec<Self> {
        vec![Self::Daemon, Self::Bot, Self::Hermes, Self::Vllm]
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Daemon => "daemon",
            Self::Bot => "bot",
            Self::Hermes => "hermes",
            Self::Vllm => "vllm",
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
        let models = status.models.clone();
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

    pub async fn route_switch_execute(
        &self,
        profile_id: impl Into<String>,
        operator_id: impl Into<String>,
    ) -> Result<OperationResponse, GuiError> {
        self.post_json(
            "/v1/route/switch",
            &route_switch_request(profile_id, operator_id, false),
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

    pub async fn route_rollback_execute(
        &self,
        operator_id: impl Into<String>,
    ) -> Result<OperationResponse, GuiError> {
        self.post_json(
            "/v1/route/rollback",
            &route_rollback_request(operator_id, false),
        )
        .await
    }

    pub async fn confirm_operation(
        &self,
        code: impl Into<String>,
        operator_id: impl Into<String>,
    ) -> Result<ConfirmationLifecycleResponse, GuiError> {
        self.post_json("/v1/confirm", &operation_confirm_request(code, operator_id))
            .await
    }

    pub async fn cancel_operation(
        &self,
        operator_id: impl Into<String>,
    ) -> Result<ConfirmationLifecycleResponse, GuiError> {
        self.post_json("/v1/cancel", &operation_cancel_request(operator_id))
            .await
    }

    pub async fn model_action_preview(
        &self,
        model_id: impl Into<String>,
        action: ModelAction,
        operator_id: impl Into<String>,
    ) -> Result<OperationResponse, GuiError> {
        self.model_action(model_id, action, operator_id, true).await
    }

    pub async fn model_action_execute(
        &self,
        model_id: impl Into<String>,
        action: ModelAction,
        operator_id: impl Into<String>,
    ) -> Result<OperationResponse, GuiError> {
        self.model_action(model_id, action, operator_id, false)
            .await
    }

    pub async fn wsl_action_preview(
        &self,
        action: WslAction,
        operator_id: impl Into<String>,
    ) -> Result<OperationResponse, GuiError> {
        self.post_json(
            "/v1/wsl/action",
            &wsl_action_request(action, operator_id, true),
        )
        .await
    }

    pub async fn wsl_action_execute(
        &self,
        action: WslAction,
        operator_id: impl Into<String>,
    ) -> Result<OperationResponse, GuiError> {
        self.post_json(
            "/v1/wsl/action",
            &wsl_action_request(action, operator_id, false),
        )
        .await
    }

    pub async fn hermes_action_preview(
        &self,
        action: HermesAction,
        operator_id: impl Into<String>,
    ) -> Result<OperationResponse, GuiError> {
        self.post_json(
            "/v1/hermes/action",
            &hermes_action_request(action, operator_id, true),
        )
        .await
    }

    pub async fn hermes_action_execute(
        &self,
        action: HermesAction,
        operator_id: impl Into<String>,
    ) -> Result<OperationResponse, GuiError> {
        self.post_json(
            "/v1/hermes/action",
            &hermes_action_request(action, operator_id, false),
        )
        .await
    }

    pub async fn openwebui_action_preview(
        &self,
        action: OpenWebUiAction,
        operator_id: impl Into<String>,
    ) -> Result<OperationResponse, GuiError> {
        self.post_json(
            "/v1/openwebui/action",
            &openwebui_action_request(action, operator_id, true),
        )
        .await
    }

    pub async fn openwebui_action_execute(
        &self,
        action: OpenWebUiAction,
        operator_id: impl Into<String>,
    ) -> Result<OperationResponse, GuiError> {
        self.post_json(
            "/v1/openwebui/action",
            &openwebui_action_request(action, operator_id, false),
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

    async fn model_action(
        &self,
        model_id: impl Into<String>,
        action: ModelAction,
        operator_id: impl Into<String>,
        dry_run: bool,
    ) -> Result<OperationResponse, GuiError> {
        let model_id = model_id.into();
        self.post_json(
            &format!("/v1/models/{model_id}/action"),
            &model_action_request(action, model_id, operator_id, dry_run),
        )
        .await
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
