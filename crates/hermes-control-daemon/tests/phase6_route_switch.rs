use std::{
    fs,
    path::{Path, PathBuf},
};

use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode, header::AUTHORIZATION},
};
use hermes_control_daemon::{
    ExecutableOperation, ExecutionOutcome, OperationExecutor, build_router,
    build_router_with_executor,
};
use rusqlite::Connection;
use serde_json::{Value, json};
use tempfile::TempDir;
use tower::ServiceExt;

const TOKEN: &str = "phase6-test-token";

#[tokio::test]
async fn route_switch_dry_run_validates_provider_without_mutating_active_route() {
    let fixture = Fixture::new();
    let router = build_router(&fixture.config_dir, TOKEN).expect("router should build");

    let response = post_json(
        router.clone(),
        "/v1/route/switch",
        json!({
            "requester": {"channel": "cli", "user_id": "phase6-test"},
            "profile_id": "external.test",
            "reason": "phase6 dry-run",
            "dry_run": true
        }),
    )
    .await;

    assert_eq!(response["status"], "dry_run");
    assert_eq!(response["risk"], "NormalMutating");
    assert!(
        response["summary"]
            .as_str()
            .unwrap()
            .contains("Switch active route to external.test")
    );
    assert_eq!(
        response["commands"][0]["args"],
        json!([
            "--distribution",
            "Ubuntu-Hermes-Codex",
            "--user",
            "root",
            "--exec",
            "/opt/hermes-control/bin/hermes-control-route-apply.sh",
            "external.test",
            "openai-compatible",
            "https://example.com/v1",
            "test-model",
            "LM_API_KEY"
        ])
    );

    let active = get_json(router, "/v1/route/active").await;
    assert!(active["active_profile_id"].is_null());
    assert!(active["last_known_good_profile_id"].is_null());
}

#[tokio::test]
async fn api_relay_route_preview_uses_anthropic_secret_boundary() {
    let fixture = Fixture::new();
    let router = build_router(&fixture.config_dir, TOKEN).expect("router should build");

    let response = post_json(
        router,
        "/v1/route/switch",
        json!({
            "requester": {"channel": "cli", "user_id": "phase6-test"},
            "profile_id": "external.api-relay",
            "reason": "relay dry-run",
            "dry_run": true
        }),
    )
    .await;

    assert_eq!(response["status"], "dry_run");
    assert_eq!(
        response["commands"][0]["args"],
        json!([
            "--distribution",
            "Ubuntu-Hermes-Codex",
            "--user",
            "root",
            "--exec",
            "/opt/hermes-control/bin/hermes-control-route-apply.sh",
            "external.api-relay",
            "claude",
            "https://api-relay.example.com/",
            "claude-sonnet-4-6",
            "ANTHROPIC_AUTH_TOKEN"
        ])
    );
    assert_eq!(
        response["commands"][0]["env"],
        json!({
            "ANTHROPIC_DEFAULT_HAIKU_MODEL": "claude-haiku-4-5",
            "ANTHROPIC_DEFAULT_OPUS_MODEL": "claude-opus-4-7",
            "ANTHROPIC_DEFAULT_SONNET_MODEL": "claude-sonnet-4-6",
            "API_TIMEOUT_MS": "600000",
            "NO_PROXY": "127.0.0.1,localhost",
            "effortLevel": "high"
        })
    );
}

#[tokio::test]
async fn deepseek_route_preview_uses_official_api_secret_boundary() {
    let fixture = Fixture::new();
    let router = build_router(&fixture.config_dir, TOKEN).expect("router should build");

    let response = post_json(
        router,
        "/v1/route/switch",
        json!({
            "requester": {"channel": "cli", "user_id": "phase6-test"},
            "profile_id": "deepseek.official",
            "reason": "deepseek official dry-run",
            "dry_run": true
        }),
    )
    .await;

    assert_eq!(response["status"], "dry_run");
    assert_eq!(
        response["commands"][0]["args"],
        json!([
            "--distribution",
            "Ubuntu-Hermes-Codex",
            "--user",
            "root",
            "--exec",
            "/opt/hermes-control/bin/hermes-control-route-apply.sh",
            "deepseek.official",
            "deepseek",
            "https://api.deepseek.com/v1",
            "deepseek-chat",
            "DEEPSEEK_API_KEY"
        ])
    );
    assert_eq!(
        response["commands"][0]["env"],
        json!({
            "API_TIMEOUT_MS": "600000"
        })
    );
}

