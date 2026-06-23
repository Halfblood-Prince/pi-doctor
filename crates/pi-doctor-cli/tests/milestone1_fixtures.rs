use pi_doctor_core::{CommandOutput, ProbeContext, Severity};
use std::path::PathBuf;

#[test]
fn pi4_bookworm_fixture_is_detected_as_raspberry_pi() {
    let report = fixture_report("pi4-bookworm");
    let system = report
        .system
        .expect("check report should include system summary");

    assert_eq!(
        system.board_model.as_deref(),
        Some("Raspberry Pi 4 Model B Rev 1.5")
    );
    assert_eq!(system.board_revision.as_deref(), Some("c03115"));
    assert_eq!(system.architecture.as_deref(), Some("aarch64"));
    assert_eq!(system.distro_name.as_deref(), Some("Debian GNU/Linux"));
    assert_eq!(system.distro_version.as_deref(), Some("12"));
    assert_eq!(system.distro_codename.as_deref(), Some("bookworm"));
    assert_eq!(system.kernel_release.as_deref(), Some("6.6.31-v8+"));
    assert!(system.is_raspberry_pi);
    assert!(report.findings.is_empty());
}

#[test]
fn pi5_trixie_fixture_is_detected_as_raspberry_pi() {
    let report = fixture_report("pi5-trixie");
    let system = report
        .system
        .expect("check report should include system summary");

    assert_eq!(
        system.board_model.as_deref(),
        Some("Raspberry Pi 5 Model B Rev 1.0")
    );
    assert_eq!(system.board_revision.as_deref(), Some("d04170"));
    assert_eq!(system.architecture.as_deref(), Some("aarch64"));
    assert_eq!(system.distro_version.as_deref(), Some("13"));
    assert_eq!(system.distro_codename.as_deref(), Some("trixie"));
    assert_eq!(system.kernel_release.as_deref(), Some("6.12.25-v8-16k+"));
    assert!(system.is_raspberry_pi);
    assert!(report.findings.is_empty());
}

#[test]
fn non_pi_fixture_emits_warning_instead_of_failing() {
    let report = fixture_report("non-pi-debian");
    let system = report
        .system
        .expect("check report should include system summary");

    assert_eq!(system.architecture.as_deref(), Some("x86_64"));
    assert_eq!(system.distro_codename.as_deref(), Some("bookworm"));
    assert_eq!(system.kernel_release.as_deref(), Some("6.1.0-27-amd64"));
    assert!(!system.is_raspberry_pi);
    assert_eq!(report.findings.len(), 1);
    assert_eq!(report.findings[0].severity, Severity::Warning);
    assert_eq!(report.findings[0].id, "board.non_raspberry_pi");
}

fn fixture_report(name: &str) -> pi_doctor_core::Report {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join("milestone1")
        .join(name);
    let ctx = ProbeContext::with_root(root).with_command_output(
        "vcgencmd",
        &["get_throttled"],
        CommandOutput::Success("throttled=0x0".to_owned()),
    );

    pi_doctor::build_check_report(&ctx)
}
