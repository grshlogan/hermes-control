use std::fs;

use hermes_control_core::parse_model_runtimes_config;

#[test]
fn model_runtime_config_points_to_project_owned_vllm_runtime() {
    let config = parse_model_runtimes_config(include_str!("../../../config/model-runtimes.toml"))
        .expect("model runtime config should parse");
    let runtime = &config.runtimes[0];

    assert_eq!(runtime.workspace, "E:\\WSL\\Hermres\\hermes-control\\vLLM");
    assert_eq!(
        runtime.log_dir,
        "E:\\WSL\\Hermres\\hermes-control\\vLLM\\logs"
    );

    for variant in &runtime.variants {
        let script = variant.start.script.as_deref().expect("start script");
        assert!(
            script.starts_with("/mnt/e/WSL/Hermres/hermes-control/vLLM/scripts/"),
            "start script should live under project-owned runtime: {script}"
        );
        assert!(
            !script.starts_with("/mnt/e/WSL/vLLM/scripts/"),
            "start script must not point to the old shared vLLM workspace: {script}"
        );
    }
}

#[test]
fn project_vllm_runtime_assets_preserve_external_model_store() {
    let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..");
    let script_root = repo_root.join("vLLM").join("scripts");

    for script in [
        "env.sh",
        "bootstrap.sh",
        "serve-openai.sh",
        "start-qwen36-mtp.sh",
        "start-qwen36-int4-eager.sh",
    ] {
        let path = script_root.join(script);
        let contents = fs::read_to_string(&path).unwrap_or_else(|error| {
            panic!("{} should exist: {error}", path.display());
        });
        assert!(
            contents.starts_with("#!/usr/bin/env bash"),
            "{} should be a bash script",
            path.display()
        );
    }

    let env_contents = fs::read_to_string(script_root.join("env.sh")).expect("env.sh should read");
    assert!(env_contents.contains("VLLM_WORKSPACE=\"/mnt/e/WSL/Hermres/hermes-control/vLLM\""));
    assert!(env_contents.contains("VLLM_MODEL_ROOT=\"/mnt/e/WSL/vLLM/models\""));
    assert!(!env_contents.contains("VLLM_WORKSPACE=\"/mnt/e/WSL/vLLM\""));
}

#[test]
fn wsl_root_helpers_default_to_project_owned_vllm_runtime() {
    let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..");
    let helper_root = repo_root.join("scripts").join("wsl-root");

    for script in [
        helper_root.join("install.sh"),
        helper_root.join("bin").join("hermes-control-common.sh"),
    ] {
        let contents = fs::read_to_string(&script).unwrap_or_else(|error| {
            panic!("{} should read: {error}", script.display());
        });
        assert!(
            contents.contains("/mnt/e/WSL/Hermres/hermes-control/vLLM"),
            "{} should default to the project-owned vLLM runtime",
            script.display()
        );
        assert!(
            contents.contains("/mnt/e/WSL/vLLM/models"),
            "{} should keep the external model-weight store",
            script.display()
        );
        assert!(
            !contents.contains("VLLM_WORKSPACE=/mnt/e/WSL/vLLM\n"),
            "{} must not default the runtime workspace to the old shared vLLM directory",
            script.display()
        );
    }
}

#[test]
fn wsl_vllm_health_checks_bypass_proxy_for_localhost() {
    let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..");
    let common = repo_root
        .join("scripts")
        .join("wsl-root")
        .join("bin")
        .join("hermes-control-common.sh");

    let contents = fs::read_to_string(&common).expect("common helper should read");
    assert!(
        contents.contains("curl --noproxy '*'"),
        "local health checks must bypass configured proxies"
    );
    assert!(
        contents.contains("VLLM_MODELS_BODY=\"$body\""),
        "vLLM health parsing should pass the models response body to Python"
    );
    assert!(
        contents.contains("json.loads(os.environ[\"VLLM_MODELS_BODY\"])"),
        "vLLM health parsing should not read JSON from the heredoc stdin"
    );
}

#[test]
fn vllm_start_scripts_bind_to_wsl_primary_ip_by_default() {
    let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..");
    let script_root = repo_root.join("vLLM").join("scripts");

    for script in ["start-qwen36-mtp.sh", "start-qwen36-int4-eager.sh"] {
        let path = script_root.join(script);
        let contents = fs::read_to_string(&path).unwrap_or_else(|error| {
            panic!("{} should read: {error}", path.display());
        });
        assert!(
            contents.contains("hostname -I"),
            "{} should resolve the WSL primary IP for vLLM clients",
            path.display()
        );
        assert!(
            contents.contains("export HOST=\"${HOST:-$DEFAULT_HOST}\""),
            "{} should bind vLLM to the WSL primary IP unless explicitly overridden",
            path.display()
        );
    }
}

#[test]
fn wsl_vllm_health_endpoint_can_be_resolved_at_runtime() {
    let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..");
    let common = repo_root
        .join("scripts")
        .join("wsl-root")
        .join("bin")
        .join("hermes-control-common.sh");
    let installer = repo_root
        .join("scripts")
        .join("wsl-root")
        .join("install.sh");

    let common_contents = fs::read_to_string(&common).expect("common helper should read");
    assert!(common_contents.contains("VLLM_MODELS_ENDPOINT:=auto"));
    assert!(common_contents.contains("hc_resolve_vllm_models_endpoint"));
    assert!(common_contents.contains("hostname -I"));

    let install_contents = fs::read_to_string(&installer).expect("installer should read");
    assert!(install_contents.contains("VLLM_MODELS_ENDPOINT=auto"));
    assert!(install_contents.contains("VLLM_PORT=18080"));
}