#[tokio::test]
async fn route_switch_updates_active_route_and_last_known_good() {
    let fixture = Fixture::new();
    let router = build_router(&fixture.config_dir, TOKEN).expect("router should build");

    let first = post_json(
        router.clone(),
        "/v1/route/switch",
        json!({
            "requester": {"channel": "cli", "user_id": "phase6-test"},
            "profile_id": "external.test",
            "reason": "phase6 first switch",
            "dry_run": false
        }),
    )
    .await;
    assert_eq!(first["status"], "completed");

    let second = post_json(
        router.clone(),
        "/v1/route/switch",
        json!({
            "requester": {"channel": "cli", "user_id": "phase6-test"},
            "profile_id": "external.backup",
            "reason": "phase6 second switch",
            "dry_run": false
        }),
    )
    .await;
    assert_eq!(second["status"], "completed");

    let active = get_json(router, "/v1/route/active").await;
    assert_eq!(active["active_profile_id"], "external.backup");
    assert_eq!(active["last_known_good_profile_id"], "external.test");
    assert_eq!(row_count(&fixture.audit_db, "audit_events"), 2);
}

#[tokio::test]
async fn route_rollback_dry_run_uses_last_known_good_without_mutating_active_route() {
    let fixture = Fixture::new();
    let router = build_router(&fixture.config_dir, TOKEN).expect("router should build");

    let first = post_json(
        router.clone(),
        "/v1/route/switch",
        json!({
            "requester": {"channel": "cli", "user_id": "phase6-test"},
            "profile_id": "external.test",
            "reason": "phase6 first switch",
            "dry_run": false
        }),
    )
    .await;
    assert_eq!(first["status"], "completed");

    let second = post_json(
        router.clone(),
        "/v1/route/switch",
        json!({
            "requester": {"channel": "cli", "user_id": "phase6-test"},
            "profile_id": "external.backup",
            "reason": "phase6 second switch",
            "dry_run": false
        }),
    )
    .await;
    assert_eq!(second["status"], "completed");

    let rollback = post_json(
        router.clone(),
        "/v1/route/rollback",
        json!({
            "requester": {"channel": "cli", "user_id": "phase6-test"},
            "reason": "phase6 rollback dry-run",
            "dry_run": true
        }),
    )
    .await;

    assert_eq!(rollback["status"], "dry_run");
    assert!(
        rollback["summary"]
            .as_str()
            .unwrap()
            .contains("Rollback active route to external.test")
    );
    assert_eq!(
        rollback["commands"][0]["args"],
        json!([
            "--distribution",
            "Ubuntu-Hermes-Codex",
            "--user",
            "root",
            "--exec",
            "/opt/hermes-control/bin/hermes-control-route-apply.sh",
            "external.test",
            "openai-compatible",
            "https://example.com/v1",
            "test-model",
            "LM_API_KEY"
        ])
    );

    let active = get_json(router, "/v1/route/active").await;
    assert_eq!(active["active_profile_id"], "external.backup");
    assert_eq!(active["last_known_good_profile_id"], "external.test");
}

