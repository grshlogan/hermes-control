use std::path::Path;

use hermes_control_gui::{
    GuiConfig, GuiDaemonCommand, GuiLogTarget, gui_boundary, gui_tauri_capability, log_tail_path,
    route_rollback_request, route_switch_request,
};
use hermes_control_types::{RequesterChannel, RouteRollbackRequest, RouteSwitchRequest};

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
        GuiLogTarget::all()
            .iter()
            .map(|target| target.as_str())
            .collect::<Vec<_>>(),
        vec!["daemon", "bot", "hermes"]
    );
}

#[test]
fn gui_daemon_commands_do_not_include_raw_process_or_filesystem_access() {
    let commands = GuiDaemonCommand::all();
    assert!(commands.contains(&GuiDaemonCommand::DashboardSnapshot));
    assert!(commands.contains(&GuiDaemonCommand::RouteSwitchPreview));
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
