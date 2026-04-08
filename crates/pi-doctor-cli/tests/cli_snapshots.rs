use insta::assert_snapshot;
use pi_doctor_core::{CommandOutput, ProbeContext};
use std::path::PathBuf;
use std::process::Command;

#[test]
fn help_output_snapshot() {
    assert_snapshot!("help_output", normalize(binary_output(&["--help"], &[0])));
}

#[test]
fn check_json_snapshot() {
    let output = pi_doctor::output::render_report(
        &pi_doctor::build_check_report(&fixture_ctx("non-pi-debian")),
        pi_doctor::output::RenderSettings::test_json(),
    )
    .expect("fixture-backed json render should succeed");

    assert_snapshot!("check_json_output", output);
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
    output.replace("pi-doctor.exe", "pi-doctor")
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
            &["-c", "import sys; print(int(sys.prefix != sys.base_prefix))"],
            CommandOutput::Missing,
        )
        .with_command_output(
            "python3",
            &["-c", "import sysconfig; print(sysconfig.get_path('stdlib'))"],
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
