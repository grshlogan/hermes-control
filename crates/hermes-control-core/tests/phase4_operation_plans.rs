use hermes_control_core::{HermesRuntimeController, WslController};
use hermes_control_types::{HermesAction, RiskLevel, WslAction};

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
            "hermes",
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
        "http://127.0.0.1:18000/health",
        "Ubuntu-Hermes-Codex",
        "hermes",
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
            "hermes",
            "--exec",
            "/home/hermes/Hermres/restart-services.sh"
        ]
    );
    assert_eq!(plan.commands[1].program, "wsl.exe");
    assert_eq!(
        plan.commands[1].args,
        [
            "--distribution",
            "Ubuntu-Hermes-Codex",
            "--user",
            "hermes",
            "--exec",
            "/home/hermes/Hermres/health-check.sh",
            "30",
            "ready"
        ]
    );
}
