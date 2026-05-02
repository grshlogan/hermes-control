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
    CommandOutput, CommandRunner, ExecutableOperation, ExecutionOutcome, OperationExecutor,
    WindowsCommandExecutor, build_router, build_router_with_executor,
};
use rusqlite::Connection;
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};
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

#[tokio::test]
async fn hermes_restart_dry_run_returns_fixed_wsl_script_previews() {
    let fixture = Fixture::new();
    let router = build_router(&fixture.config_dir, TOKEN).expect("router should build");

    let response = post_json(
        router,
        "/v1/hermes/action",
        json!({
            "requester": {"channel": "cli", "user_id": "phase4-test"},
            "action": "Restart",
            "reason": "phase4 Hermes restart preview",
            "dry_run": true
        }),
    )
    .await;

    assert_eq!(response["status"], "dry_run");
    assert_eq!(response["risk"], "Destructive");
    assert_eq!(response["commands"][0]["program"], "wsl.exe");
    assert_eq!(
        response["commands"][0]["args"],
        json!([
            "--distribution",
            "Ubuntu-Hermes-Codex",
            "--user",
            "hermes",
            "--exec",
            "/home/hermes/Hermres/restart-services.sh"
        ])
    );
    assert_eq!(
        response["commands"][1]["args"],
        json!([
            "--distribution",
            "Ubuntu-Hermes-Codex",
            "--user",
            "hermes",
            "--exec",
            "/home/hermes/Hermres/health-check.sh",
            "30",
            "ready"
        ])
    );
}

#[tokio::test]
async fn pending_confirmation_locks_second_mutating_action_until_confirmed() {
    let fixture = Fixture::new();
    let router = build_router(&fixture.config_dir, TOKEN).expect("router should build");

    let first = post_json(
        router.clone(),
        "/v1/hermes/action",
        json!({
            "requester": {"channel": "cli", "user_id": "phase4-test"},
            "action": "Restart",
            "reason": "phase4 lock test",
            "dry_run": false
        }),
    )
    .await;
    assert_eq!(first["status"], "confirmation_required");

    let locked = post_raw_json(
        router.clone(),
        "/v1/wsl/action",
        json!({
            "requester": {"channel": "cli", "user_id": "phase4-test"},
            "action": "RestartDistro",
            "reason": "phase4 lock test",
            "dry_run": false
        }),
    )
    .await;
    assert_eq!(locked.status, StatusCode::CONFLICT);

    let confirmed = post_json(
        router.clone(),
        "/v1/confirm",
        json!({
            "requester": {"channel": "cli", "user_id": "phase4-test"},
            "code": first["code_hint"]
        }),
    )
    .await;
    assert_eq!(confirmed["status"], "confirmed");

    let after_confirm = post_json(
        router,
        "/v1/wsl/action",
        json!({
            "requester": {"channel": "cli", "user_id": "phase4-test"},
            "action": "RestartDistro",
            "reason": "phase4 lock test",
            "dry_run": false
        }),
    )
    .await;
    assert_eq!(after_confirm["status"], "confirmation_required");
    assert_eq!(
        confirmation_statuses(&fixture.state_db),
        vec!["confirmed".to_owned(), "pending".to_owned()]
    );
}

#[tokio::test]
async fn cancel_marks_pending_confirmation_cancelled_and_releases_lock() {
    let fixture = Fixture::new();
    let router = build_router(&fixture.config_dir, TOKEN).expect("router should build");

    let first = post_json(
        router.clone(),
        "/v1/hermes/action",
        json!({
            "requester": {"channel": "cli", "user_id": "phase4-test"},
            "action": "Kill",
            "reason": "phase4 cancel test",
            "dry_run": false
        }),
    )
    .await;
    assert_eq!(first["status"], "confirmation_required");

    let cancelled = post_json(
        router.clone(),
        "/v1/cancel",
        json!({
            "requester": {"channel": "cli", "user_id": "phase4-test"}
        }),
    )
    .await;
    assert_eq!(cancelled["status"], "cancelled");

    let second = post_json(
        router,
        "/v1/hermes/action",
        json!({
            "requester": {"channel": "cli", "user_id": "phase4-test"},
            "action": "Restart",
            "reason": "phase4 cancel test",
            "dry_run": false
        }),
    )
    .await;
    assert_eq!(second["status"], "confirmation_required");
    assert_eq!(
        confirmation_statuses(&fixture.state_db),
        vec!["cancelled".to_owned(), "pending".to_owned()]
    );
}

