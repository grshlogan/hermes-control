use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AiProviderKind {
    OpenAiCompatible,
    AnthropicClaude,
    DeepSeek,
    Codex,
    LocalVllm,
    LmStudio,
    Disabled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HermesAction {
    Wake,
    Stop,
    Restart,
    Kill,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WslAction {
    Wake,
    StopDistro,
    RestartDistro,
    ShutdownAll,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelAction {
    Install,
    Start,
    Stop,
    Restart,
    Health,
    Logs,
    Benchmark,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskLevel {
    ReadOnly,
    NormalMutating,
    Destructive,
    Experimental,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RequesterChannel {
    Gui,
    Cli,
    Telegram,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Requester {
    pub channel: RequesterChannel,
    pub user_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chat_id: Option<String>,
}

impl Requester {
    pub fn telegram(user_id: impl Into<String>, chat_id: impl Into<String>) -> Self {
        Self {
            channel: RequesterChannel::Telegram,
            user_id: user_id.into(),
            chat_id: Some(chat_id.into()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionRequest<T> {
    pub requester: Requester,
    pub action: T,
    pub reason: String,
    pub dry_run: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteSwitchRequest {
    pub requester: Requester,
    pub profile_id: String,
    pub reason: String,
    pub dry_run: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfirmRequest {
    pub requester: Requester,
    pub code: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CancelRequest {
    pub requester: Requester,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlConfig {
    pub daemon: DaemonConfig,
    pub wsl: WslConfig,
    pub hermes: HermesConfig,
    pub policy: PolicyConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonConfig {
    pub bind: String,
    pub api_token_ref: String,
    pub state_db: String,
    pub audit_db: String,
    pub log_dir: String,
    pub operation_timeout_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WslConfig {
    pub distro: String,
    pub default_user: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HermesConfig {
    pub agent_root: String,
    pub health_url: String,
    pub logs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyConfig {
    pub require_confirm_for_destructive: bool,
    pub allow_lan_bind: bool,
    pub allow_raw_shell: bool,
    pub redact_secrets: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProvidersConfig {
    pub providers: Vec<ProviderConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub id: String,
    pub kind: AiProviderKind,
    pub display_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key_ref: Option<String>,
    #[serde(default)]
    pub models: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_runtime: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub served_model_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelRuntimesConfig {
    pub runtimes: Vec<ModelRuntimeConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelRuntimeConfig {
    pub id: String,
    pub kind: ModelRuntimeKind,
    pub workspace: String,
    pub wsl_distro: String,
    pub endpoint: String,
    pub models_endpoint: String,
    pub log_dir: String,
    #[serde(default)]
    pub variants: Vec<ModelRuntimeVariant>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelRuntimeKind {
    Vllm,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelRuntimeVariant {
    pub id: String,
    pub served_model_name: String,
    pub mode: String,
    pub max_model_len: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub speculative_method: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub num_speculative_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cpu_offload_gb: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cuda_visible_devices: Option<String>,
    #[serde(default)]
    pub requires_explicit_confirm: bool,
    pub start: ModelRuntimeStart,
    pub stop: ModelRuntimeStop,
    #[serde(default)]
    pub profiles: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelRuntimeStart {
    pub kind: ModelRuntimeStartKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub script: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelRuntimeStartKind {
    WslScript,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelRuntimeStop {
    pub kind: ModelRuntimeStopKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub served_model_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelRuntimeStopKind {
    ProcessMatch,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    Ok,
    Degraded,
    Down,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EndpointStatus {
    pub url: String,
    pub reachable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_code: Option<u16>,
    pub message: String,
}

impl EndpointStatus {
    pub fn ok(url: impl Into<String>, status_code: u16) -> Self {
        Self {
            url: url.into(),
            reachable: true,
            status_code: Some(status_code),
            message: "ok".to_owned(),
        }
    }

    pub fn unavailable(url: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            reachable: false,
            status_code: None,
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WslDistroStatus {
    pub name: String,
    pub state: String,
    pub version: Option<u8>,
    pub default: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelRuntimeSummary {
    pub runtime_id: String,
    pub variant_id: String,
    pub served_model_name: String,
    pub endpoint: EndpointStatus,
    pub ready: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateSummary {
    pub state_db_exists: bool,
    pub audit_db_exists: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadOnlyStatus {
    pub wsl: Option<WslDistroStatus>,
    pub hermes: EndpointStatus,
    pub models: Vec<ModelRuntimeSummary>,
    pub state: StateSummary,
    pub overall: HealthStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActiveRouteStatus {
    pub active_profile_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_known_good_profile_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditEventSummary {
    pub id: i64,
    pub happened_at: String,
    pub requester_channel: String,
    pub requester_user_id: String,
    pub action: String,
    pub risk_level: String,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandPreview {
    pub program: String,
    pub args: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperationResponse {
    pub status: String,
    pub risk: RiskLevel,
    pub summary: String,
    pub dry_run: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub commands: Vec<CommandPreview>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confirmation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_hint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfirmationLifecycleResponse {
    pub status: String,
    pub confirmation_id: String,
    pub summary: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_status: Option<String>,
}
