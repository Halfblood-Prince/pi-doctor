use pi_doctor_core::{
    CameraDevice, CameraSummary, ConfigEntry, ConfigEntryKind, ConfigSummary, Finding,
    FindingDomain, FindingGroup, Impact, OverallStatus, ProbeHealth, ProbeOutcome, PythonSummary,
    Report, ReportMetadata, Severity, SystemSummary,
};
use serde_json::Value;

#[test]
fn emitted_json_matches_documented_schema_structure() {
    let json = pi_doctor_report::json::render(&sample_report()).expect("report json should render");
    let value: Value = serde_json::from_str(&json).expect("rendered report should be valid json");
    let object = value
        .as_object()
        .expect("top level should be a JSON object");

    let mut top_level_keys = object.keys().map(String::as_str).collect::<Vec<_>>();
    top_level_keys.sort_unstable();
    assert_eq!(
        top_level_keys,
        vec![
            "camera",
            "config",
            "findings",
            "groups",
            "metadata",
            "overall_status",
            "probe_health",
            "python",
            "schema_version",
            "system",
        ]
    );

    assert_eq!(value["metadata"]["command"], "check");
    assert_eq!(value["metadata"]["pi_doctor_version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(value["metadata"]["supported_os"]["supported"], true);
    assert_eq!(value["metadata"]["probe_availability"]["total"], 1);
    assert_eq!(value["schema_version"], "1.0.0");
    assert_eq!(value["overall_status"], "critical");
    assert_eq!(value["probe_health"][0]["name"], "board");
    assert_eq!(value["probe_health"][0]["outcome"], "success");

    assert_eq!(value["system"]["board_model"], "Raspberry Pi 5");
    assert_eq!(value["system"]["is_raspberry_pi"], true);

    assert_eq!(value["config"]["source_path"], "/boot/firmware/config.txt");
    assert_eq!(value["config"]["using_firmware_path"], true);
    assert_eq!(value["config"]["legacy_path_present"], false);
    assert_eq!(value["config"]["diagnostics_count"], 1);
    assert_eq!(value["config"]["entries"][0]["kind"], "setting");

    assert_eq!(value["camera"]["tool_used"], "rpicam-hello");
    assert_eq!(value["camera"]["rpicam_hello_present"], true);
    assert_eq!(value["camera"]["libcamera_hello_present"], false);
    assert_eq!(value["camera"]["video_devices"][0], "video0");
    assert_eq!(value["camera"]["cameras"][0]["name"], "imx708");

    assert_eq!(value["python"]["version"], "Python 3.11.2");
    assert_eq!(value["python"]["externally_managed"], true);
    assert_eq!(value["python"]["detected_packages"][0], "python3-picamera2");

    let group_domains = value["groups"]
        .as_array()
        .expect("groups should be an array")
        .iter()
        .map(|group| group["domain"].as_str().expect("domain should be a string"))
        .collect::<Vec<_>>();
    assert_eq!(group_domains, vec!["power", "thermal", "config"]);

    let finding_ids = value["findings"]
        .as_array()
        .expect("findings should be an array")
        .iter()
        .map(|finding| finding["id"].as_str().expect("id should be a string"))
        .collect::<Vec<_>>();
    assert_eq!(
        finding_ids,
        vec![
            "throttling.undervoltage_now",
            "thermal.near_throttle",
            "config_txt.stale_legacy_path",
        ]
    );
    assert_eq!(value["findings"][0]["severity"], "warning");
    assert_eq!(value["findings"][0]["impact"], "critical");
}

fn sample_report() -> Report {
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
        metadata: ReportMetadata::new("check")
            .with_supported_os(pi_doctor_core::SupportedOs {
                supported: true,
                family: Some("Debian GNU/Linux".to_owned()),
                version: Some("12".to_owned()),
                codename: Some("bookworm".to_owned()),
                reason: None,
            })
            .with_probe_availability(pi_doctor_core::ProbeAvailabilitySummary {
                total: 1,
                success: 1,
                ..pi_doctor_core::ProbeAvailabilitySummary::default()
        }),
        schema_version: "1.0.0",
        overall_status: OverallStatus::Critical,
        probe_health: vec![ProbeHealth {
            name: "board",
            outcome: ProbeOutcome::Success,
            detail: None,
        }],
        system: Some(SystemSummary {
            board_model: Some("Raspberry Pi 5".to_owned()),
            board_revision: Some("c04170".to_owned()),
            architecture: Some("aarch64".to_owned()),
            distro_name: Some("Debian GNU/Linux".to_owned()),
            distro_version: Some("12".to_owned()),
            distro_codename: Some("bookworm".to_owned()),
            kernel_release: Some("6.6.20-v8+".to_owned()),
            is_raspberry_pi: true,
        }),
        config: Some(ConfigSummary {
            source_path: Some("/boot/firmware/config.txt".to_owned()),
            using_firmware_path: true,
            legacy_path_present: false,
            diagnostics_count: 1,
            entries: vec![ConfigEntry {
                line_number: 3,
                kind: ConfigEntryKind::Setting,
                raw: "dtoverlay=vc4-kms-v3d".to_owned(),
                section: Some("all".to_owned()),
                key: Some("dtoverlay".to_owned()),
                value: Some("vc4-kms-v3d".to_owned()),
                comment: None,
            }],
        }),
        camera: Some(CameraSummary {
            tool_used: Some("rpicam-hello".to_owned()),
            rpicam_hello_present: true,
            libcamera_hello_present: false,
            video_devices: vec!["video0".to_owned()],
            cameras: vec![CameraDevice {
                index: 0,
                name: "imx708".to_owned(),
                mode_hint: Some("Modes: 'SRGGB10_CSI2P' : 2304x1296 [30.00 fps]".to_owned()),
            }],
        }),
        python: Some(PythonSummary {
            version: Some("Python 3.11.2".to_owned()),
            executable: Some("/usr/bin/python3".to_owned()),
            in_virtualenv: false,
            externally_managed: true,
            detected_packages: vec!["python3-picamera2".to_owned()],
        }),
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
