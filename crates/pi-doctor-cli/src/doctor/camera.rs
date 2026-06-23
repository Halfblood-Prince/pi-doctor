use pi_doctor_core::ProbeContext;
use pi_doctor_probes::camera::CameraProbe;

pub fn render(ctx: &ProbeContext) -> String {
    let analysis = CameraProbe.collect(ctx).unwrap_or_default();
    let summary = analysis.summary;
    let available_tools = match (
        summary.rpicam_hello_present,
        summary.libcamera_hello_present,
    ) {
        (true, true) => "rpicam-hello, libcamera-hello".to_owned(),
        (true, false) => "rpicam-hello".to_owned(),
        (false, true) => "libcamera-hello".to_owned(),
        (false, false) => "none".to_owned(),
    };

    let verdict = if summary.tool_used.is_none() {
        "userspace tools missing"
    } else if summary.cameras.is_empty() {
        "userspace tools present, no camera detected"
    } else if summary.rpicam_hello_present || summary.libcamera_hello_present {
        "camera detected and ready"
    } else {
        "ambiguous; re-seat cable / check connector orientation"
    };

    let mut lines = vec![
        "pi-doctor doctor camera".to_owned(),
        format!("Verdict: {verdict}"),
        format!(
            "  tool used: {}",
            summary.tool_used.as_deref().unwrap_or("none")
        ),
        format!("  available tools: {}", available_tools),
        format!(
            "  video devices: {}",
            if summary.video_devices.is_empty() {
                "none detected".to_owned()
            } else {
                summary.video_devices.join(", ")
            }
        ),
    ];

    lines.push(String::new());
    lines.push("Inventory".to_owned());
    if summary.cameras.is_empty() {
        lines.push("  No cameras detected by the available camera tool.".to_owned());
    } else {
        for camera in &summary.cameras {
            lines.push(format!("  [{}] {}", camera.index, camera.name));
            if let Some(mode_hint) = &camera.mode_hint {
                lines.push(format!("      {mode_hint}"));
            }
        }
    }

    if !analysis.findings.is_empty() {
        lines.push(String::new());
        lines.push("Why this verdict".to_owned());
        for finding in &analysis.findings {
            lines.push(format!("  {}.", finding.title));
            lines.push(format!("  {}", finding.summary));
        }
    }

    format!("{}\n", lines.join("\n"))
}
