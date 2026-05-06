use hermes_control_bot::{
    BotConfig, BotDecision, BotEventLog, BotStateStore, HermesBotCommand, HttpMethod, plan_message,
    telegram_command_menu,
};
use serde_json::json;
use teloxide::utils::command::BotCommands;

fn test_config() -> BotConfig {
    BotConfig::builder_for_tests()
        .telegram_token("telegram-token")
        .daemon_base_url("http://127.0.0.1:18787")
        .daemon_api_token("daemon-token")
        .allowed_users(["123"])
        .allowed_chats(["chat-a"])
        .build()
        .expect("test config should be valid")
}

#[test]
fn rejects_disallowed_user_before_daemon_call() {
    let decision = plan_message("/status", "999", "chat-a", &test_config()).unwrap();

    assert_eq!(
        decision,
        BotDecision::Reply("You are not allowed to use Hermes admin controls.".to_owned())
    );
}

#[test]
fn status_is_a_read_only_daemon_status_call() {
    let decision = plan_message("/status", "123", "chat-a", &test_config()).unwrap();

    assert_eq!(
        decision,
        BotDecision::Daemon {
            method: HttpMethod::Get,
            path: "/v1/status".to_owned(),
            body: None,
        }
    );
}

#[test]
fn start_alias_returns_help_without_daemon_call() {
    let decision = plan_message("/start", "123", "chat-a", &test_config()).unwrap();

    let BotDecision::Reply(reply) = decision else {
        panic!("/start should render local help");
    };
    assert!(reply.contains("/status"));
    assert!(reply.contains("/confirm <code>"));
}

#[test]
fn telegram_mention_is_accepted_for_commands() {
    let decision = plan_message(
        "/switch@HermesControlBot local.vllm",
        "123",
        "chat-a",
        &test_config(),
    )
    .unwrap();

    assert_eq!(
        decision,
        BotDecision::Daemon {
            method: HttpMethod::Post,
            path: "/v1/route/switch".to_owned(),
            body: Some(json!({
                "requester": {
                    "channel": "telegram",
                    "user_id": "123",
                    "chat_id": "chat-a"
                },
                "reason": "telegram /switch local.vllm",
                "profile_id": "local.vllm",
                "dry_run": false
            })),
        }
    );
}

#[test]
fn audit_without_limit_uses_default_limit() {
    let decision = plan_message("/audit", "123", "chat-a", &test_config()).unwrap();

    assert_eq!(
        decision,
        BotDecision::Daemon {
            method: HttpMethod::Get,
            path: "/v1/audit?limit=100".to_owned(),
            body: None,
        }
    );
}

#[test]
fn wsl_restart_posts_typed_action_to_daemon() {
    let decision = plan_message("/wsl restart", "123", "chat-a", &test_config()).unwrap();

    assert_eq!(
        decision,
        BotDecision::Daemon {
            method: HttpMethod::Post,
            path: "/v1/wsl/action".to_owned(),
            body: Some(json!({
                "requester": {
                    "channel": "telegram",
                    "user_id": "123",
                    "chat_id": "chat-a"
                },
                "action": "RestartDistro",
                "reason": "telegram /wsl restart",
                "dry_run": false
            })),
        }
    );
}

#[test]
fn model_start_posts_typed_model_action_to_daemon() {
    let decision =
        plan_message("/model start qwen36-mtp", "123", "chat-a", &test_config()).unwrap();

    assert_eq!(
        decision,
        BotDecision::Daemon {
            method: HttpMethod::Post,
            path: "/v1/models/qwen36-mtp/action".to_owned(),
            body: Some(json!({
                "requester": {
                    "channel": "telegram",
                    "user_id": "123",
                    "chat_id": "chat-a"
                },
                "action": "Start",
                "reason": "telegram /model start qwen36-mtp",
                "dry_run": false
            })),
        }
    );
}

