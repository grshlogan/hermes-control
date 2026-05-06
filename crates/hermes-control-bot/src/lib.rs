use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use hermes_control_types::{
    ActionRequest, CancelRequest, ConfirmRequest, HermesAction, ModelAction, Requester,
    RouteRollbackRequest, RouteSwitchRequest, WslAction,
};
use rusqlite::{Connection, OptionalExtension};
use serde_json::Value;
use teloxide::prelude::*;
use teloxide::requests::{HasPayload, Request, Requester as TeloxideRequester};
use teloxide::types::{BotCommand, Message, Update, UpdateKind};
use teloxide::utils::command::BotCommands;
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
    #[error("bot state I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("bot state SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
}

#[derive(Debug, Clone)]
pub struct BotConfig {
    telegram_token: String,
    daemon_base_url: Url,
    daemon_api_token: String,
    allowed_users: BTreeSet<String>,
    allowed_chats: BTreeSet<String>,
    bot_id: String,
    state_db: PathBuf,
    log_dir: PathBuf,
    poll_timeout_seconds: u32,
    poll_error_retry_seconds: u64,
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
        let bot_id = env
            .get("HERMES_CONTROL_BOT_ID")
            .cloned()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "primary".to_owned());
        let state_db = env
            .get("HERMES_CONTROL_BOT_STATE_DB")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("state/bot.sqlite"));
        let log_dir = env
            .get("HERMES_CONTROL_BOT_LOG_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("logs/bot"));
        let poll_timeout_seconds = env
            .get("HERMES_CONTROL_BOT_POLL_TIMEOUT_SECONDS")
            .and_then(|value| value.parse::<u32>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(30);
        let poll_error_retry_seconds = env
            .get("HERMES_CONTROL_BOT_POLL_ERROR_RETRY_SECONDS")
            .and_then(|value| value.parse::<u64>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(5);
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
            bot_id,
            state_db,
            log_dir,
            poll_timeout_seconds,
            poll_error_retry_seconds,
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

    pub fn bot_id(&self) -> &str {
        &self.bot_id
    }

    pub fn state_db(&self) -> &Path {
        &self.state_db
    }

    pub fn log_dir(&self) -> &Path {
        &self.log_dir
    }

    pub fn poll_timeout_seconds(&self) -> u32 {
        self.poll_timeout_seconds
    }

    pub fn poll_error_retry_seconds(&self) -> u64 {
        self.poll_error_retry_seconds
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
    bot_id: Option<String>,
    state_db: Option<PathBuf>,
    log_dir: Option<PathBuf>,
    poll_timeout_seconds: Option<u32>,
    poll_error_retry_seconds: Option<u64>,
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

    pub fn bot_id(mut self, value: impl Into<String>) -> Self {
        self.bot_id = Some(value.into());
        self
    }

    pub fn state_db(mut self, value: impl Into<PathBuf>) -> Self {
        self.state_db = Some(value.into());
        self
    }

    pub fn log_dir(mut self, value: impl Into<PathBuf>) -> Self {
        self.log_dir = Some(value.into());
        self
    }

    pub fn poll_timeout_seconds(mut self, value: u32) -> Self {
        self.poll_timeout_seconds = Some(value);
        self
    }

    pub fn poll_error_retry_seconds(mut self, value: u64) -> Self {
        self.poll_error_retry_seconds = Some(value);
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
            bot_id: self.bot_id.unwrap_or_else(|| "primary".to_owned()),
            state_db: self
                .state_db
                .unwrap_or_else(|| PathBuf::from("state/bot.sqlite")),
            log_dir: self.log_dir.unwrap_or_else(|| PathBuf::from("logs/bot")),
            poll_timeout_seconds: self.poll_timeout_seconds.unwrap_or(30),
            poll_error_retry_seconds: self.poll_error_retry_seconds.unwrap_or(5),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, BotCommands)]
#[command(rename_rule = "lowercase")]
pub enum HermesBotCommand {
    /// Show command help.
    #[command(aliases = ["start"])]
    Help,
    /// Show full local-stack status.
    Status,
    /// Show daemon health.
    Health,
    /// List provider profiles.
    Providers,
    /// Show active AI route.
    Route,
    /// Switch active AI route.
    Switch(String),
    /// Roll back to last-known-good route.
    Rollback,
    /// List model runtimes.
    Models,
    /// Run a model command.
    Model(String),
    /// Run a Hermes command.
    Hermes(String),
    /// Run a WSL command.
    Wsl(String),
    /// Tail logs.
    Logs(String),
    /// Show audit events.
    Audit(String),
    /// Confirm a pending operation.
    Confirm(String),
    /// Cancel a pending operation.
    Cancel,
}

pub fn telegram_command_menu() -> Vec<BotCommand> {
    HermesBotCommand::bot_commands()
        .into_iter()
        .map(|mut command| {
            command.command = command.command.trim_start_matches('/').to_owned();
            command
        })
        .collect()
}

#[derive(Debug, Clone)]
pub struct BotStateStore {
    state_db: Arc<PathBuf>,
    bot_id: Arc<str>,
}

impl BotStateStore {
    pub fn initialize(
        state_db: impl AsRef<Path>,
        bot_id: impl Into<String>,
    ) -> Result<Self, BotError> {
        let state_db = state_db.as_ref().to_path_buf();
        if let Some(parent) = state_db
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            fs::create_dir_all(parent)?;
        }

        let connection = Connection::open(&state_db)?;
        connection.execute_batch(
            "
            PRAGMA journal_mode = WAL;
            CREATE TABLE IF NOT EXISTS telegram_state (
                bot_id TEXT PRIMARY KEY,
                update_offset INTEGER NOT NULL,
                updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            ",
        )?;

        Ok(Self {
            state_db: Arc::new(state_db),
            bot_id: Arc::from(bot_id.into()),
        })
    }

    pub fn read_next_offset(&self) -> Result<Option<i32>, BotError> {
        let connection = Connection::open(&*self.state_db)?;
        let offset = connection
            .query_row(
                "SELECT update_offset FROM telegram_state WHERE bot_id = ?1",
                [&*self.bot_id],
                |row| row.get::<_, i32>(0),
            )
            .optional()?;
        Ok(offset)
    }

    pub fn write_next_offset(&self, offset: i32) -> Result<(), BotError> {
        let connection = Connection::open(&*self.state_db)?;
        connection.execute(
            "
            INSERT INTO telegram_state (bot_id, update_offset, updated_at)
            VALUES (?1, ?2, CURRENT_TIMESTAMP)
            ON CONFLICT(bot_id) DO UPDATE SET
                update_offset = excluded.update_offset,
                updated_at = CURRENT_TIMESTAMP
            ",
            (&*self.bot_id, offset),
        )?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct BotEventLog {
    log_file: Arc<PathBuf>,
}

impl BotEventLog {
    pub fn initialize(log_dir: impl AsRef<Path>) -> Result<Self, BotError> {
        let log_dir = log_dir.as_ref();
        fs::create_dir_all(log_dir)?;
        Ok(Self {
            log_file: Arc::new(log_dir.join("bot.log")),
        })
    }

    pub fn append(&self, message: &str) -> Result<(), BotError> {
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&*self.log_file)?;
        writeln!(
            file,
            "{} {}",
            unix_epoch_seconds(),
            redact_log_line(message)
        )?;
        Ok(())
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

impl BotDecision {
    fn path(&self) -> &str {
        match self {
            Self::Reply(_) => "",
            Self::Daemon { path, .. } => path,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum AdminCommand {
    Help,
    Status,
    Health,
    Providers,
    Route,
    Switch { profile_id: String },
    Rollback,
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
        AdminCommand::Rollback => post_json(
            "/v1/route/rollback",
            RouteRollbackRequest {
                requester,
                reason: "telegram /rollback".to_owned(),
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

    let normalized = normalize_command_text(trimmed);
    let command = HermesBotCommand::parse(&normalized, "")
        .map_err(|err| BotError::InvalidCommand(err.to_string()))?;
    admin_command_from_bot_command(command)
}

fn admin_command_from_bot_command(command: HermesBotCommand) -> Result<AdminCommand, BotError> {
    match command {
        HermesBotCommand::Help => Ok(AdminCommand::Help),
        HermesBotCommand::Status => Ok(AdminCommand::Status),
        HermesBotCommand::Health => Ok(AdminCommand::Health),
        HermesBotCommand::Providers => Ok(AdminCommand::Providers),
        HermesBotCommand::Route => Ok(AdminCommand::Route),
        HermesBotCommand::Switch(args) => Ok(AdminCommand::Switch {
            profile_id: single_arg(&args, "/switch <profile-id>")?.to_owned(),
        }),
        HermesBotCommand::Rollback => Ok(AdminCommand::Rollback),
        HermesBotCommand::Models => Ok(AdminCommand::Models),
        HermesBotCommand::Model(args) => {
            let parts = split_args(&args);
            Ok(AdminCommand::Model {
                action: required_arg(
                    &parts,
                    0,
                    "/model <status|install|start|stop|restart|logs|benchmark> <model-id>",
                )?
                .to_owned(),
                model_id: required_arg(
                    &parts,
                    1,
                    "/model <status|install|start|stop|restart|logs|benchmark> <model-id>",
                )?
                .to_owned(),
            })
        }
        HermesBotCommand::Hermes(args) => Ok(AdminCommand::Hermes {
            action: single_arg(&args, "/hermes <wake|stop|restart|kill|status>")?.to_owned(),
        }),
        HermesBotCommand::Wsl(args) => Ok(AdminCommand::Wsl {
            action: single_arg(&args, "/wsl <status|wake|stop|restart>")?.to_owned(),
        }),
        HermesBotCommand::Logs(args) => {
            let parts = split_args(&args);
            Ok(AdminCommand::Logs {
                target: required_arg(&parts, 0, "/logs <hermes|daemon|bot|model> [id]")?.to_owned(),
                id: parts.get(1).map(|value| (*value).to_owned()),
            })
        }
        HermesBotCommand::Audit(args) => Ok(AdminCommand::Audit {
            limit: args.trim().parse::<usize>().unwrap_or(100),
        }),
        HermesBotCommand::Confirm(code) => Ok(AdminCommand::Confirm {
            code: single_arg(&code, "/confirm <code>")?.to_owned(),
        }),
        HermesBotCommand::Cancel => Ok(AdminCommand::Cancel),
    }
}

fn normalize_command_text(text: &str) -> String {
    let mut parts = text.splitn(2, char::is_whitespace);
    let command = parts.next().unwrap_or_default();
    let args = parts.next();
    let command = command
        .split('@')
        .next()
        .filter(|value| !value.is_empty())
        .unwrap_or(command);

    match (command.eq_ignore_ascii_case("/audit"), args) {
        (true, None) => "/audit 100".to_owned(),
        _ => args.map_or_else(|| command.to_owned(), |args| format!("{command} {args}")),
    }
}

fn split_args(value: &str) -> Vec<&str> {
    value.split_whitespace().collect()
}

fn single_arg<'a>(value: &'a str, usage: &str) -> Result<&'a str, BotError> {
    value
        .split_whitespace()
        .next()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| BotError::InvalidCommand(format!("usage: {usage}")))
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

fn unix_epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

fn redact_log_line(value: &str) -> String {
    let tokens = value.split_whitespace().collect::<Vec<_>>();
    let mut redacted = Vec::with_capacity(tokens.len());
    let mut index = 0;
    while index < tokens.len() {
        let token = tokens[index];
        if token.eq_ignore_ascii_case("Bearer") {
            redacted.push("Bearer".to_owned());
            if index + 1 < tokens.len() {
                redacted.push("<redacted>".to_owned());
                index += 2;
                continue;
            }
        } else {
            redacted.push(redact_log_token(token));
        }
        index += 1;
    }
    redacted.join(" ")
}

fn redact_log_token(token: &str) -> String {
    let upper = token.to_ascii_uppercase();
    if upper.starts_with("TELOXIDE_TOKEN=")
        || upper.starts_with("HERMES_CONTROL_TELEGRAM_TOKEN=")
        || upper.starts_with("HERMES_CONTROL_API_TOKEN=")
        || upper.starts_with("AUTHORIZATION=")
    {
        return token
            .split_once('=')
            .map(|(key, _)| format!("{key}=<redacted>"))
            .unwrap_or_else(|| "<redacted>".to_owned());
    }

    token.to_owned()
}

fn help_text() -> String {
    [
        "Hermes control commands:",
        "/status",
        "/health",
        "/providers",
        "/route",
        "/switch <profile-id>",
        "/rollback",
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
    let state = BotStateStore::initialize(config.state_db(), config.bot_id())?;
    let event_log = BotEventLog::initialize(config.log_dir())?;
    let daemon = DaemonClient::from_config(&config);
    let mut offset = state.read_next_offset()?;
    event_log.append(&format!(
        "bot started bot_id={} state_db={} offset={}",
        config.bot_id(),
        config.state_db().display(),
        offset
            .map(|value| value.to_string())
            .unwrap_or_else(|| "none".to_owned())
    ))?;
    publish_command_menu(&bot, &event_log).await;

    loop {
        let updates = match bot
            .get_updates()
            .with_payload_mut(|payload| {
                payload.offset = offset;
                payload.timeout = Some(config.poll_timeout_seconds());
                payload.limit = Some(50);
            })
            .send()
            .await
        {
            Ok(updates) => updates,
            Err(err) => {
                tracing::warn!(error = %err, "Telegram polling failed; retrying");
                let _ = event_log.append(&format!("telegram polling failed: {err}"));
                tokio::time::sleep(Duration::from_secs(config.poll_error_retry_seconds())).await;
                continue;
            }
        };

        for update in updates {
            let next_offset = update.id.as_offset();
            state.write_next_offset(next_offset)?;
            offset = Some(next_offset);

            if let Some(message) = message_from_update(update)
                && let Err(err) = answer_message(&bot, &daemon, &config, &event_log, message).await
            {
                tracing::warn!(error = %err, "Telegram message handling failed; continuing");
                let _ = event_log.append(&format!("telegram message handling failed: {err}"));
            }
        }
    }
}

async fn publish_command_menu(bot: &Bot, event_log: &BotEventLog) {
    match bot.set_my_commands(telegram_command_menu()).send().await {
        Ok(_) => {
            let _ = event_log.append("telegram command menu published");
        }
        Err(err) => {
            tracing::warn!(error = %err, "failed to publish Telegram command menu");
            let _ = event_log.append(&format!("telegram command menu publish failed: {err}"));
        }
    }
}

fn message_from_update(update: Update) -> Option<Message> {
    match update.kind {
        UpdateKind::Message(message) | UpdateKind::EditedMessage(message) => Some(message),
        _ => None,
    }
}

async fn answer_message(
    bot: &Bot,
    daemon: &DaemonClient,
    config: &BotConfig,
    event_log: &BotEventLog,
    msg: Message,
) -> ResponseResult<()> {
    if let Some(text) = msg.text() {
        let user_id = msg
            .from
            .as_ref()
            .map(|user| user.id.0.to_string())
            .unwrap_or_else(|| "unknown".to_owned());
        let chat_id = msg.chat.id.to_string();
        let reply = match plan_message(text, &user_id, &chat_id, config) {
            Ok(BotDecision::Reply(reply)) => reply,
            Ok(decision @ BotDecision::Daemon { .. }) => {
                let _ =
                    event_log.append(&format!("daemon request planned path={}", decision.path()));
                daemon.send(&decision).await.unwrap_or_else(|err| {
                    let _ = event_log.append(&format!("daemon request failed: {err}"));
                    format!("Daemon request failed: {err}")
                })
            }
            Err(err) => {
                let _ = event_log.append(&format!("invalid bot command: {err}"));
                err.to_string()
            }
        };

        bot.send_message(msg.chat.id, reply).await?;
    }
    Ok(())
}
