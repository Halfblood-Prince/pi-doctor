use insta::assert_snapshot;
use pi_doctor_core::{CommandOutput, ProbeContext};
use std::path::PathBuf;

#[test]
fn modern_layout_fixture_populates_config_summary_and_findings() {
    let report = pi_doctor::build_check_report(&fixture_ctx("modern-layout"));
    let config = report.config.expect("config summary should be present");

    assert_eq!(
        config.source_path.as_deref(),
        Some("/boot/firmware/config.txt")
    );
    assert!(config.using_firmware_path);
    assert!(config.legacy_path_present);
    assert_eq!(config.diagnostics_count, 4);
    assert!(config.entries.iter().any(|entry| entry.line_number == 3));
}

#[test]
fn modern_layout_fixture_emits_expected_config_findings() {
    let report = pi_doctor::build_check_report(&fixture_ctx("modern-layout"));

    let ids = report
        .findings
        .iter()
        .map(|finding| finding.id)
        .collect::<Vec<_>>();

    assert!(!ids.contains(&"config_txt.duplicate_dtoverlay"));
    assert!(ids.contains(&"config_txt.conflicting_dtparam"));
    assert!(ids.contains(&"config_txt.legacy_option"));
    assert!(ids.contains(&"config_txt.malformed_line"));
    assert!(ids.contains(&"config_txt.stale_legacy_path"));
}

#[test]
fn explain_config_snapshot() {
    let output = pi_doctor::explain::config::render(&fixture_ctx("modern-layout"));
    assert!(output.contains("source path: /boot/firmware/config.txt"));
    assert!(output.contains("Potential conflicting `i2c_arm` dtparam entries detected."));
    assert!(output.contains("Malformed config.txt line detected."));
    assert_snapshot!("explain_config_modern_layout", output);
}

fn fixture_ctx(name: &str) -> ProbeContext {
    ProbeContext::with_root(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("fixtures")
            .join("milestone3")
            .join(name),
    )
    .with_command_output("vcgencmd", &["get_throttled"], CommandOutput::Missing)
    .with_command_output("rpicam-hello", &["--help"], CommandOutput::Missing)
    .with_command_output("libcamera-hello", &["--help"], CommandOutput::Missing)
    .with_command_output("python3", &["--version"], CommandOutput::Missing)
}
