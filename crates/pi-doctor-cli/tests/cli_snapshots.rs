use insta::assert_snapshot;
use std::process::Command;

#[test]
fn help_output_snapshot() {
    assert_snapshot!("help_output", normalize(binary_output(&["--help"], &[0])));
}

#[test]
fn check_json_snapshot() {
    assert_snapshot!(
        "check_json_output",
        binary_output(&["--json", "check"], &[1])
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
    output.replace("pi-doctor.exe", "pi-doctor")
}
