use std::{
    fs,
    path::{Path, PathBuf},
};

use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode, header::AUTHORIZATION},
};
use hermes_control_daemon::build_router;
use rusqlite::Connection;
use serde_json::{Value, json};
use tempfile::TempDir;
use tower::ServiceExt;

const TOKEN: &str = "phase4-test-token";

#[tokio::test]
async fn wsl_action_dry_run_returns_fixed_command_preview() {
    let fixture = Fixture::new();
    let router = build_router(&fixture.config_dir, TOKEN).expect("router should build");

    let response = post_json(
        router,
        "/v1/wsl/action",
        json!({
            "requester": {"channel": "cli", "user_id": "phase4-test"},
            "action": "RestartDistro",
            "reason": "phase4 test",
            "dry_run": true
        }),
    )
    .await;

    assert_eq!(response["status"], "dry_run");
    assert_eq!(response["risk"], "Destructive");
    assert_eq!(response["dry_run"], true);
    assert!(
        response["summary"]
            .as_str()
            .unwrap()
            .contains("Restart WSL distro Ubuntu-Hermes-Codex")
    );
    assert_eq!(response["commands"][0]["program"], "wsl.exe");
    assert_eq!(response["commands"][0]["args"][0], "--terminate");
}

#[tokio::test]
async fn hermes_destructive_action_creates_confirmation_and_audit_preview() {
    let fixture = Fixture::new();
    let router = build_router(&fixture.config_dir, TOKEN).expect("router should build");

    let response = post_json(
        router,
        "/v1/hermes/action",
        json!({
            "requester": {"channel": "cli", "user_id": "phase4-test"},
            "action": "Kill",
            "reason": "phase4 test",
            "dry_run": false
        }),
    )
    .await;

    assert_eq!(response["status"], "confirmation_required");
    assert_eq!(response["risk"], "Destructive");
    assert!(
        response["confirmation_id"]
            .as_str()
            .unwrap()
            .starts_with("confirm_")
    );
    assert!(
        response["code_hint"]
            .as_str()
            .unwrap()
            .starts_with("HERMES-")
    );
    assert!(
        response["summary"]
            .as_str()
            .unwrap()
            .contains("Kill Hermes runtime")
    );

    let confirmation_count = row_count(&fixture.state_db, "confirmations");
    assert_eq!(confirmation_count, 1);
    let audit_count = row_count(&fixture.audit_db, "audit_events");
    assert_eq!(audit_count, 1);
}

async fn post_json(router: Router, path: &str, body: Value) -> Value {
    let response = router
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
        .expect("request should complete");
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body should collect");
    serde_json::from_slice(&bytes).expect("response should be JSON")
}

fn row_count(path: &Path, table: &str) -> i64 {
    let connection = Connection::open(path).expect("database should open");
    connection
        .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .expect("row count should read")
}

struct Fixture {
    _temp: TempDir,
    config_dir: PathBuf,
    state_db: PathBuf,
    audit_db: PathBuf,
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
default_user = "hermes"

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
id = "external.test"
kind = "OpenAiCompatible"
display_name = "External test"
base_url = "https://example.com/v1"
api_key_ref = "hermes/provider/external-test"
models = ["test-model"]
"#,
        )
        .expect("providers config should write");

        fs::write(
            config_dir.join("model-runtimes.toml"),
            r#"
[[runtimes]]
id = "vllm-local"
kind = "Vllm"
workspace = "E:\\WSL\\vLLM"
wsl_distro = "Ubuntu-Hermes-Codex"
endpoint = "http://127.0.0.1:9/v1"
models_endpoint = "http://127.0.0.1:9/v1/models"
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
        .expect("model runtime config should write");

        Self {
            _temp: temp,
            config_dir,
            state_db: root.join("state/state.sqlite"),
            audit_db: root.join("state/audit.sqlite"),
        }
    }
}
