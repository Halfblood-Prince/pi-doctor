use insta::assert_snapshot;
use pi_doctor_core::{CommandOutput, ProbeContext};
use serde_json::Value;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn help_output_snapshot() {
    let output = normalize(binary_output(&["--help"], &[0]));
    assert!(output.contains("Run the full diagnostic probe set"));
    assert!(output.contains("support-bundle"));
    assert!(output.contains("--json"));
    assert_snapshot!("help_output", output);
}

#[test]
fn check_json_snapshot() {
    let output = pi_doctor::output::render_report(
        &pi_doctor::build_check_report(&fixture_ctx("non-pi-debian")),
        pi_doctor::output::RenderSettings::test_json(),
    )
    .expect("fixture-backed json render should succeed");

    let parsed: Value = serde_json::from_str(&output).expect("output should be valid json");
    assert_eq!(parsed["schema_version"], "1.0.0");
    assert_eq!(parsed["overall_status"], "degraded");
    assert_eq!(parsed["metadata"]["command"], "check");
    assert!(parsed["probe_health"].is_array());
    assert!(
        parsed["findings"]
            .as_array()
            .expect("findings should be an array")
            .iter()
            .any(|finding| finding["id"] == "board.non_raspberry_pi")
    );
    assert!(
        parsed["findings"]
            .as_array()
            .expect("findings should be an array")
            .iter()
            .any(|finding| finding["id"] == "camera.tool_missing")
    );
}

#[test]
fn version_includes_package_version() {
    let output = normalize(binary_output(&["--version"], &[0]));
    assert!(output.contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn bash_completions_render() {
    let output = normalize(binary_output(&["completions", "bash"], &[0]));
    assert!(output.contains("pi-doctor"));
    assert!(output.contains("complete"));
}

fn binary_output(args: &[&str], allowed_exit_codes: &[i32]) -> String {
    let exe = env!("CARGO_BIN_EXE_pi-doctor");
    let output = Command::new(exe)
        .args(args)
        .output()
        .expect("pi-doctor binary should execute");

    let exit_code = output.status.code().unwrap_or_default();
    assert!(
        allowed_exit_codes.contains(&exit_code),
        "binary exited with unexpected status {exit_code}"
    );

    String::from_utf8(output.stdout).expect("binary output should be UTF-8")
}

fn normalize(output: String) -> String {
    let normalized = output.replace("pi-doctor.exe", "pi-doctor");
    let mut lines = normalized
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n");
    if normalized.ends_with('\n') {
        lines.push('\n');
    }
    lines
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
