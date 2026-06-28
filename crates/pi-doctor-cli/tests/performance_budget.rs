use pi_doctor_core::{CommandOutput, ProbeContext};
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

#[test]
fn check_fixture_stays_within_runtime_budget() {
    let ctx = fixture_context("pi5-bookworm-desktop-camera");
    let started = Instant::now();
    let report = pi_doctor::build_check_report(&ctx);
    let elapsed = started.elapsed();

    assert!(
        elapsed.as_secs_f32() < 5.0,
        "fixture-backed check took {elapsed:?}"
    );
    assert_eq!(
        report.metadata.probe_availability.total,
        report.probe_health.len()
    );
}

fn fixture_context(name: &str) -> ProbeContext {
    let root = fixture_root(name);
    ProbeContext::with_root(root)
        .with_command_output(
            "vcgencmd",
            &["get_throttled"],
            CommandOutput::Success(capture(name, "vcgencmd-get_throttled.txt")),
        )
        .with_command_output(
            "rpicam-hello",
            &["--list-cameras"],
            CommandOutput::Success(capture(name, "rpicam-hello-list-cameras.txt")),
        )
        .with_command_output(
            "libcamera-hello",
            &["--list-cameras"],
            CommandOutput::Missing,
        )
        .with_command_output(
            "python3",
            &["--version"],
            CommandOutput::Success(capture(name, "python-version.txt")),
        )
        .with_command_output(
            "python3",
            &["-c", "import sys; print(sys.executable)"],
            CommandOutput::Success(capture(name, "python-executable.txt")),
        )
        .with_command_output(
            "python3",
            &[
                "-c",
                "import sys; print(int(sys.prefix != sys.base_prefix))",
            ],
            CommandOutput::Success(capture(name, "python-venv-flag.txt")),
        )
        .with_command_output(
            "python3",
            &[
                "-c",
                "import sysconfig; print(sysconfig.get_path('stdlib'))",
            ],
            CommandOutput::Success(capture(name, "python-stdlib.txt")),
        )
        .with_command_output(
            "dpkg-query",
            &["-W", "-f=${Status}", "python3-picamera2"],
            CommandOutput::Success(capture(name, "dpkg-picamera2.txt")),
        )
        .with_command_output(
            "dpkg-query",
            &["-W", "-f=${Status}", "python3-gpiozero"],
            CommandOutput::Success(capture(name, "dpkg-gpiozero.txt")),
        )
}

fn fixture_root(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join("hardware-matrix")
        .join(name)
}

fn capture(fixture: &str, file: &str) -> String {
    fs::read_to_string(fixture_root(fixture).join("captures").join(file))
        .expect("fixture capture should exist")
}
