use pi_doctor::cli::args::{Cli, Commands};
use pi_doctor::output::RenderSettings;
use pi_doctor_core::{CommandOutput, ProbeContext};
use serde_json::Value;
use std::path::PathBuf;

#[test]
fn json_contract_exposes_expected_top_level_fields() {
    let output = pi_doctor::output::render_report(
        &pi_doctor::build_check_report(&fixture_ctx("non-pi-debian")),
        RenderSettings::test_json(),
    )
    .expect("fixture-backed json render should succeed");

    assert!(output.ends_with('\n'));

    let value: Value = serde_json::from_str(&output).expect("json should parse");
    let object = value.as_object().expect("top level should be an object");

    for key in [
        "metadata",
        "schema_version",
        "overall_status",
        "system",
        "config",
        "camera",
        "python",
        "groups",
        "findings",
    ] {
        assert!(object.contains_key(key), "missing top-level key `{key}`");
    }

    assert_eq!(value["schema_version"], "1.0.0");
}

#[test]
fn non_check_commands_return_zero_on_success() {
    for command in [
        Commands::SupportBundle,
        Commands::Completions {
            shell: clap_complete::Shell::Bash,
        },
    ] {
        let response = pi_doctor::run(Cli {
            json: false,
            quiet: false,
            verbose: false,
            no_color: true,
            command,
        })
        .expect("command should succeed");

        assert_eq!(response.exit_code, 0);
    }
}

fn fixture_ctx(name: &str) -> ProbeContext {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join("milestone1")
        .join(name);

    ProbeContext::with_root(root)
        .with_command_output("vcgencmd", &["get_throttled"], CommandOutput::Missing)
        .with_command_output("rpicam-hello", &["--help"], CommandOutput::Missing)
        .with_command_output("libcamera-hello", &["--help"], CommandOutput::Missing)
        .with_command_output("python3", &["--version"], CommandOutput::Missing)
        .with_command_output(
            "python3",
            &["-c", "import sys; print(sys.executable)"],
            CommandOutput::Missing,
        )
        .with_command_output(
            "python3",
            &[
                "-c",
                "import sys; print(int(sys.prefix != sys.base_prefix))",
            ],
            CommandOutput::Missing,
        )
        .with_command_output(
            "python3",
            &[
                "-c",
                "import sysconfig; print(sysconfig.get_path('stdlib'))",
            ],
            CommandOutput::Missing,
        )
        .with_command_output(
            "dpkg-query",
            &["-W", "-f=${Status}", "python3-picamera2"],
            CommandOutput::Missing,
        )
        .with_command_output(
            "dpkg-query",
            &["-W", "-f=${Status}", "python3-gpiozero"],
            CommandOutput::Missing,
        )
}
