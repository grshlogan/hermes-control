use hermes_control_core::{
    import_provider_json, parse_control_config, parse_model_runtimes_config, parse_providers_config,
};
use hermes_control_types::{
    AiProviderKind, ModelRuntimeStartKind, ModelRuntimeStopKind, ProviderSecretSource,
};

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
default_user = "root"

[hermes]
agent_root = "E:\\WSL\\Hermres\\hermes-agent"
health_url = "http://127.0.0.1:8642/health"
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
    assert_eq!(config.wsl.default_user, "root");
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
id = "external.api-relay"
kind = "AnthropicClaude"
display_name = "API Relay"
base_url = "https://api-relay.example.com/"
api_key_ref = "hermes/provider/api-relay"
models = ["claude-sonnet-4-6", "claude-haiku-4-5"]
default_account_id = "main"
default_model = "claude-sonnet-4-6"

[providers.anthropic_defaults]
model = "claude-sonnet-4-6"
sonnet = "claude-sonnet-4-6"
haiku = "claude-haiku-4-5"
opus = "claude-opus-4-7"

[providers.runtime_env]
API_TIMEOUT_MS = "600000"
effortLevel = "high"
NO_PROXY = "127.0.0.1,localhost"

[[providers.accounts]]
id = "main"
display_name = "Main relay token"
secret_ref = "env:ANTHROPIC_AUTH_TOKEN"
secret_env_key = "ANTHROPIC_AUTH_TOKEN"
enabled = true
priority = 10
"#,
    )
    .expect("providers config should parse");

    let provider = &providers.providers[0];
    assert_eq!(provider.kind, AiProviderKind::AnthropicClaude);
    assert_eq!(
        provider.api_key_ref.as_deref(),
        Some("hermes/provider/api-relay")
    );
    assert_eq!(provider.default_account_id.as_deref(), Some("main"));
    assert_eq!(provider.default_model.as_deref(), Some("claude-sonnet-4-6"));
    assert_eq!(
        provider
            .anthropic_defaults
            .as_ref()
            .and_then(|defaults| defaults.haiku.as_deref()),
        Some("claude-haiku-4-5")
    );
    assert_eq!(
        provider
            .runtime_env
            .get("API_TIMEOUT_MS")
            .map(String::as_str),
        Some("600000")
    );
    assert_eq!(provider.accounts.len(), 1);
    assert_eq!(provider.accounts[0].id, "main");
    assert_eq!(
        provider.accounts[0].secret_source,
        ProviderSecretSource::Env
    );
    assert_eq!(provider.accounts[0].secret_env_key, "ANTHROPIC_AUTH_TOKEN");

    let runtimes = parse_model_runtimes_config(
        r#"
[[runtimes]]
id = "vllm-local"
kind = "Vllm"
workspace = "E:\\WSL\\Hermres\\hermes-control\\vLLM"
wsl_distro = "Ubuntu-Hermes-Codex"
endpoint = "http://127.0.0.1:18080/v1"
models_endpoint = "http://127.0.0.1:18080/v1/models"
log_dir = "E:\\WSL\\Hermres\\hermes-control\\vLLM\\logs"
model_root = "/root/Hermres/models"

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
"#,
    )
    .expect("model runtimes config should parse");

    let variant = &runtimes.runtimes[0].variants[0];
    assert_eq!(variant.id, "qwen36-mtp");
    assert_eq!(
        runtimes.runtimes[0].model_root.as_deref(),
        Some("/root/Hermres/models")
    );
    assert_eq!(variant.max_model_len, 90000);
    assert_eq!(variant.start.kind, ModelRuntimeStartKind::WslScript);
    assert_eq!(variant.stop.kind, ModelRuntimeStopKind::ProcessMatch);
}

#[test]
fn project_providers_include_api_relay_route() {
    let providers = parse_providers_config(include_str!("../../../config/providers.toml"))
        .expect("project providers config should parse");

    let relay = providers
        .providers
        .iter()
        .find(|provider| provider.id == "external.api-relay")
        .expect("API relay provider should be configured");

    assert_eq!(relay.kind, AiProviderKind::AnthropicClaude);
    assert_eq!(relay.display_name, "API Relay");
    assert_eq!(
        relay.api_key_ref.as_deref(),
        Some("hermes/provider/api-relay")
    );
    assert!(
        relay
            .base_url
            .as_deref()
            .is_some_and(|url| url.starts_with("https://"))
    );
    assert!(relay.models.iter().any(|model| model.contains("claude")));
    assert_eq!(relay.default_account_id.as_deref(), Some("main"));
    assert!(
        relay
            .accounts
            .iter()
            .any(|account| account.secret_env_key == "ANTHROPIC_AUTH_TOKEN")
    );
}

#[test]
fn imports_claude_relay_env_json_without_storing_raw_token() {
    let import = import_provider_json(
        r#"
{
  "type": "claude-relay",
  "id": "external.api-relay",
  "name": "API Relay",
  "ANTHROPIC_BASE_URL": "https://api-relay.example.com/",
  "ANTHROPIC_AUTH_TOKEN": "$env:ANTHROPIC_AUTH_TOKEN",
  "ANTHROPIC_MODEL": "claude-sonnet-4-6",
  "ANTHROPIC_DEFAULT_SONNET_MODEL": "claude-sonnet-4-6",
  "ANTHROPIC_DEFAULT_HAIKU_MODEL": "claude-haiku-4-5",
  "ANTHROPIC_DEFAULT_OPUS_MODEL": "claude-opus-4-7",
  "API_TIMEOUT_MS": "600000",
  "effortLevel": "high"
}
"#,
    )
    .expect("claude relay import should normalize");

    assert_eq!(import.providers.len(), 1);
    let provider = &import.providers[0];
    assert_eq!(provider.id, "external.api-relay");
    assert_eq!(provider.kind, AiProviderKind::AnthropicClaude);
    assert_eq!(
        provider.base_url.as_deref(),
        Some("https://api-relay.example.com/")
    );
    assert_eq!(provider.default_account_id.as_deref(), Some("main"));
    assert_eq!(provider.accounts.len(), 1);
    assert_eq!(provider.accounts[0].secret_env_key, "ANTHROPIC_AUTH_TOKEN");
    assert_eq!(provider.accounts[0].secret_ref, "env:ANTHROPIC_AUTH_TOKEN");
    assert_eq!(
        provider
            .anthropic_defaults
            .as_ref()
            .and_then(|defaults| defaults.opus.as_deref()),
        Some("claude-opus-4-7")
    );
    assert_eq!(
        provider.runtime_env.get("effortLevel").map(String::as_str),
        Some("high")
    );
    assert!(
        !serde_json::to_string(provider)
            .expect("provider should serialize")
            .contains("sk-")
    );
}

#[test]
fn provider_json_import_rejects_raw_api_tokens() {
    let err = import_provider_json(
        r#"
{
  "type": "claude-relay",
  "name": "Unsafe Relay",
  "ANTHROPIC_BASE_URL": "https://api-relay.example.com/",
  "ANTHROPIC_AUTH_TOKEN": "sk-ant-raw-secret"
}
"#,
    )
    .expect_err("raw token import should be rejected");

    assert!(
        err.to_string().contains("raw secret"),
        "unexpected error: {err}"
    );
}