#[tokio::test]
async fn confirm_executes_pending_operation_through_injected_executor() {
    let fixture = Fixture::new();
    let executor = Arc::new(RecordingExecutor::default());
    let router = build_router_with_executor(&fixture.config_dir, TOKEN, executor.clone())
        .expect("router should build");

    let planned = post_json(
        router.clone(),
        "/v1/hermes/action",
        json!({
            "requester": {"channel": "cli", "user_id": "phase4-test"},
            "action": "Restart",
            "reason": "phase4 executor test",
            "dry_run": false
        }),
    )
    .await;
    assert_eq!(planned["status"], "confirmation_required");

    let confirmed = post_json(
        router,
        "/v1/confirm",
        json!({
            "requester": {"channel": "cli", "user_id": "phase4-test"},
            "code": planned["code_hint"]
        }),
    )
    .await;

    assert_eq!(confirmed["status"], "confirmed");
    let operations = executor.operations.lock().expect("executor lock");
    assert_eq!(operations.len(), 1);
    assert_eq!(operations[0].action, "hermes::Restart");
    assert_eq!(operations[0].requester_user_id, "phase4-test");
    assert_eq!(operations[0].commands.len(), 2);
    assert_eq!(
        operations[0].commands[0].args,
        [
            "--distribution",
            "Ubuntu-Hermes-Codex",
            "--user",
            "hermes",
            "--exec",
            "/home/hermes/Hermres/restart-services.sh"
        ]
    );
    assert_eq!(
        operation_statuses(&fixture.state_db),
        vec!["completed".to_owned()]
    );
}

#[tokio::test]
async fn confirm_response_exposes_failed_execution_outcome() {
    let fixture = Fixture::new();
    let router = build_router_with_executor(&fixture.config_dir, TOKEN, Arc::new(FailingExecutor))
        .expect("router should build");

    let planned = post_json(
        router.clone(),
        "/v1/wsl/action",
        json!({
            "requester": {"channel": "cli", "user_id": "phase4-test"},
            "action": "RestartDistro",
            "reason": "phase4 failed executor test",
            "dry_run": false
        }),
    )
    .await;
    assert_eq!(planned["status"], "confirmation_required");

    let confirmed = post_json(
        router.clone(),
        "/v1/confirm",
        json!({
            "requester": {"channel": "cli", "user_id": "phase4-test"},
            "code": planned["code_hint"]
        }),
    )
    .await;

    assert_eq!(confirmed["status"], "confirmed");
    assert_eq!(confirmed["execution_status"], "failed");
    assert!(
        confirmed["summary"]
            .as_str()
            .unwrap()
            .contains("simulated executor failure")
    );
    assert_eq!(
        operation_statuses(&fixture.state_db),
        vec!["failed".to_owned()]
    );

    let next = post_json(
        router,
        "/v1/hermes/action",
        json!({
            "requester": {"channel": "cli", "user_id": "phase4-test"},
            "action": "Restart",
            "reason": "phase4 lock release after failure",
            "dry_run": false
        }),
    )
    .await;
    assert_eq!(next["status"], "confirmation_required");
}

#[derive(Default)]
struct RecordingExecutor {
    operations: Mutex<Vec<ExecutableOperation>>,
}

impl OperationExecutor for RecordingExecutor {
    fn execute(&self, operation: &ExecutableOperation) -> ExecutionOutcome {
        self.operations
            .lock()
            .expect("executor lock")
            .push(operation.clone());
        ExecutionOutcome {
            status: "completed".to_owned(),
            summary: "recorded by test executor".to_owned(),
        }
    }
}

