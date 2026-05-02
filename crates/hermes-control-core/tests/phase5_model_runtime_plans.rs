use hermes_control_core::{ModelRuntimeController, parse_model_runtimes_config};
use hermes_control_types::{ModelAction, RiskLevel};

const MODEL_RUNTIMES: &str = r#"
[[runtimes]]
id = "vllm-local"
kind = "Vllm"
workspace = "E:\\WSL\\Hermres\\hermes-control\\vLLM"
wsl_distro = "Ubuntu-Hermes-Codex"
endpoint = "http://127.0.0.1:18080/v1"
models_endpoint = "http://127.0.0.1:18080/v1/models"
log_dir = "E:\\WSL\\Hermres\\hermes-control\\vLLM\\logs"

[[runtimes.variants]]
id = "qwen36-awq-int4"
served_model_name = "qwen36-awq-int4"
mode = "stable"
max_model_len = 90000
start = { kind = "wsl_script", script = "/mnt/e/WSL/Hermres/hermes-control/vLLM/scripts/start-qwen36-int4-eager.sh" }
stop = { kind = "process_match", served_model_name = "qwen36-awq-int4" }
profiles = ["vllm.qwen36-awq-int4"]

[[runtimes.variants]]
id = "qwen36-mtp"
served_model_name = "qwen36-mtp"
mode = "latency"
max_model_len = 90000
speculative_method = "mtp"
num_speculative_tokens = 2
start = { kind = "wsl_script", script = "/mnt/e/WSL/Hermres/hermes-control/vLLM/scripts/start-qwen36-mtp.sh" }
stop = { kind = "process_match", served_model_name = "qwen36-mtp" }
profiles = ["vllm.qwen36-mtp"]
"#;

#[test]
fn model_start_plan_uses_canonical_vllm_root_helpers() {
    let config = parse_model_runtimes_config(MODEL_RUNTIMES).expect("config should parse");
    let controller = ModelRuntimeController::new(&config, "root");

    let plan = controller
        .plan("qwen36-mtp", ModelAction::Start)
        .expect("qwen36-mtp should be known");

    assert_eq!(plan.risk, RiskLevel::NormalMutating);
    assert!(!plan.requires_confirmation);
    assert!(plan.summary.contains("Start vLLM model qwen36-mtp"));
    assert!(plan.summary.contains("fallback qwen36-awq-int4"));
    assert_eq!(plan.commands.len(), 1);
    assert_eq!(
        plan.commands[0].args,
        [
            "--distribution",
            "Ubuntu-Hermes-Codex",
            "--user",
            "root",
            "--exec",
            "/opt/hermes-control/bin/hermes-control-vllm-start-with-fallback.sh",
            "qwen36-mtp",
            "qwen36-awq-int4",
        ]
    );
}

#[test]
fn model_stop_plan_requires_confirmation_and_targets_served_model() {
    let config = parse_model_runtimes_config(MODEL_RUNTIMES).expect("config should parse");
    let controller = ModelRuntimeController::new(&config, "root");

    let plan = controller
        .plan("qwen36-mtp", ModelAction::Stop)
        .expect("qwen36-mtp should be known");

    assert_eq!(plan.risk, RiskLevel::Destructive);
    assert!(plan.requires_confirmation);
    assert_eq!(
        plan.commands[0].args,
        [
            "--distribution",
            "Ubuntu-Hermes-Codex",
            "--user",
            "root",
            "--exec",
            "/opt/hermes-control/bin/hermes-control-vllm-stop.sh",
            "qwen36-mtp",
        ]
    );
}

#[test]
fn model_install_plan_bootstraps_project_vllm_runtime() {
    let config = parse_model_runtimes_config(MODEL_RUNTIMES).expect("config should parse");
    let controller = ModelRuntimeController::new(&config, "root");

    let plan = controller
        .plan("qwen36-mtp", ModelAction::Install)
        .expect("qwen36-mtp should be known");

    assert_eq!(plan.risk, RiskLevel::NormalMutating);
    assert!(!plan.requires_confirmation);
    assert!(plan.summary.contains("Install or repair vLLM runtime"));
    assert_eq!(
        plan.commands[0].args,
        [
            "--distribution",
            "Ubuntu-Hermes-Codex",
            "--user",
            "root",
            "--exec",
            "/opt/hermes-control/bin/hermes-control-vllm-bootstrap.sh",
            "qwen36-mtp",
        ]
    );
}
