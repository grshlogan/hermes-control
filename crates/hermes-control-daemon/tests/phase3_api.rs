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
use serde_json::Value;
use tempfile::TempDir;
use tower::ServiceExt;

const TOKEN: &str = "phase3-test-token";

#[tokio::test]
async fn phase3_router_requires_bearer_auth() {
    let fixture = Fixture::new();
    let router = build_router(&fixture.config_dir, TOKEN).expect("router should build");

    let response = get(router.clone(), "/v1/providers", None).await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let response = get(router, "/v1/providers", Some("wrong-token")).await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn phase3_router_initializes_sqlite_state_and_audit_files() {
    let fixture = Fixture::new();
    let _router = build_router(&fixture.config_dir, TOKEN).expect("router should build");

    assert!(fixture.state_db.exists());
    assert!(fixture.audit_db.exists());

    let state_tables = table_names(&fixture.state_db);
    assert!(state_tables.contains(&"route_state".to_owned()));
    assert!(state_tables.contains(&"operation_state".to_owned()));
    assert!(state_tables.contains(&"confirmations".to_owned()));

    let audit_tables = table_names(&fixture.audit_db);
    assert!(audit_tables.contains(&"audit_events".to_owned()));
}

#[tokio::test]
async fn phase3_router_serves_read_only_config_state_and_model_routes() {
    let fixture = Fixture::new();
    let router = build_router(&fixture.config_dir, TOKEN).expect("router should build");

    let providers = get_json(router.clone(), "/v1/providers").await;
    assert_eq!(providers[0]["id"], "external.test");
    assert_eq!(providers[1]["id"], "local.vllm.qwen36-mtp");

    let active_route = get_json(router.clone(), "/v1/route/active").await;
    assert!(active_route["active_profile_id"].is_null());

    let models = get_json(router, "/v1/models").await;
    assert_eq!(models[0]["runtime_id"], "vllm-local");
    assert_eq!(models[0]["variant_id"], "qwen36-mtp");
    assert_eq!(models[0]["served_model_name"], "qwen36-mtp");
    assert_eq!(models[0]["ready"], false);
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

fn table_names(path: &Path) -> Vec<String> {
    let connection = Connection::open(path).expect("database should open");
    let mut statement = connection
        .prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
        .expect("table query should prepare");
    statement
        .query_map([], |row| row.get::<_, String>(0))
        .expect("table query should run")
        .map(|row| row.expect("table name should read"))
        .collect()
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
