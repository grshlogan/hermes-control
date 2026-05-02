use std::fs;

#[test]
fn wsl_root_install_assets_define_canonical_helper_contract() {
    let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..");
    let script_root = repo_root.join("scripts").join("wsl-root");
    let install = script_root.join("install.sh");
    let bin = script_root.join("bin");

    let install_contents = fs::read_to_string(&install).expect("install.sh should exist");
    assert!(install_contents.contains("INSTALL_PREFIX=\"/opt/hermes-control\""));
    assert!(install_contents.contains("/etc/hermes-control/runtime.env"));

    for script in [
        "hermes-control-common.sh",
        "hermes-control-start.sh",
        "hermes-control-stop.sh",
        "hermes-control-restart.sh",
        "hermes-control-kill.sh",
        "hermes-control-health.sh",
        "hermes-control-status.sh",
    ] {
        let path = bin.join(script);
        let contents = fs::read_to_string(&path).unwrap_or_else(|error| {
            panic!("{} should exist: {error}", path.display());
        });
        assert!(
            contents.starts_with("#!/usr/bin/env bash"),
            "{} should be a bash script",
            path.display()
        );
    }
}
