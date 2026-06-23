use pi_doctor_core::{CommandOutput, OverallStatus, ProbeContext};
use std::fs;
use std::path::PathBuf;

#[test]
fn pi4_lite_fixture_builds_clean_report() {
    let root = fixture_root("pi4-bookworm-lite-no-camera");
    let ctx = ProbeContext::with_root(root).with_command_output(
        "vcgencmd",
        &["get_throttled"],
        CommandOutput::Success(capture(
            "pi4-bookworm-lite-no-camera",
            "vcgencmd-get_throttled.txt",
        )),
    );

    let report = pi_doctor::build_check_report(&ctx);

    assert_eq!(report.overall_status, OverallStatus::Healthy);
    let system = report.system.expect("report should include system summary");
    assert_eq!(
        system.board_model.as_deref(),
        Some("Raspberry Pi 4 Model B Rev 1.5")
    );
    assert_eq!(system.distro_codename.as_deref(), Some("bookworm"));
    assert!(report.findings.is_empty());
}

#[test]
fn stressed_pi5_fixture_rolls_up_to_degraded() {
    let root = fixture_root("pi5-stressed-lab-rig");
    let ctx = ProbeContext::with_root(root).with_command_output(
        "vcgencmd",
        &["get_throttled"],
        CommandOutput::Success(capture(
            "pi5-stressed-lab-rig",
            "vcgencmd-get_throttled.txt",
        )),
    );

    let report = pi_doctor::build_check_report(&ctx);

    assert_eq!(report.overall_status, OverallStatus::Degraded);
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
