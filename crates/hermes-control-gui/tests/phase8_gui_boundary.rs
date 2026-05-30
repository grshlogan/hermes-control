use std::path::Path;

use hermes_control_gui::{
    GuiConfig, GuiDaemonCommand, GuiLogTarget, gui_boundary, gui_connection_summary_from_env_iter,
    gui_tauri_capability, hermes_action_request, log_tail_path, model_action_request,
    openwebui_action_request, operation_cancel_request, operation_confirm_request,
    route_rollback_request, route_switch_request, wsl_action_request,
};
use hermes_control_types::{
    ActionRequest, CancelRequest, ConfirmRequest, HermesAction, ModelAction, OpenWebUiAction,
    RequesterChannel, RouteRollbackRequest, RouteSwitchRequest, WslAction,
};

#[test]
fn gui_keeps_existing_thin_client_boundary() {
    let boundary = gui_boundary();

    assert_eq!(boundary.channel, RequesterChannel::Gui);
    assert!(!boundary.raw_process_execution);
}

#[test]
fn gui_config_reads_local_daemon_client_settings() {
    let config = GuiConfig::from_env_iter([
        ("HERMES_CONTROL_DAEMON_URL", "http://127.0.0.1:18787"),
        ("HERMES_CONTROL_API_TOKEN", "daemon-token"),
        ("HERMES_CONTROL_GUI_OPERATOR_ID", "desktop-operator"),
    ])
    .expect("GUI config should parse");

    assert_eq!(config.daemon_base_url().as_str(), "http://127.0.0.1:18787/");
    assert_eq!(config.api_token(), "daemon-token");
    assert_eq!(config.operator_id(), "desktop-operator");
}

#[test]
fn gui_connection_summary_redacts_token_for_renderer_settings() {
    let summary = gui_connection_summary_from_env_iter([
        ("HERMES_CONTROL_DAEMON_URL", "http://127.0.0.1:18787"),
        ("HERMES_CONTROL_API_TOKEN", "phase8-secret-token"),
        ("HERMES_CONTROL_GUI_OPERATOR_ID", "desktop-operator"),
    ])
    .expect("connection summary should parse");

    assert_eq!(summary.daemon_url, "http://127.0.0.1:18787/");
    assert_eq!(summary.operator_id, "desktop-operator");
    assert!(summary.token_configured);
    assert_eq!(summary.token_label, "****oken");
}

#[test]
fn gui_connection_summary_handles_missing_token_without_blocking_settings() {
    let summary = gui_connection_summary_from_env_iter([
        ("HERMES_CONTROL_DAEMON_URL", "http://127.0.0.1:18787"),
        ("HERMES_CONTROL_GUI_OPERATOR_ID", ""),
    ])
    .expect("settings summary should not require a token");

    assert_eq!(summary.daemon_url, "http://127.0.0.1:18787/");
    assert_eq!(summary.operator_id, "local-gui");
    assert!(!summary.token_configured);
    assert_eq!(summary.token_label, "not set");
}

#[test]
fn route_switch_request_uses_gui_requester_and_dry_run_default() {
    let request = route_switch_request("local.qwen36", "desktop-operator", true);

    assert_eq!(
        request,
        RouteSwitchRequest {
            requester: hermes_control_types::Requester {
                channel: RequesterChannel::Gui,
                user_id: "desktop-operator".to_owned(),
                chat_id: None,
            },
            profile_id: "local.qwen36".to_owned(),
            reason: "GUI route switch local.qwen36".to_owned(),
            dry_run: true,
        }
    );
}

#[test]
fn route_rollback_request_uses_gui_requester_and_dry_run_default() {
    let request = route_rollback_request("desktop-operator", true);

    assert_eq!(
        request,
        RouteRollbackRequest {
            requester: hermes_control_types::Requester {
                channel: RequesterChannel::Gui,
                user_id: "desktop-operator".to_owned(),
                chat_id: None,
            },
            reason: "GUI route rollback".to_owned(),
            dry_run: true,
        }
    );
}

#[test]
fn route_switch_execute_request_uses_gui_requester_without_dry_run() {
    let request = route_switch_request("external.deepseek", "desktop-operator", false);

    assert_eq!(request.requester.channel, RequesterChannel::Gui);
    assert_eq!(request.profile_id, "external.deepseek");
    assert_eq!(request.reason, "GUI route switch external.deepseek");
    assert!(!request.dry_run);
}

#[test]
fn confirmation_lifecycle_requests_use_gui_requester() {
    assert_eq!(
        operation_confirm_request("HERMES-7421", "desktop-operator"),
        ConfirmRequest {
            requester: hermes_control_types::Requester {
                channel: RequesterChannel::Gui,
                user_id: "desktop-operator".to_owned(),
                chat_id: None,
            },
            code: "HERMES-7421".to_owned(),
        }
    );
    assert_eq!(
        operation_cancel_request("desktop-operator"),
        CancelRequest {
            requester: hermes_control_types::Requester {
                channel: RequesterChannel::Gui,
                user_id: "desktop-operator".to_owned(),
                chat_id: None,
            },
        }
    );
}

#[test]
fn model_action_request_uses_gui_requester_and_action_reason() {
    assert_eq!(
        model_action_request(
            ModelAction::Restart,
            "qwen36-mtp",
            "desktop-operator",
            false
        ),
        ActionRequest {
            requester: hermes_control_types::Requester {
                channel: RequesterChannel::Gui,
                user_id: "desktop-operator".to_owned(),
                chat_id: None,
            },
            action: ModelAction::Restart,
            reason: "GUI model restart qwen36-mtp".to_owned(),
            dry_run: false,
        }
    );
}