#[test]
fn model_install_posts_typed_model_action_to_daemon() {
    let decision =
        plan_message("/model install qwen36-mtp", "123", "chat-a", &test_config()).unwrap();

    assert_eq!(
        decision,
        BotDecision::Daemon {
            method: HttpMethod::Post,
            path: "/v1/models/qwen36-mtp/action".to_owned(),
            body: Some(json!({
                "requester": {
                    "channel": "telegram",
                    "user_id": "123",
                    "chat_id": "chat-a"
                },
                "action": "Install",
                "reason": "telegram /model install qwen36-mtp",
                "dry_run": false
            })),
        }
    );
}

#[test]
fn route_rollback_posts_typed_request_to_daemon() {
    let decision = plan_message("/rollback", "123", "chat-a", &test_config()).unwrap();

    assert_eq!(
        decision,
        BotDecision::Daemon {
            method: HttpMethod::Post,
            path: "/v1/route/rollback".to_owned(),
            body: Some(json!({
                "requester": {
                    "channel": "telegram",
                    "user_id": "123",
                    "chat_id": "chat-a"
                },
                "reason": "telegram /rollback",
                "dry_run": false
            })),
        }
    );
}

#[test]
fn confirm_forwards_code_to_daemon_confirmation_endpoint() {
    let decision = plan_message("/confirm HERMES-7421", "123", "chat-a", &test_config()).unwrap();

    assert_eq!(
        decision,
        BotDecision::Daemon {
            method: HttpMethod::Post,
            path: "/v1/confirm".to_owned(),
            body: Some(json!({
                "requester": {
                    "channel": "telegram",
                    "user_id": "123",
                    "chat_id": "chat-a"
                },
                "code": "HERMES-7421"
            })),
        }
    );
}

#[test]
fn teloxide_command_enum_parses_model_command_arguments() {
    let command = HermesBotCommand::parse("/model start qwen36-mtp", "")
        .expect("teloxide command enum should parse model args");

    assert_eq!(
        command,
        HermesBotCommand::Model("start qwen36-mtp".to_owned())
    );
}

#[test]
fn bot_state_store_persists_next_offset_across_restarts() {
    let db_path = std::env::temp_dir().join(format!(
        "hermes-control-bot-state-{}.sqlite",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&db_path);

    let store =
        BotStateStore::initialize(&db_path, "primary").expect("state store should initialize");
    assert_eq!(store.read_next_offset().unwrap(), None);

    store.write_next_offset(7422).unwrap();

    let reopened =
        BotStateStore::initialize(&db_path, "primary").expect("state store should reopen");
    assert_eq!(reopened.read_next_offset().unwrap(), Some(7422));

    let _ = std::fs::remove_file(&db_path);
}

#[test]
fn telegram_command_menu_is_derived_from_teloxide_command_enum() {
    let commands = telegram_command_menu();
    let names = commands
        .iter()
        .map(|command| command.command.as_str())
        .collect::<Vec<_>>();

    assert!(names.contains(&"status"));
    assert!(names.contains(&"rollback"));
    assert!(names.contains(&"confirm"));
    assert!(names.contains(&"logs"));
    assert!(!names.contains(&"start"));
}

#[test]
fn bot_event_log_appends_redacted_lines() {
    let log_dir =
        std::env::temp_dir().join(format!("hermes-control-bot-logs-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&log_dir);

    let log = BotEventLog::initialize(&log_dir).expect("log should initialize");
    log.append("bot started with TELOXIDE_TOKEN=secret and Bearer abc")
        .expect("log should append");

    let content = std::fs::read_to_string(log_dir.join("bot.log")).expect("log should read");
    assert!(content.contains("bot started"));
    assert!(content.contains("TELOXIDE_TOKEN=<redacted>"));
    assert!(content.contains("Bearer <redacted>"));
    assert!(!content.contains("secret"));
    assert!(!content.contains("Bearer abc"));

    let _ = std::fs::remove_dir_all(&log_dir);
}