struct FailingExecutor;

impl OperationExecutor for FailingExecutor {
    fn execute(&self, _operation: &ExecutableOperation) -> ExecutionOutcome {
        ExecutionOutcome {
            status: "failed".to_owned(),
            summary: "simulated executor failure".to_owned(),
        }
    }
}

#[test]
fn windows_executor_runs_only_allowed_wsl_command_shapes() {
    let runner = Arc::new(RecordingRunner::default());
    let executor = WindowsCommandExecutor::new(runner.clone());
    let operation = ExecutableOperation {
        id: "op_test".to_owned(),
        confirmation_id: "confirm_test".to_owned(),
        action: "wsl::RestartDistro".to_owned(),
        requester_channel: "cli".to_owned(),
        requester_user_id: "phase4-test".to_owned(),
        summary: "Restart WSL distro Ubuntu-Hermes-Codex".to_owned(),
        commands: vec![
            command("wsl.exe", ["--terminate", "Ubuntu-Hermes-Codex"]),
            command(
                "wsl.exe",
                [
                    "--distribution",
                    "Ubuntu-Hermes-Codex",
                    "--user",
                    "hermes",
                    "--exec",
                    "true",
                ],
            ),
        ],
    };

    let outcome = executor.execute(&operation);

    assert_eq!(outcome.status, "completed", "{}", outcome.summary);
    assert_eq!(
        runner.commands.lock().expect("runner lock").clone(),
        operation.commands
    );
}

#[test]
fn windows_executor_rejects_non_allowlisted_programs_before_running_anything() {
    let runner = Arc::new(RecordingRunner::default());
    let executor = WindowsCommandExecutor::new(runner.clone());
    let operation = ExecutableOperation {
        id: "op_test".to_owned(),
        confirmation_id: "confirm_test".to_owned(),
        action: "hermes::Kill".to_owned(),
        requester_channel: "cli".to_owned(),
        requester_user_id: "phase4-test".to_owned(),
        summary: "Bad command".to_owned(),
        commands: vec![command(
            "powershell.exe",
            ["Stop-Process", "-Name", "python"],
        )],
    };

    let outcome = executor.execute(&operation);

    assert_eq!(outcome.status, "failed");
    assert!(outcome.summary.contains("not allowlisted"));
    assert!(runner.commands.lock().expect("runner lock").is_empty());
}

#[test]
fn windows_executor_rejects_unknown_wsl_argument_shapes() {
    let runner = Arc::new(RecordingRunner::default());
    let executor = WindowsCommandExecutor::new(runner.clone());
    let operation = ExecutableOperation {
        id: "op_test".to_owned(),
        confirmation_id: "confirm_test".to_owned(),
        action: "wsl::Bad".to_owned(),
        requester_channel: "cli".to_owned(),
        requester_user_id: "phase4-test".to_owned(),
        summary: "Bad WSL command".to_owned(),
        commands: vec![command("wsl.exe", ["--exec", "rm", "-rf", "/"])],
    };

    let outcome = executor.execute(&operation);

    assert_eq!(outcome.status, "failed");
    assert!(outcome.summary.contains("not allowlisted"));
    assert!(runner.commands.lock().expect("runner lock").is_empty());
}

