use insta::assert_snapshot;
use pi_doctor_core::{
    Finding, FindingDomain, FindingGroup, Impact, OverallStatus, Report, ReportMetadata, Severity,
};
use pi_doctor_report::human::{RenderOptions, Verbosity};

#[test]
fn mixed_report_human_snapshot() {
    let report = mixed_report();
    assert_snapshot!(
        "mixed_report_human",
        pi_doctor_report::human::render(
            &report,
            RenderOptions {
                verbosity: Verbosity::Normal,
                color: false,
            }
        )
    );
}

#[test]
fn mixed_report_json_snapshot() {
    let report = mixed_report();
    let json = pi_doctor_report::json::render(&report).expect("report json should render");
    assert_snapshot!("mixed_report_json", json);
}

fn mixed_report() -> Report {
    let mut metadata = ReportMetadata::new("check");
    metadata.target_architecture = "test-arch".to_owned();

    let config_finding = finding(
        "config_txt.stale_legacy_path",
        Severity::Warning,
        Impact::Warning,
        "Legacy /boot/config.txt is present alongside the modern config path",
        "This system appears to use /boot/firmware/config.txt, but /boot/config.txt also exists.",
    );
    let thermal_finding = finding(
        "thermal.near_throttle",
        Severity::Warning,
        Impact::Warning,
        "CPU temperature is near throttling range",
        "CPU temperature is 78.2 C, which is close to the Raspberry Pi throttling threshold.",
    );
    let power_finding = finding(
        "throttling.undervoltage_now",
        Severity::Warning,
        Impact::Critical,
        "Under-voltage is active now",
        "Firmware telemetry reports an active under-voltage condition.",
    );

    Report {
        metadata,
        schema_version: "1.0.0",
        overall_status: OverallStatus::Critical,
        probe_health: Vec::new(),
        system: None,
        config: None,
        camera: None,
        python: None,
        groups: vec![
            FindingGroup {
                domain: FindingDomain::Power,
                findings: vec![power_finding.clone()],
            },
            FindingGroup {
                domain: FindingDomain::Thermal,
                findings: vec![thermal_finding.clone()],
            },
            FindingGroup {
                domain: FindingDomain::Config,
                findings: vec![config_finding.clone()],
            },
        ],
        findings: vec![power_finding, thermal_finding, config_finding],
    }
}

fn finding(
    id: &'static str,
    severity: Severity,
    impact: Impact,
    title: &str,
    summary: &str,
) -> Finding {
    Finding {
        id,
        severity,
        impact,
        title: title.to_owned(),
        summary: summary.to_owned(),
        evidence: Vec::new(),
        suggested_actions: Vec::new(),
    }
}
