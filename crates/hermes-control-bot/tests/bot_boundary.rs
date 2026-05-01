use hermes_control_bot::{BotConfig, BotDecision, HttpMethod, plan_message};
use serde_json::json;

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