#[tokio::test]
async fn route_rollback_applies_last_known_good_route() {
    let fixture = Fixture::new();
    let router = build_router(&fixture.config_dir, TOKEN).expect("router should build");

    let first = post_json(
        router.clone(),
        "/v1/route/switch",
        json!({
            "requester": {"channel": "cli", "user_id": "phase6-test"},
            "profile_id": "external.test",
            "reason": "phase6 first switch",
            "dry_run": false
        }),
    )
    .await;
    assert_eq!(first["status"], "completed");

    let second = post_json(
        router.clone(),
        "/v1/route/switch",
        json!({
            "requester": {"channel": "cli", "user_id": "phase6-test"},
            "profile_id": "external.backup",
            "reason": "phase6 second switch",
            "dry_run": false
        }),
    )
    .await;
    assert_eq!(second["status"], "completed");

    let rollback = post_json(
        router.clone(),
        "/v1/route/rollback",
        json!({
            "requester": {"channel": "cli", "user_id": "phase6-test"},
            "reason": "phase6 rollback",
            "dry_run": false
        }),
    )
    .await;

    assert_eq!(rollback["status"], "completed");
    assert_eq!(
        rollback["summary"],
        "Rolled active route back to external.test."
    );

    let active = get_json(router, "/v1/route/active").await;
    assert_eq!(active["active_profile_id"], "external.test");
    assert_eq!(active["last_known_good_profile_id"], "external.backup");
    assert_eq!(row_count(&fixture.audit_db, "audit_events"), 3);
}

#[tokio::test]
async fn route_rollback_without_last_known_good_is_rejected() {
    let fixture = Fixture::new();
    let router = build_router(&fixture.config_dir, TOKEN).expect("router should build");

    let response = post_raw_json(
        router,
        "/v1/route/rollback",
        json!({
            "requester": {"channel": "cli", "user_id": "phase6-test"},
            "reason": "phase6 rollback unavailable",
            "dry_run": false
        }),
    )
    .await;

    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn route_switch_to_unready_local_vllm_is_rejected() {
    let fixture = Fixture::new();
    let router = build_router(&fixture.config_dir, TOKEN).expect("router should build");

    let response = post_raw_json(
        router,
        "/v1/route/switch",
        json!({
            "requester": {"channel": "cli", "user_id": "phase6-test"},
            "profile_id": "local.vllm.qwen36-mtp",
            "reason": "phase6 local gate",
            "dry_run": false
        }),
    )
    .await;

    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn local_vllm_route_preview_uses_no_secret_env_key() {
    let fixture = Fixture::new();
    let router = build_router(&fixture.config_dir, TOKEN).expect("router should build");

    let response = post_json(
        router,
        "/v1/route/switch",
        json!({
            "requester": {"channel": "cli", "user_id": "phase6-test"},
            "profile_id": "local.vllm.qwen36-mtp",
            "reason": "phase6 local dry-run",
            "dry_run": true
        }),
    )
    .await;

    assert_eq!(
        response["commands"][0]["args"],
        json!([
            "--distribution",
            "Ubuntu-Hermes-Codex",
            "--user",
            "root",
            "--exec",
            "/opt/hermes-control/bin/hermes-control-route-apply.sh",
            "local.vllm.qwen36-mtp",
            "local-vllm",
            "auto-vllm",
            "qwen36-mtp",
            "none"
        ])
    );
}

#[tokio::test]
async fn route_switch_does_not_update_active_route_when_apply_fails() {
    let fixture = Fixture::new();
    let router = build_router_with_executor(
        &fixture.config_dir,
        TOKEN,
        std::sync::Arc::new(FailingExecutor),
    )
    .expect("router should build");

    let response = post_json(
        router.clone(),
        "/v1/route/switch",
        json!({
            "requester": {"channel": "cli", "user_id": "phase6-test"},
            "profile_id": "external.test",
            "reason": "phase6 failed apply",
            "dry_run": false
        }),
    )
    .await;

    assert_eq!(response["status"], "failed");
    let active = get_json(router, "/v1/route/active").await;
    assert!(active["active_profile_id"].is_null());
    assert!(active["last_known_good_profile_id"].is_null());
}

async fn get(router: Router, path: &str, token: Option<&str>) -> axum::response::Response {
    let mut request = Request::builder().uri(path);
    if let Some(token) = token {
        request = request.header(AUTHORIZATION, format!("Bearer {token}"));
    }

    router
        .oneshot(request.body(Body::empty()).expect("request should build"))
        .await
        .expect("request should complete")
}

async fn get_json(router: Router, path: &str) -> Value {
    let response = get(router, path, Some(TOKEN)).await;
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body should collect");
    serde_json::from_slice(&bytes).expect("response should be JSON")
}

async fn post_json(router: Router, path: &str, body: Value) -> Value {
    let response = post_raw_json(router, path, body).await;
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body should collect");
    serde_json::from_slice(&bytes).expect("response should be JSON")
}

async fn post_raw_json(router: Router, path: &str, body: Value) -> axum::response::Response {
    router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(path)
                .header(AUTHORIZATION, format!("Bearer {TOKEN}"))
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .expect("request should build"),
        )
        .await
        .expect("request should complete")
}

fn row_count(path: &Path, table: &str) -> i64 {
    let connection = Connection::open(path).expect("database should open");
    connection
        .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get::<_, i64>(0)
        })
        .expect("count should read")
}