#[test]
fn runtime_action_requests_use_gui_requester_and_action_reason() {
    assert_eq!(
        wsl_action_request(WslAction::RestartDistro, "desktop-operator", false),
        ActionRequest {
            requester: hermes_control_types::Requester {
                channel: RequesterChannel::Gui,
                user_id: "desktop-operator".to_owned(),
                chat_id: None,
            },
            action: WslAction::RestartDistro,
            reason: "GUI WSL restart distro".to_owned(),
            dry_run: false,
        }
    );
    assert_eq!(
        hermes_action_request(HermesAction::Kill, "desktop-operator", true),
        ActionRequest {
            requester: hermes_control_types::Requester {
                channel: RequesterChannel::Gui,
                user_id: "desktop-operator".to_owned(),
                chat_id: None,
            },
            action: HermesAction::Kill,
            reason: "GUI Hermes kill".to_owned(),
            dry_run: true,
        }
    );
    assert_eq!(
        openwebui_action_request(OpenWebUiAction::Restart, "desktop-operator", false),
        ActionRequest {
            requester: hermes_control_types::Requester {
                channel: RequesterChannel::Gui,
                user_id: "desktop-operator".to_owned(),
                chat_id: None,
            },
            action: OpenWebUiAction::Restart,
            reason: "GUI Open WebUI restart".to_owned(),
            dry_run: false,
        }
    );
}

#[test]
fn log_tail_path_accepts_only_gui_safe_log_targets() {
    assert_eq!(
        log_tail_path(GuiLogTarget::Daemon, 120).expect("daemon target should be valid"),
        "/v1/logs/daemon?tail=120"
    );
    assert_eq!(
        log_tail_path(GuiLogTarget::Bot, 0).expect("tail should clamp to minimum"),
        "/v1/logs/bot?tail=1"
    );
    assert_eq!(
        log_tail_path(GuiLogTarget::Hermes, 5000).expect("tail should clamp to maximum"),
        "/v1/logs/hermes?tail=1000"
    );
    assert_eq!(
        log_tail_path(GuiLogTarget::Vllm, 200).expect("vLLM target should be valid"),
        "/v1/logs/vllm?tail=200"
    );
    assert_eq!(
        GuiLogTarget::all()
            .iter()
            .map(|target| target.as_str())
            .collect::<Vec<_>>(),
        vec!["daemon", "bot", "hermes", "vllm"]
    );
}

#[test]
fn gui_daemon_commands_do_not_include_raw_process_or_filesystem_access() {
    let commands = GuiDaemonCommand::all();
    assert!(commands.contains(&GuiDaemonCommand::DashboardSnapshot));
    assert!(commands.contains(&GuiDaemonCommand::RouteSwitchPreview));
    assert!(commands.contains(&GuiDaemonCommand::RouteSwitchExecute));
    assert!(commands.contains(&GuiDaemonCommand::RouteRollbackExecute));
    assert!(commands.contains(&GuiDaemonCommand::ConfirmOperation));
    assert!(commands.contains(&GuiDaemonCommand::CancelOperation));
    assert!(commands.contains(&GuiDaemonCommand::ModelActionPreview));
    assert!(commands.contains(&GuiDaemonCommand::ModelActionExecute));
    assert!(commands.contains(&GuiDaemonCommand::WslActionPreview));
    assert!(commands.contains(&GuiDaemonCommand::WslActionExecute));
    assert!(commands.contains(&GuiDaemonCommand::HermesActionPreview));
    assert!(commands.contains(&GuiDaemonCommand::HermesActionExecute));
    assert!(commands.contains(&GuiDaemonCommand::OpenWebUiActionPreview));
    assert!(commands.contains(&GuiDaemonCommand::OpenWebUiActionExecute));
    assert!(commands.contains(&GuiDaemonCommand::LogsTail));

    for command in commands {
        assert!(!command.name().contains("shell"));
        assert!(!command.name().contains("process"));
        assert!(!command.name().contains("filesystem"));
    }
}

#[test]
fn default_tauri_capability_exposes_only_core_permissions() {
    let capability = gui_tauri_capability();

    assert_eq!(capability.identifier, "main");
    assert_eq!(capability.windows, vec!["main".to_owned()]);
    assert_eq!(capability.permissions, vec!["core:default".to_owned()]);
    assert!(
        capability
            .permissions
            .iter()
            .all(|permission| !permission.starts_with("shell:") && !permission.starts_with("fs:"))
    );
}

#[test]
fn tauri_capability_file_matches_safe_boundary_contract() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let capability_path = manifest_dir
        .join("..")
        .join("..")
        .join("apps")
        .join("hermes-control-gui")
        .join("src-tauri")
        .join("capabilities")
        .join("default.json");

    let content = std::fs::read_to_string(&capability_path).expect("capability file should exist");
    let value: serde_json::Value =
        serde_json::from_str(&content).expect("capability file should be valid JSON");
    let permissions = value
        .get("permissions")
        .and_then(serde_json::Value::as_array)
        .expect("permissions should be an array");

    assert_eq!(permissions, &[serde_json::json!("core:default")]);
    assert!(!content.contains("shell:"));
    assert!(!content.contains("fs:"));
    assert!(!content.contains("process"));
}

#[test]
fn root_desktop_gui_launcher_wraps_tauri_start_mode() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let launcher_path = manifest_dir
        .join("..")
        .join("..")
        .join("start-hermes-control-gui.ps1");

    let content = std::fs::read_to_string(&launcher_path).expect("desktop launcher should exist");

    assert!(content.contains("start-hermes-control.ps1"));
    assert!(content.contains("-GuiMode"));
    assert!(content.contains("Tauri"));
    assert!(content.contains("-OperatorId"));
    assert!(!content.contains("@arguments"));
    assert!(!content.contains("Start-Process"));
    assert!(!content.contains("npm run dev"));
}