#[test]
fn windows_executor_allows_fixed_hermes_wsl_scripts() {
    let runner = Arc::new(RecordingRunner::default());
    let executor = WindowsCommandExecutor::new(runner.clone());
    let operation = ExecutableOperation {
        id: "op_test".to_owned(),
        confirmation_id: "confirm_test".to_owned(),
        action: "hermes::Restart".to_owned(),
        requester_channel: "cli".to_owned(),
        requester_user_id: "phase4-test".to_owned(),
        summary: "Restart Hermes runtime".to_owned(),
        commands: vec![
            command(
                "wsl.exe",
                [
                    "--distribution",
                    "Ubuntu-Hermes-Codex",
                    "--user",
                    "hermes",
                    "--exec",
                    "/home/hermes/Hermres/restart-services.sh",
                ],
            ),
            command(
                "wsl.exe",
                [
                    "--distribution",
                    "Ubuntu-Hermes-Codex",
                    "--user",
                    "hermes",
                    "--exec",
                    "/home/hermes/Hermres/health-check.sh",
                    "30",
                    "ready",
                ],
            ),
        ],
    };

    let outcome = executor.execute(&operation);

    assert_eq!(outcome.status, "completed", "{}", outcome.summary);
    assert_eq!(
        runner.commands.lock().expect("runner lock").clone(),
        operation.commands
    );
}

#[test]
fn windows_executor_rejects_unknown_hermes_wsl_scripts() {
    let runner = Arc::new(RecordingRunner::default());
    let executor = WindowsCommandExecutor::new(runner.clone());
    let operation = ExecutableOperation {
        id: "op_test".to_owned(),
        confirmation_id: "confirm_test".to_owned(),
        action: "hermes::Bad".to_owned(),
        requester_channel: "cli".to_owned(),
        requester_user_id: "phase4-test".to_owned(),
        summary: "Bad Hermes command".to_owned(),
        commands: vec![command(
            "wsl.exe",
            [
                "--distribution",
                "Ubuntu-Hermes-Codex",
                "--user",
                "hermes",
                "--exec",
                "/home/hermes/Hermres/delete-everything.sh",
            ],
        )],
    };

    let outcome = executor.execute(&operation);

    assert_eq!(outcome.status, "failed");
    assert!(outcome.summary.contains("not allowlisted"));
    assert!(runner.commands.lock().expect("runner lock").is_empty());
}

#[derive(Default)]
struct RecordingRunner {
    commands: Mutex<Vec<hermes_control_types::CommandPreview>>,
}

impl CommandRunner for RecordingRunner {
    fn run(&self, command: &hermes_control_types::CommandPreview) -> CommandOutput {
        self.commands
            .lock()
            .expect("runner lock")
            .push(command.clone());
        CommandOutput {
            status_code: 0,
            stdout: "ok".to_owned(),
            stderr: String::new(),
        }
    }
}

fn command<const N: usize>(program: &str, args: [&str; N]) -> hermes_control_types::CommandPreview {
    hermes_control_types::CommandPreview {
        program: program.to_owned(),
        args: args.into_iter().map(ToOwned::to_owned).collect(),
    }
}

async fn post_json(router: Router, path: &str, body: Value) -> Value {
    let response = post_raw_json(router, path, body).await;
    assert_eq!(response.status, StatusCode::OK);
    serde_json::from_slice(&response.body).expect("response should be JSON")
}

struct TestResponse {
    status: StatusCode,
    body: Vec<u8>,
}

async fn post_raw_json(router: Router, path: &str, body: Value) -> TestResponse {
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
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body should collect");
    TestResponse {
        status,
        body: bytes.to_vec(),
    }
}

fn row_count(path: &Path, table: &str) -> i64 {
    let connection = Connection::open(path).expect("database should open");
    connection
        .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .expect("row count should read")
}

fn confirmation_statuses(path: &Path) -> Vec<String> {
    let connection = Connection::open(path).expect("database should open");
    let mut statement = connection
        .prepare("SELECT status FROM confirmations ORDER BY created_at, id")
        .expect("status query should prepare");
    statement
        .query_map([], |row| row.get::<_, String>(0))
        .expect("status query should run")
        .map(|row| row.expect("status should read"))
        .collect()
}

fn operation_statuses(path: &Path) -> Vec<String> {
    let connection = Connection::open(path).expect("database should open");
    let mut statement = connection
        .prepare("SELECT status FROM operation_state ORDER BY created_at, id")
        .expect("status query should prepare");
    statement
        .query_map([], |row| row.get::<_, String>(0))
        .expect("status query should run")
        .map(|row| row.expect("status should read"))
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
