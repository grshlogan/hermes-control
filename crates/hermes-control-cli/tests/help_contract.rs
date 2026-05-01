use clap::CommandFactory;
use hermes_control_cli::Cli;

#[test]
fn help_lists_phase_one_command_surface() {
    let mut buffer = Vec::new();
    Cli::command()
        .write_long_help(&mut buffer)
        .expect("help should render");

    let help = String::from_utf8(buffer).expect("help should be valid utf8");

    for expected in [
        "status",
        "health",
        "providers",
        "route",
        "models",
        "model",
        "wsl",
        "--json",
    ] {
        assert!(
            help.contains(expected),
            "CLI help should mention {expected:?}.\n{help}"
        );
    }
}
