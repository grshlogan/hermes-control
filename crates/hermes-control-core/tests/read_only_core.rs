use std::fs;

use hermes_control_core::{
    FixedProgram, KnownWslOperation, WslCommandSpec, build_model_readiness_from_models_response,
    build_wsl_models_endpoint, load_config_dir, models_response_has_model, parse_wsl_hostname_ips,
    parse_wsl_list_verbose, should_fallback_to_wsl_vllm_helper, tail_file_lines,
    vllm_health_command, vllm_helper_response_ready,
};

#[test]
fn loads_all_static_configs_from_directory() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::write(
        temp.path().join("control.toml"),
        include_str!("../../../config/control.toml"),
    )
    .expect("write control");
    fs::write(
        temp.path().join("providers.toml"),
        include_str!("../../../config/providers.toml"),
    )
    .expect("write providers");
    fs::write(
        temp.path().join("model-runtimes.toml"),
        include_str!("../../../config/model-runtimes.toml"),
    )
    .expect("write model runtimes");

    let loaded = load_config_dir(temp.path()).expect("config dir should load");

    assert_eq!(loaded.control.wsl.distro, "Ubuntu-Hermes-Codex");
    assert_eq!(loaded.providers.providers.len(), 2);
    assert_eq!(loaded.model_runtimes.runtimes[0].id, "vllm-local");
}

#[test]
fn parses_wsl_list_verbose_output_for_target_distro() {
    let output = r#"
  NAME                   STATE           VERSION
* Ubuntu-Hermes-Codex    Running         2
  docker-desktop         Stopped         2
"#;

    let distros = parse_wsl_list_verbose(output);

    let hermes = distros
        .iter()
        .find(|distro| distro.name == "Ubuntu-Hermes-Codex")
        .expect("target distro should be parsed");
    assert_eq!(hermes.state, "Running");
    assert_eq!(hermes.version, Some(2));
    assert!(hermes.default);
}

#[test]
fn wsl_list_verbose_command_uses_fixed_program_and_arguments() {
    let command = WslCommandSpec::new(KnownWslOperation::ListVerbose).to_command();

    assert_eq!(command.program, FixedProgram::WslExe);
    assert_eq!(command.args, ["--list", "--verbose"]);
}

#[test]
fn detects_expected_vllm_model_in_openai_models_response() {
    let body = r#"
{
  "object": "list",
  "data": [
    { "id": "qwen36-awq-int4", "object": "model" },
    { "id": "qwen36-mtp", "object": "model" }
  ]
}
"#;

    assert!(models_response_has_model(body, "qwen36-mtp").expect("valid json"));
    assert!(!models_response_has_model(body, "missing-model").expect("valid json"));
}

#[test]
fn builds_multiple_model_readiness_values_from_one_models_response() {
    let endpoint = "http://127.0.0.1:18080/v1/models";
    let body = r#"
{
  "object": "list",
  "data": [
    { "id": "qwen36-awq-int4", "object": "model" },
    { "id": "qwen36-mtp", "object": "model" }
  ]
}
"#;

    let results = build_model_readiness_from_models_response(
        endpoint,
        200,
        body,
        &["qwen36-awq-int4", "qwen36-mtp", "missing-model"],
    )
    .expect("valid models response");

    assert_eq!(results.len(), 3);
    assert_eq!(results[0].1, true);
    assert_eq!(results[1].1, true);
    assert_eq!(results[2].1, false);
    assert!(results.iter().all(|(status, _)| status.url == endpoint));
    assert!(results.iter().all(|(status, _)| status.reachable));
}

#[test]
fn parses_wsl_hostname_ips_for_vllm_endpoint_fallback() {
    let ips = parse_wsl_hostname_ips("10.2.176.55 172.18.0.1 \n");

    assert_eq!(ips, vec!["10.2.176.55".to_owned(), "172.18.0.1".to_owned()]);
}

#[test]
fn builds_wsl_models_endpoint_from_localhost_template() {
    assert_eq!(
        build_wsl_models_endpoint("http://127.0.0.1:18080/v1/models", "10.2.176.55"),
        Some("http://10.2.176.55:18080/v1/models".to_owned())
    );
    assert_eq!(
        build_wsl_models_endpoint("http://localhost:18080/v1/models", "10.2.176.55"),
        Some("http://10.2.176.55:18080/v1/models".to_owned())
    );
    assert_eq!(
        build_wsl_models_endpoint("http://192.168.1.5:18080/v1/models", "10.2.176.55"),
        None
    );
}

#[test]
fn detects_ready_vllm_helper_response() {
    let body = r#"
{
  "detail": "vLLM served model is ready.",
  "models_endpoint": "http://10.2.176.55:18080/v1/models",
  "ready": true,
  "served_model_name": "qwen36-mtp",
  "state": "ready"
}
"#;

    assert!(vllm_helper_response_ready(body, "qwen36-mtp").expect("valid json"));
    assert!(!vllm_helper_response_ready(body, "qwen36-awq-int4").expect("valid json"));
}

#[test]
fn skips_wsl_vllm_helper_when_runtime_endpoint_is_unreachable() {
    assert!(!should_fallback_to_wsl_vllm_helper(false, false));
    assert!(!should_fallback_to_wsl_vllm_helper(false, true));
    assert!(should_fallback_to_wsl_vllm_helper(true, false));
    assert!(!should_fallback_to_wsl_vllm_helper(true, true));
}

#[test]
fn vllm_health_command_checks_the_runtime_endpoint_inside_wsl() {
    let command = vllm_health_command(
        "Ubuntu-Hermes-Codex",
        "root",
        "http://10.2.176.55:18080/v1/models",
        "qwen36-mtp",
    );

    assert_eq!(command.program, FixedProgram::WslExe);
    assert_eq!(
        command.args,
        [
            "--distribution",
            "Ubuntu-Hermes-Codex",
            "--user",
            "root",
            "--exec",
            "/usr/bin/env",
            "HERMES_CONTROL_VLLM_MODELS_ENDPOINT_OVERRIDE=http://10.2.176.55:18080/v1/models",
            "/opt/hermes-control/bin/hermes-control-vllm-health.sh",
            "qwen36-mtp",
            "1",
            "ready"
        ]
    );
}

#[test]
fn tails_last_n_lines_from_log_file() {
    let temp = tempfile::tempdir().expect("tempdir");
    let log_path = temp.path().join("runtime.log");
    fs::write(&log_path, "one\ntwo\nthree\nfour\n").expect("write log");

    let lines = tail_file_lines(&log_path, 2).expect("tail log");

    assert_eq!(lines, vec!["three".to_owned(), "four".to_owned()]);
}
