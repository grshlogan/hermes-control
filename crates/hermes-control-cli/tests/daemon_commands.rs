use std::{
    io::{Read, Write},
    net::TcpListener,
    thread,
};

use clap::Parser;
use hermes_control_cli::{Cli, run_cli};

#[tokio::test]
async fn wsl_restart_posts_typed_action_to_daemon() {
    let server = OneShotHttpServer::new(
        r#"{"status":"dry_run","risk":"Destructive","summary":"Restart WSL distro Ubuntu-Hermes-Codex","dry_run":true,"commands":[]}"#,
    );
    let url = server.url();
    let cli = Cli::try_parse_from([
        "hermes-control",
        "--daemon-url",
        &url,
        "--api-token",
        "phase4-cli-token",
        "wsl",
        "restart",
        "--dry-run",
        "--reason",
        "phase4 CLI smoke",
    ])
    .expect("CLI args should parse");

    let rendered = run_cli(cli).await.expect("CLI command should run");
    let request = server.join();

    assert!(request.starts_with("POST /v1/wsl/action HTTP/1.1"));
    assert!(
        request
            .to_ascii_lowercase()
            .contains("authorization: bearer phase4-cli-token")
    );
    assert!(request.contains(r#""channel":"cli""#));
    assert!(request.contains(r#""action":"RestartDistro""#));
    assert!(request.contains(r#""reason":"phase4 CLI smoke""#));
    assert!(request.contains(r#""dry_run":true"#));
    assert!(rendered.contains("dry_run"));
    assert!(rendered.contains("Restart WSL distro Ubuntu-Hermes-Codex"));
}

#[tokio::test]
async fn hermes_kill_posts_typed_action_to_daemon() {
    let server = OneShotHttpServer::new(
        r#"{"status":"confirmation_required","risk":"Destructive","summary":"Kill Hermes runtime","dry_run":false,"commands":[],"confirmation_id":"confirm_test","code_hint":"HERMES-1234","expires_at":"2026-05-02T13:00:00Z"}"#,
    );
    let url = server.url();
    let cli = Cli::try_parse_from([
        "hermes-control",
        "--daemon-url",
        &url,
        "--api-token",
        "phase4-cli-token",
        "hermes",
        "kill",
        "--reason",
        "phase4 CLI smoke",
    ])
    .expect("CLI args should parse");

    let rendered = run_cli(cli).await.expect("CLI command should run");
    let request = server.join();

    assert!(request.starts_with("POST /v1/hermes/action HTTP/1.1"));
    assert!(request.contains(r#""action":"Kill""#));
    assert!(request.contains(r#""dry_run":false"#));
    assert!(rendered.contains("confirmation_required"));
    assert!(rendered.contains("HERMES-1234"));
}

#[tokio::test]
async fn confirm_posts_code_to_daemon_confirmation_endpoint() {
    let server = OneShotHttpServer::new(
        r#"{"status":"confirmed","confirmation_id":"confirm_test","summary":"Restart done","execution_status":"completed"}"#,
    );
    let url = server.url();
    let cli = Cli::try_parse_from([
        "hermes-control",
        "--daemon-url",
        &url,
        "--api-token",
        "phase4-cli-token",
        "confirm",
        "HERMES-1234",
    ])
    .expect("CLI args should parse");

    let rendered = run_cli(cli).await.expect("CLI command should run");
    let request = server.join();

    assert!(request.starts_with("POST /v1/confirm HTTP/1.1"));
    assert!(request.contains(r#""code":"HERMES-1234""#));
    assert!(rendered.contains("confirmed"));
    assert!(rendered.contains("completed"));
}

#[tokio::test]
async fn model_start_posts_typed_action_to_daemon() {
    let server = OneShotHttpServer::new(
        r#"{"status":"dry_run","risk":"NormalMutating","summary":"Start vLLM model qwen36-mtp","dry_run":true,"commands":[]}"#,
    );
    let url = server.url();
    let cli = Cli::try_parse_from([
        "hermes-control",
        "--daemon-url",
        &url,
        "--api-token",
        "phase5-cli-token",
        "model",
        "start",
        "qwen36-mtp",
        "--dry-run",
        "--reason",
        "phase5 CLI smoke",
    ])
    .expect("CLI args should parse");

    let rendered = run_cli(cli).await.expect("CLI command should run");
    let request = server.join();

    assert!(request.starts_with("POST /v1/models/qwen36-mtp/action HTTP/1.1"));
    assert!(request.contains(r#""action":"Start""#));
    assert!(request.contains(r#""reason":"phase5 CLI smoke""#));
    assert!(request.contains(r#""dry_run":true"#));
    assert!(rendered.contains("dry_run"));
    assert!(rendered.contains("Start vLLM model qwen36-mtp"));
}

#[tokio::test]
async fn model_install_posts_typed_action_to_daemon() {
    let server = OneShotHttpServer::new(
        r#"{"status":"dry_run","risk":"NormalMutating","summary":"Install or repair vLLM runtime","dry_run":true,"commands":[]}"#,
    );
    let url = server.url();
    let cli = Cli::try_parse_from([
        "hermes-control",
        "--daemon-url",
        &url,
        "--api-token",
        "phase5-cli-token",
        "model",
        "install",
        "qwen36-mtp",
        "--dry-run",
        "--reason",
        "phase5 install smoke",
    ])
    .expect("CLI args should parse");

    let rendered = run_cli(cli).await.expect("CLI command should run");
    let request = server.join();

    assert!(request.starts_with("POST /v1/models/qwen36-mtp/action HTTP/1.1"));
    assert!(request.contains(r#""action":"Install""#));
    assert!(request.contains(r#""reason":"phase5 install smoke""#));
    assert!(request.contains(r#""dry_run":true"#));
    assert!(rendered.contains("Install or repair vLLM runtime"));
}

#[tokio::test]
async fn model_logs_posts_typed_action_to_daemon_and_renders_output() {
    let server = OneShotHttpServer::new(
        r#"{"status":"completed","risk":"ReadOnly","summary":"Executed 1 allowlisted command(s) for op_logs.","dry_run":false,"commands":[],"output":"tail line 1\ntail line 2\n"}"#,
    );
    let url = server.url();
    let cli = Cli::try_parse_from([
        "hermes-control",
        "--daemon-url",
        &url,
        "--api-token",
        "phase5-cli-token",
        "model",
        "logs",
        "qwen36-mtp",
        "--reason",
        "phase5 logs smoke",
    ])
    .expect("CLI args should parse");

    let rendered = run_cli(cli).await.expect("CLI command should run");
    let request = server.join();

    assert!(request.starts_with("POST /v1/models/qwen36-mtp/action HTTP/1.1"));
    assert!(request.contains(r#""action":"Logs""#));
    assert!(request.contains(r#""reason":"phase5 logs smoke""#));
    assert!(request.contains(r#""dry_run":false"#));
    assert!(rendered.contains("tail line 1"));
    assert!(rendered.contains("tail line 2"));
}

#[tokio::test]
async fn route_switch_posts_typed_request_to_daemon() {
    let server = OneShotHttpServer::new(
        r#"{"status":"completed","risk":"NormalMutating","summary":"Switched active route to external.test.","dry_run":false,"commands":[]}"#,
    );
    let url = server.url();
    let cli = Cli::try_parse_from([
        "hermes-control",
        "--daemon-url",
        &url,
        "--api-token",
        "phase6-cli-token",
        "route",
        "switch",
        "external.test",
        "--reason",
        "phase6 route smoke",
    ])
    .expect("CLI args should parse");

    let rendered = run_cli(cli).await.expect("CLI command should run");
    let request = server.join();

    assert!(request.starts_with("POST /v1/route/switch HTTP/1.1"));
    assert!(request.contains(r#""profile_id":"external.test""#));
    assert!(request.contains(r#""reason":"phase6 route smoke""#));
    assert!(request.contains(r#""dry_run":false"#));
    assert!(rendered.contains("Switched active route to external.test"));
}

struct OneShotHttpServer {
    address: String,
    handle: thread::JoinHandle<String>,
}

impl OneShotHttpServer {
    fn new(response_body: &'static str) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("test server should bind");
        let address = listener.local_addr().expect("local addr").to_string();
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("request should arrive");
            let request = read_http_request(&mut stream);
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                response_body.len(),
                response_body
            );
            stream
                .write_all(response.as_bytes())
                .expect("response should write");
            request
        });

        Self { address, handle }
    }

    fn url(&self) -> String {
        format!("http://{}", self.address)
    }

    fn join(self) -> String {
        self.handle.join().expect("server thread should finish")
    }
}

fn read_http_request(stream: &mut impl Read) -> String {
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 1024];

    loop {
        let read = stream.read(&mut buffer).expect("request should read");
        if read == 0 {
            break;
        }
        bytes.extend_from_slice(&buffer[..read]);
        if bytes.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
    }

    let header_end = bytes
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .expect("headers should complete")
        + 4;
    let headers = String::from_utf8_lossy(&bytes[..header_end]).to_string();
    let content_length = headers
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().ok())
                .flatten()
        })
        .unwrap_or(0);

    while bytes.len().saturating_sub(header_end) < content_length {
        let read = stream.read(&mut buffer).expect("body should read");
        if read == 0 {
            break;
        }
        bytes.extend_from_slice(&buffer[..read]);
    }

    String::from_utf8(bytes).expect("request should be utf8")
}
