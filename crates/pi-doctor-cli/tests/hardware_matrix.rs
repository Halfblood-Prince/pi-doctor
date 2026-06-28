use pi_doctor_core::{CommandOutput, OverallStatus, ProbeContext, ProbeOutcome};
use std::fs;
use std::path::PathBuf;

#[test]
fn pi4_lite_no_camera_fixture_reports_camera_problem() {
    let root = fixture_root("pi4-bookworm-lite-no-camera");
    let ctx = hardware_context(root, "pi4-bookworm-lite-no-camera")
        .with_command_output(
            "rpicam-hello",
            &["--list-cameras"],
            CommandOutput::Success(capture(
                "pi4-bookworm-lite-no-camera",
                "rpicam-hello-list-cameras.txt",
            )),
        )
        .with_command_output("libcamera-hello", &["--list-cameras"], CommandOutput::Missing);

    let report = pi_doctor::build_check_report(&ctx);

    assert_eq!(report.overall_status, OverallStatus::Degraded);
    let system = report.system.expect("report should include system summary");
    assert_eq!(
        system.board_model.as_deref(),
        Some("Raspberry Pi 4 Model B Rev 1.5")
    );
    assert_eq!(system.distro_codename.as_deref(), Some("bookworm"));
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.id == "camera.no_cameras_detected")
    );
    assert!(
        report
            .probe_health
            .iter()
            .any(|health| health.name == "camera" && health.outcome == ProbeOutcome::Success)
    );
}

#[test]
fn stressed_pi5_fixture_rolls_up_to_critical() {
    let root = fixture_root("pi5-stressed-lab-rig");
    let ctx = hardware_context(root, "pi5-stressed-lab-rig");

    let report = pi_doctor::build_check_report(&ctx);

    assert_eq!(report.overall_status, OverallStatus::Critical);
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.id == "thermal.throttling_likely")
    );
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.id == "throttling.active")
    );
}

fn hardware_context(root: PathBuf, fixture: &str) -> ProbeContext {
    ProbeContext::with_root(root)
        .with_command_output(
            "vcgencmd",
            &["get_throttled"],
            CommandOutput::Success(capture(fixture, "vcgencmd-get_throttled.txt")),
        )
        .with_command_output("rpicam-hello", &["--list-cameras"], CommandOutput::Missing)
        .with_command_output(
            "libcamera-hello",
            &["--list-cameras"],
            CommandOutput::Missing,
        )
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
