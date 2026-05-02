use hermes_control_cli::{CliOutputFormat, render_models, render_providers, render_status};
use hermes_control_types::{
    EndpointStatus, HealthStatus, ModelRuntimeSummary, ProviderConfig, ReadOnlyStatus,
    StateSummary, WslDistroStatus,
};

fn sample_status() -> ReadOnlyStatus {
    ReadOnlyStatus {
        wsl: Some(WslDistroStatus {
            name: "Ubuntu-Hermes-Codex".to_owned(),
            state: "Running".to_owned(),
            version: Some(2),
            default: true,
        }),
        hermes: EndpointStatus::ok("http://127.0.0.1:8642/health", 200),
        models: vec![ModelRuntimeSummary {
            runtime_id: "vllm-local".to_owned(),
            variant_id: "qwen36-mtp".to_owned(),
            served_model_name: "qwen36-mtp".to_owned(),
            endpoint: EndpointStatus::ok("http://127.0.0.1:18080/v1/models", 200),
            ready: true,
        }],
        state: StateSummary {
            state_db_exists: false,
            audit_db_exists: false,
        },
        overall: HealthStatus::Ok,
    }
}

#[test]
fn renders_status_text_for_operator() {
    let rendered = render_status(&sample_status(), CliOutputFormat::Text).expect("render status");

    assert!(rendered.contains("WSL: Ubuntu-Hermes-Codex Running"));
    assert!(rendered.contains("Hermes: ok"));
    assert!(rendered.contains("qwen36-mtp: ready"));
}

#[test]
fn renders_status_json_for_machine_use() {
    let rendered = render_status(&sample_status(), CliOutputFormat::Json).expect("render status");

    assert!(rendered.contains("\"overall\""));
    assert!(rendered.contains("\"Ok\""));
}

#[test]
fn renders_providers_without_secret_values() {
    let providers = vec![ProviderConfig {
        id: "external.openai-compatible".to_owned(),
        kind: hermes_control_types::AiProviderKind::OpenAiCompatible,
        display_name: "External API".to_owned(),
        base_url: Some("https://example.com/v1".to_owned()),
        api_key_ref: Some("hermes/provider/external-openai-compatible".to_owned()),
        models: vec!["coder".to_owned()],
        model_runtime: None,
        served_model_name: None,
    }];

    let rendered = render_providers(&providers, CliOutputFormat::Text).expect("render providers");

    assert!(rendered.contains("external.openai-compatible"));
    assert!(rendered.contains("secret_ref"));
    assert!(!rendered.contains("api_key ="));
}

#[test]
fn renders_model_summaries() {
    let status = sample_status();

    let rendered = render_models(&status.models, CliOutputFormat::Text).expect("render models");

    assert!(rendered.contains("vllm-local/qwen36-mtp"));
    assert!(rendered.contains("ready"));
}