struct Fixture {
    _temp: TempDir,
    config_dir: PathBuf,
    audit_db: PathBuf,
}

struct FailingExecutor;

impl OperationExecutor for FailingExecutor {
    fn execute(&self, operation: &ExecutableOperation) -> ExecutionOutcome {
        assert_eq!(operation.commands.len(), 1);
        ExecutionOutcome {
            status: "failed".to_owned(),
            summary: "simulated route apply failure".to_owned(),
            output: None,
        }
    }
}

impl Fixture {
    fn new() -> Self {
        let temp = TempDir::new().expect("temp dir should create");
        let root = temp.path().to_path_buf();
        let config_dir = root.join("config");
        fs::create_dir_all(&config_dir).expect("config dir should create");

        fs::write(
            config_dir.join("control.toml"),
            r#"
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
health_url = "http://127.0.0.1:9/health"
logs = ["E:\\WSL\\Hermres\\hermes-agent\\logs"]

[policy]
require_confirm_for_destructive = true
allow_lan_bind = false
allow_raw_shell = false
redact_secrets = true
"#,
        )
        .expect("control config should write");

        fs::write(
            config_dir.join("providers.toml"),
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

[[providers]]
id = "deepseek.official"
kind = "DeepSeek"
display_name = "DeepSeek Official API"
base_url = "https://api.deepseek.com/v1"
api_key_ref = "env:DEEPSEEK_API_KEY"
models = ["deepseek-chat", "deepseek-reasoner"]
default_account_id = "main"
default_model = "deepseek-chat"

[providers.runtime_env]
API_TIMEOUT_MS = "600000"

[[providers.accounts]]
id = "main"
display_name = "Main DeepSeek token"
secret_ref = "env:DEEPSEEK_API_KEY"
secret_env_key = "DEEPSEEK_API_KEY"
enabled = true
priority = 10

[[providers]]
id = "external.test"
kind = "OpenAiCompatible"
display_name = "External test"
base_url = "https://example.com/v1"
api_key_ref = "hermes/provider/external-test"
models = ["test-model"]

[[providers]]
id = "external.backup"
kind = "DeepSeek"
display_name = "External backup"
base_url = "https://backup.example.com/v1"
api_key_ref = "hermes/provider/external-backup"
models = ["backup-model"]

[[providers]]
id = "local.vllm.qwen36-mtp"
kind = "LocalVllm"
display_name = "Qwen3.6 MTP via vLLM"
base_url = "http://127.0.0.1:9/v1"
model_runtime = "vllm-local"
served_model_name = "qwen36-mtp"
"#,
        )
        .expect("providers config should write");

        fs::write(
            config_dir.join("model-runtimes.toml"),
            r#"
[[runtimes]]
id = "vllm-local"
kind = "Vllm"
workspace = "E:\\WSL\\Hermres\\hermes-control\\vLLM"
wsl_distro = "Ubuntu-Hermes-Codex"
endpoint = "http://127.0.0.1:9/v1"
models_endpoint = "http://127.0.0.1:9/v1/models"
log_dir = "E:\\WSL\\Hermres\\hermes-control\\vLLM\\logs"

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
        .expect("model runtime config should write");

        Self {
            _temp: temp,
            config_dir,
            audit_db: root.join("state/audit.sqlite"),
        }
    }
}
