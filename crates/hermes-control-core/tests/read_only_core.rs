use std::fs;

use hermes_control_core::{
    FixedProgram, KnownWslOperation, WslCommandSpec, load_config_dir, models_response_has_model,
    parse_wsl_list_verbose, tail_file_lines,
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
fn tails_last_n_lines_from_log_file() {
    let temp = tempfile::tempdir().expect("tempdir");
    let log_path = temp.path().join("runtime.log");
    fs::write(&log_path, "one\ntwo\nthree\nfour\n").expect("write log");

    let lines = tail_file_lines(&log_path, 2).expect("tail log");

    assert_eq!(lines, vec!["three".to_owned(), "four".to_owned()]);
}
