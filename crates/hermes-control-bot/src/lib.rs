use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use hermes_control_types::{
    ActionRequest, CancelRequest, ConfirmRequest, HermesAction, ModelAction, Requester,
    RouteSwitchRequest, WslAction,
};
use serde_json::Value;
use teloxide::prelude::*;
use teloxide::requests::Requester as TeloxideRequester;
use thiserror::Error;
use url::Url;

const NOT_ALLOWED: &str = "You are not allowed to use Hermes admin controls.";

#[derive(Debug, Error)]
pub enum BotError {
    #[error("missing required environment variable {0}")]
    MissingEnv(&'static str),
    #[error("invalid daemon url: {0}")]
    InvalidDaemonUrl(#[from] url::ParseError),
    #[error("invalid command: {0}")]
    InvalidCommand(String),
    #[error("failed to serialize daemon request: {0}")]
    Serialize(#[from] serde_json::Error),
    #[error("daemon request failed: {0}")]
    Http(#[from] reqwest::Error),
}

#[derive(Debug, Clone)]
pub struct BotConfig {
    telegram_token: String,
    daemon_base_url: Url,
    daemon_api_token: String,
    allowed_users: BTreeSet<String>,
    allowed_chats: BTreeSet<String>,
}

impl BotConfig {
    pub fn builder_for_tests() -> BotConfigBuilder {
        BotConfigBuilder::default()
    }

    pub fn from_env() -> Result<Self, BotError> {
        Self::from_env_iter(std::env::vars())
    }

    pub fn from_env_iter<I, K, V>(vars: I) -> Result<Self, BotError>
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        let env = vars
            .into_iter()
            .map(|(key, value)| (key.into(), value.into()))
            .collect::<BTreeMap<_, _>>();

        let telegram_token = env
            .get("HERMES_CONTROL_TELEGRAM_TOKEN")
            .or_else(|| env.get("TELOXIDE_TOKEN"))
            .cloned()
            .filter(|value| !value.trim().is_empty())
            .ok_or(BotError::MissingEnv("TELOXIDE_TOKEN"))?;
        let daemon_api_token = env
            .get("HERMES_CONTROL_API_TOKEN")
            .cloned()
            .filter(|value| !value.trim().is_empty())
            .ok_or(BotError::MissingEnv("HERMES_CONTROL_API_TOKEN"))?;
        let allowed_users = env
            .get("HERMES_CONTROL_TELEGRAM_ALLOWED_USERS")
            .or_else(|| env.get("HERMES_ADMIN_ALLOWED_USERS"))
            .map(|value| parse_csv_set(value))
            .filter(|value| !value.is_empty())
            .ok_or(BotError::MissingEnv(
                "HERMES_CONTROL_TELEGRAM_ALLOWED_USERS",
            ))?;
        let allowed_chats = env
            .get("HERMES_CONTROL_TELEGRAM_ALLOWED_CHATS")
            .map(|value| parse_csv_set(value))
            .unwrap_or_default();
        let daemon_base_url = env
            .get("HERMES_CONTROL_DAEMON_URL")
            .map(String::as_str)
            .unwrap_or("http://127.0.0.1:18787")
            .parse()?;

        Ok(Self {
            telegram_token,
            daemon_base_url,
            daemon_api_token,
            allowed_users,
            allowed_chats,
        })
    }

    pub fn telegram_token(&self) -> &str {
        &self.telegram_token
    }

    pub fn daemon_base_url(&self) -> &Url {
        &self.daemon_base_url
    }

    pub fn daemon_api_token(&self) -> &str {
        &self.daemon_api_token
    }

    fn is_allowed(&self, user_id: &str, chat_id: &str) -> bool {
        let user_allowed = self.allowed_users.contains("*") || self.allowed_users.contains(user_id);
        let chat_allowed = self.allowed_chats.is_empty()
            || self.allowed_chats.contains("*")
            || self.allowed_chats.contains(chat_id);
        user_allowed && chat_allowed
    }
}

#[derive(Debug, Default)]
pub struct BotConfigBuilder {
    telegram_token: Option<String>,
    daemon_base_url: Option<String>,
    daemon_api_token: Option<String>,
    allowed_users: BTreeSet<String>,
    allowed_chats: BTreeSet<String>,
}

impl BotConfigBuilder {
    pub fn telegram_token(mut self, value: impl Into<String>) -> Self {
        self.telegram_token = Some(value.into());
        self
    }

    pub fn daemon_base_url(mut self, value: impl Into<String>) -> Self {
        self.daemon_base_url = Some(value.into());
        self
    }

    pub fn daemon_api_token(mut self, value: impl Into<String>) -> Self {
        self.daemon_api_token = Some(value.into());
        self
    }

    pub fn allowed_users<I, S>(mut self, values: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.allowed_users = values.into_iter().map(Into::into).collect();
        self
    }

    pub fn allowed_chats<I, S>(mut self, values: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.allowed_chats = values.into_iter().map(Into::into).collect();
        self
    }

    pub fn build(self) -> Result<BotConfig, BotError> {
        Ok(BotConfig {
            telegram_token: self
                .telegram_token
                .ok_or(BotError::MissingEnv("TELOXIDE_TOKEN"))?,
            daemon_base_url: self
                .daemon_base_url
                .unwrap_or_else(|| "http://127.0.0.1:18787".to_owned())
                .parse()?,
            daemon_api_token: self
                .daemon_api_token
                .ok_or(BotError::MissingEnv("HERMES_CONTROL_API_TOKEN"))?,
            allowed_users: self.allowed_users,
            allowed_chats: self.allowed_chats,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpMethod {
    Get,
    Post,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BotDecision {
    Reply(String),
    Daemon {
        method: HttpMethod,
        path: String,
        body: Option<Value>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum AdminCommand {
    Help,
    Status,
    Health,
    Providers,
    Route,
    Switch { profile_id: String },
    Models,
    Model { action: String, model_id: String },
    Hermes { action: String },
    Wsl { action: String },
    Logs { target: String, id: Option<String> },
    Audit { limit: usize },
    Confirm { code: String },
    Cancel,
}

pub fn plan_message(
    text: &str,
    user_id: &str,
    chat_id: &str,
    config: &BotConfig,
) -> Result<BotDecision, BotError> {
    if !config.is_allowed(user_id, chat_id) {
        return Ok(BotDecision::Reply(NOT_ALLOWED.to_owned()));
    }

    let command = parse_command(text)?;
    let requester = Requester::telegram(user_id, chat_id);
    plan_command(command, requester)
}

fn plan_command(command: AdminCommand, requester: Requester) -> Result<BotDecision, BotError> {
    match command {
        AdminCommand::Help => Ok(BotDecision::Reply(help_text())),
        AdminCommand::Status => Ok(get("/v1/status")),
        AdminCommand::Health => Ok(get("/v1/health")),
        AdminCommand::Providers => Ok(get("/v1/providers")),
        AdminCommand::Route => Ok(get("/v1/route/active")),
        AdminCommand::Switch { profile_id } => post_json(
            "/v1/route/switch",
            RouteSwitchRequest {
                requester,
                reason: format!("telegram /switch {profile_id}"),
                profile_id,
                dry_run: false,
            },
        ),
        AdminCommand::Models => Ok(get("/v1/models")),
        AdminCommand::Model { action, model_id } => plan_model(action, model_id, requester),
        AdminCommand::Hermes { action } => plan_hermes(action, requester),
        AdminCommand::Wsl { action } => plan_wsl(action, requester),
        AdminCommand::Logs { target, id } => Ok(plan_logs(target, id)),
        AdminCommand::Audit { limit } => Ok(get(format!("/v1/audit?limit={limit}"))),
        AdminCommand::Confirm { code } => {
            post_json("/v1/confirm", ConfirmRequest { requester, code })
        }
        AdminCommand::Cancel => post_json("/v1/cancel", CancelRequest { requester }),
    }
}

fn plan_model(
    action: String,
    model_id: String,
    requester: Requester,
) -> Result<BotDecision, BotError> {
    let lower = action.to_ascii_lowercase();
    match lower.as_str() {
        "status" => Ok(get(format!("/v1/models/{model_id}"))),
        "logs" => Ok(get(format!("/v1/models/{model_id}/logs?tail=200"))),
        "install" => model_action(model_id, requester, ModelAction::Install, "install"),
        "start" => model_action(model_id, requester, ModelAction::Start, "start"),
        "stop" => model_action(model_id, requester, ModelAction::Stop, "stop"),
        "restart" => model_action(model_id, requester, ModelAction::Restart, "restart"),
        "health" => model_action(model_id, requester, ModelAction::Health, "health"),
        "benchmark" => model_action(model_id, requester, ModelAction::Benchmark, "benchmark"),
        _ => Err(BotError::InvalidCommand(format!(
            "unknown model action {action}"
        ))),
    }
}

fn model_action(
    model_id: String,
    requester: Requester,
    action: ModelAction,
    raw_action: &str,
) -> Result<BotDecision, BotError> {
    post_json(
        format!("/v1/models/{model_id}/action"),
        ActionRequest {
            requester,
            action,
            reason: format!("telegram /model {raw_action} {model_id}"),
            dry_run: false,
        },
    )
}

fn plan_hermes(action: String, requester: Requester) -> Result<BotDecision, BotError> {
    let lower = action.to_ascii_lowercase();
    match lower.as_str() {
        "status" => Ok(get("/v1/hermes/status")),
        "wake" => hermes_action(requester, HermesAction::Wake, "wake"),
        "stop" => hermes_action(requester, HermesAction::Stop, "stop"),
        "restart" => hermes_action(requester, HermesAction::Restart, "restart"),
        "kill" => hermes_action(requester, HermesAction::Kill, "kill"),
        _ => Err(BotError::InvalidCommand(format!(
            "unknown hermes action {action}"
        ))),
    }
}

fn hermes_action(
    requester: Requester,
    action: HermesAction,
    raw_action: &str,
) -> Result<BotDecision, BotError> {
    post_json(
        "/v1/hermes/action",
        ActionRequest {
            requester,
            action,
            reason: format!("telegram /hermes {raw_action}"),
            dry_run: false,
        },
    )
}

fn plan_wsl(action: String, requester: Requester) -> Result<BotDecision, BotError> {
    let lower = action.to_ascii_lowercase();
    match lower.as_str() {
        "status" => Ok(get("/v1/wsl/status")),
        "wake" => wsl_action(requester, WslAction::Wake, "wake"),
        "stop" => wsl_action(requester, WslAction::StopDistro, "stop"),
        "restart" => wsl_action(requester, WslAction::RestartDistro, "restart"),
        "shutdown" | "shutdownall" => wsl_action(requester, WslAction::ShutdownAll, "shutdown"),
        _ => Err(BotError::InvalidCommand(format!(
            "unknown wsl action {action}"
        ))),
    }
}

fn wsl_action(
    requester: Requester,
    action: WslAction,
    raw_action: &str,
) -> Result<BotDecision, BotError> {
    post_json(
        "/v1/wsl/action",
        ActionRequest {
            requester,
            action,
            reason: format!("telegram /wsl {raw_action}"),
            dry_run: false,
        },
    )
}

fn plan_logs(target: String, id: Option<String>) -> BotDecision {
    if target.eq_ignore_ascii_case("model")
        && let Some(model_id) = id
    {
        return get(format!("/v1/models/{model_id}/logs?tail=200"));
    }
    get(format!("/v1/logs/{target}?tail=200"))
}

fn get(path: impl Into<String>) -> BotDecision {
    BotDecision::Daemon {
        method: HttpMethod::Get,
        path: path.into(),
        body: None,
    }
}

fn post_json(
    path: impl Into<String>,
    body: impl serde::Serialize,
) -> Result<BotDecision, BotError> {
    Ok(BotDecision::Daemon {
        method: HttpMethod::Post,
        path: path.into(),
        body: Some(serde_json::to_value(body)?),
    })
}

fn parse_command(text: &str) -> Result<AdminCommand, BotError> {
    let trimmed = text.trim();
    if trimmed.is_empty() || !trimmed.starts_with('/') {
        return Ok(AdminCommand::Help);
    }

    let parts = trimmed.split_whitespace().collect::<Vec<_>>();
    let command = parts[0]
        .trim_start_matches('/')
        .split('@')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();

    match command.as_str() {
        "help" | "start" => Ok(AdminCommand::Help),
        "status" => Ok(AdminCommand::Status),
        "health" => Ok(AdminCommand::Health),
        "providers" => Ok(AdminCommand::Providers),
        "route" => Ok(AdminCommand::Route),
        "switch" => Ok(AdminCommand::Switch {
            profile_id: required_arg(&parts, 1, "/switch <profile-id>")?.to_owned(),
        }),
        "models" => Ok(AdminCommand::Models),
        "model" => Ok(AdminCommand::Model {
            action: required_arg(
                &parts,
                1,
                "/model <status|install|start|stop|restart|logs|benchmark> <model-id>",
            )?
            .to_owned(),
            model_id: required_arg(
                &parts,
                2,
                "/model <status|install|start|stop|restart|logs|benchmark> <model-id>",
            )?
            .to_owned(),
        }),
        "hermes" => Ok(AdminCommand::Hermes {
            action: required_arg(&parts, 1, "/hermes <wake|stop|restart|kill|status>")?.to_owned(),
        }),
        "wsl" => Ok(AdminCommand::Wsl {
            action: required_arg(&parts, 1, "/wsl <status|wake|stop|restart>")?.to_owned(),
        }),
        "logs" => Ok(AdminCommand::Logs {
            target: required_arg(&parts, 1, "/logs <hermes|daemon|bot|model> [id]")?.to_owned(),
            id: parts.get(2).map(|value| (*value).to_owned()),
        }),
        "audit" => Ok(AdminCommand::Audit {
            limit: parts
                .get(1)
                .and_then(|value| value.parse::<usize>().ok())
                .unwrap_or(100),
        }),
        "confirm" => Ok(AdminCommand::Confirm {
            code: required_arg(&parts, 1, "/confirm <code>")?.to_owned(),
        }),
        "cancel" => Ok(AdminCommand::Cancel),
        other => Err(BotError::InvalidCommand(format!(
            "unknown command /{other}"
        ))),
    }
}

fn required_arg<'a>(parts: &'a [&str], index: usize, usage: &str) -> Result<&'a str, BotError> {
    parts
        .get(index)
        .copied()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| BotError::InvalidCommand(format!("usage: {usage}")))
}

fn parse_csv_set(value: &str) -> BTreeSet<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn help_text() -> String {
    [
        "Hermes control commands:",
        "/status",
        "/health",
        "/providers",
        "/route",
        "/switch <profile-id>",
        "/models",
        "/model <status|install|start|stop|restart|logs|benchmark> <model-id>",
        "/hermes <wake|stop|restart|kill|status>",
        "/wsl <status|wake|stop|restart>",
        "/logs <hermes|daemon|bot|model> [id]",
        "/audit [limit]",
        "/confirm <code>",
        "/cancel",
    ]
    .join("\n")
}

#[derive(Clone)]
pub struct DaemonClient {
    client: reqwest::Client,
    base_url: Url,
    api_token: String,
}

impl DaemonClient {
    pub fn from_config(config: &BotConfig) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: config.daemon_base_url.clone(),
            api_token: config.daemon_api_token.clone(),
        }
    }

    pub async fn send(&self, decision: &BotDecision) -> Result<String, BotError> {
        let BotDecision::Daemon { method, path, body } = decision else {
            return Ok(String::new());
        };

        let url = self.base_url.join(path)?;
        let request = match method {
            HttpMethod::Get => self.client.get(url),
            HttpMethod::Post => self.client.post(url),
        }
        .bearer_auth(&self.api_token);

        let request = if let Some(body) = body {
            request.json(body)
        } else {
            request
        };

        let response = request.send().await?.error_for_status()?;
        let value = response.json::<Value>().await?;
        Ok(format_daemon_response(&value))
    }
}

fn format_daemon_response(value: &Value) -> String {
    if value
        .get("status")
        .and_then(Value::as_str)
        .is_some_and(|status| status == "confirmation_required")
    {
        let summary = value
            .get("summary")
            .and_then(Value::as_str)
            .unwrap_or("Confirmation required.");
        let code = value
            .get("code_hint")
            .and_then(Value::as_str)
            .unwrap_or("<code>");
        return format!(
            "Confirmation required.\n{summary}\nReply with /confirm {code}, or /cancel."
        );
    }

    serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
}

pub async fn run_bot(config: BotConfig) -> anyhow::Result<()> {
    let bot = Bot::new(config.telegram_token.clone());
    let config = Arc::new(config);
    let daemon = Arc::new(DaemonClient::from_config(&config));

    teloxide::repl(bot, move |bot: Bot, msg: Message| {
        let config = Arc::clone(&config);
        let daemon = Arc::clone(&daemon);
        async move {
            if let Some(text) = msg.text() {
                let user_id = msg
                    .from
                    .as_ref()
                    .map(|user| user.id.0.to_string())
                    .unwrap_or_else(|| "unknown".to_owned());
                let chat_id = msg.chat.id.to_string();
                let reply = match plan_message(text, &user_id, &chat_id, &config) {
                    Ok(BotDecision::Reply(reply)) => reply,
                    Ok(decision @ BotDecision::Daemon { .. }) => daemon
                        .send(&decision)
                        .await
                        .unwrap_or_else(|err| format!("Daemon request failed: {err}")),
                    Err(err) => err.to_string(),
                };

                bot.send_message(msg.chat.id, reply).await?;
            }
            Ok(())
        }
    })
    .await;

    Ok(())
}
