use hermes_control_core::{
    parse_control_config, parse_model_runtimes_config, parse_providers_config,
};
use hermes_control_types::{AiProviderKind, ModelRuntimeStartKind, ModelRuntimeStopKind};

const CONTROL_TOML: &str = r#"
[daemon]
bind = "127.0.0.1:18787"
api_token_ref = "hermes/control/api-token"
state_db = "state/state.sqlite"
audit_db = "state/audit.sqlite"
log_dir = "logs/daemon"
operation_timeout_seconds = 900

[wsl]
distro = "Ubuntu-Hermes-Codex"
default_user = "hermes"

[hermes]
agent_root = "E:\\WSL\\Hermres\\hermes-agent"
health_url = "http://127.0.0.1:18000/health"
logs = ["E:\\WSL\\Hermres\\hermes-agent\\logs"]

[policy]
require_confirm_for_destructive = true
allow_lan_bind = false
allow_raw_shell = false
redact_secrets = true
"#;

#[test]
fn parses_control_config_schema_from_plan() {
    let config = parse_control_config(CONTROL_TOML).expect("control config should parse");

    assert_eq!(config.daemon.bind, "127.0.0.1:18787");
    assert_eq!(config.daemon.api_token_ref, "hermes/control/api-token");
    assert_eq!(config.wsl.distro, "Ubuntu-Hermes-Codex");
    assert_eq!(config.wsl.default_user, "hermes");
    assert!(config.policy.require_confirm_for_destructive);
    assert!(!config.policy.allow_lan_bind);
    assert!(!config.policy.allow_raw_shell);
    assert!(config.policy.redact_secrets);
}

#[test]
fn rejects_lan_bind_when_policy_disallows_it() {
    let lan_config = CONTROL_TOML.replace("127.0.0.1:18787", "0.0.0.0:18787");

    let err = parse_control_config(&lan_config).expect_err("LAN bind should be rejected");

    assert!(
        err.to_string().contains("LAN bind"),
        "unexpected error: {err}"
    );
}

#[test]
fn parses_provider_and_model_runtime_schema() {
    let providers = parse_providers_config(
        r#"
[[providers]]
id = "local.vllm.qwen36-mtp"
kind = "LocalVllm"
display_name = "Qwen3.6 MTP via vLLM"
base_url = "http://127.0.0.1:18080/v1"
model_runtime = "vllm-local"
served_model_name = "qwen36-mtp"
"#,
    )
    .expect("providers config should parse");

    assert_eq!(providers.providers[0].kind, AiProviderKind::LocalVllm);
    assert_eq!(
        providers.providers[0].served_model_name.as_deref(),
        Some("qwen36-mtp")
    );

    let runtimes = parse_model_runtimes_config(
        r#"
[[runtimes]]
id = "vllm-local"
kind = "Vllm"
workspace = "E:\\WSL\\vLLM"
wsl_distro = "Ubuntu-Hermes-Codex"
endpoint = "http://127.0.0.1:18080/v1"
models_endpoint = "http://127.0.0.1:18080/v1/models"
log_dir = "E:\\WSL\\vLLM\\logs"

[[runtimes.variants]]
id = "qwen36-mtp"
served_model_name = "qwen36-mtp"
mode = "latency"
max_model_len = 90000
speculative_method = "mtp"
num_speculative_tokens = 2
start = { kind = "wsl_script", script = "/mnt/e/WSL/vLLM/scripts/serve-qwen36-mtp.sh" }
stop = { kind = "process_match", served_model_name = "qwen36-mtp" }
profiles = ["vllm.qwen36-mtp"]
"#,
    )
    .expect("model runtimes config should parse");

    let variant = &runtimes.runtimes[0].variants[0];
    assert_eq!(variant.id, "qwen36-mtp");
    assert_eq!(variant.max_model_len, 90000);
    assert_eq!(variant.start.kind, ModelRuntimeStartKind::WslScript);
    assert_eq!(variant.stop.kind, ModelRuntimeStopKind::ProcessMatch);
}
