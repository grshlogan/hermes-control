use hermes_control_core::{HermesRuntimeController, OpenWebUiController, WslController};
use hermes_control_types::{HermesAction, OpenWebUiAction, RiskLevel, WslAction};

#[test]
fn wsl_restart_plan_uses_fixed_wsl_commands_and_requires_confirmation() {
    let controller = WslController::new("Ubuntu-Hermes-Codex");

    let plan = controller.plan(WslAction::RestartDistro);

    assert_eq!(plan.risk, RiskLevel::Destructive);
    assert!(plan.requires_confirmation);
    assert!(
        plan.summary
            .contains("Restart WSL distro Ubuntu-Hermes-Codex")
    );
    assert_eq!(plan.commands.len(), 2);
    assert_eq!(plan.commands[0].program, "wsl.exe");
    assert_eq!(
        plan.commands[0].args,
        ["--terminate", "Ubuntu-Hermes-Codex"]
    );
    assert_eq!(plan.commands[1].program, "wsl.exe");
    assert_eq!(
        plan.commands[1].args,
        [
            "--distribution",
            "Ubuntu-Hermes-Codex",
            "--user",
            "root",
            "--exec",
            "true"
        ]
    );
}

#[test]
fn wsl_shutdown_all_plan_is_destructive_and_fixed() {
    let controller = WslController::new("Ubuntu-Hermes-Codex");

    let plan = controller.plan(WslAction::ShutdownAll);

    assert_eq!(plan.risk, RiskLevel::Destructive);
    assert!(plan.requires_confirmation);
    assert!(plan.summary.contains("Shutdown all WSL distributions"));
    assert_eq!(plan.commands.len(), 1);
    assert_eq!(plan.commands[0].program, "wsl.exe");
    assert_eq!(plan.commands[0].args, ["--shutdown"]);
}

#[test]
fn hermes_restart_plan_uses_fixed_wsl_scripts_and_health_check() {
    let controller = HermesRuntimeController::with_wsl(
        "E:\\WSL\\Hermres\\hermes-agent",
        "http://127.0.0.1:8642/health",
        "Ubuntu-Hermes-Codex",
        "root",
    );

    let plan = controller.plan(HermesAction::Restart);

    assert_eq!(plan.risk, RiskLevel::Destructive);
    assert!(plan.requires_confirmation);
    assert!(plan.summary.contains("Restart Hermes runtime"));
    assert!(plan.summary.contains("E:\\WSL\\Hermres\\hermes-agent"));
    assert_eq!(plan.commands.len(), 2);
    assert_eq!(plan.commands[0].program, "wsl.exe");
    assert_eq!(
        plan.commands[0].args,
        [
            "--distribution",
            "Ubuntu-Hermes-Codex",
            "--user",
            "root",
            "--exec",
            "/opt/hermes-control/bin/hermes-control-restart.sh"
        ]
    );
    assert_eq!(plan.commands[1].program, "wsl.exe");
    assert_eq!(
        plan.commands[1].args,
        [
            "--distribution",
            "Ubuntu-Hermes-Codex",
            "--user",
            "root",
            "--exec",
            "/opt/hermes-control/bin/hermes-control-health.sh",
            "30",
            "ready"
        ]
    );
}

#[test]
fn openwebui_runtime_plans_use_fixed_wsl_scripts() {
    let controller = OpenWebUiController::new("Ubuntu-Hermes-Codex", "root");

    let wake = controller.plan(OpenWebUiAction::Wake);
    assert_eq!(wake.risk, RiskLevel::NormalMutating);
    assert!(!wake.requires_confirmation);
    assert!(wake.summary.contains("Wake Open WebUI"));
    assert_eq!(
        wake.commands[0].args,
        [
            "--distribution",
            "Ubuntu-Hermes-Codex",
            "--user",
            "root",
            "--exec",
            "/opt/hermes-control/bin/hermes-control-openwebui-refresh.sh",
            "force"
        ]
    );

    let status = controller.plan(OpenWebUiAction::Status);
    assert_eq!(status.risk, RiskLevel::ReadOnly);
    assert!(!status.requires_confirmation);
    assert_eq!(
        status.commands[0].args,
        [
            "--distribution",
            "Ubuntu-Hermes-Codex",
            "--user",
            "root",
            "--exec",
            "/opt/hermes-control/bin/hermes-control-openwebui-status.sh"
        ]
    );

    let stop = controller.plan(OpenWebUiAction::Stop);
    assert_eq!(stop.risk, RiskLevel::Destructive);
    assert!(stop.requires_confirmation);
    assert_eq!(
        stop.commands[0].args,
        [
            "--distribution",
            "Ubuntu-Hermes-Codex",
            "--user",
            "root",
            "--exec",
            "/opt/hermes-control/bin/hermes-control-openwebui-stop.sh"
        ]
    );
}
