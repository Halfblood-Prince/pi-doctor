use crate::ProbeError;
use log::{debug, warn};
use pi_doctor_core::{
    CameraDevice, CameraSummary, CommandOutput, Finding, Probe, ProbeContext, ProbeResult, Severity,
};

#[derive(Debug, Clone, Default)]
pub struct CameraAnalysis {
    pub summary: CameraSummary,
    pub findings: Vec<Finding>,
}

pub struct CameraProbe;

impl CameraProbe {
    pub fn collect(&self, ctx: &ProbeContext) -> Result<CameraAnalysis, ProbeError> {
        let rpicam_present = ctx.command_exists("rpicam-hello");
        let libcamera_present = ctx.command_exists("libcamera-hello");
        let video_devices = ctx
            .list_dir("/dev")
            .into_iter()
            .filter(|name| name.starts_with("video"))
            .collect::<Vec<_>>();

        let (tool_used, tool_output) = if rpicam_present {
            match ctx.run_command("rpicam-hello", &["--list-cameras"]) {
                CommandOutput::Success(output) => (Some("rpicam-hello".to_owned()), Some(output)),
                CommandOutput::Failure(_) | CommandOutput::Missing => {
                    if libcamera_present {
                        match ctx.run_command("libcamera-hello", &["--list-cameras"]) {
                            CommandOutput::Success(output) => {
                                (Some("libcamera-hello".to_owned()), Some(output))
                            }
                            CommandOutput::Failure(_) | CommandOutput::Missing => (None, None),
                        }
                    } else {
                        (None, None)
                    }
                }
            }
        } else if libcamera_present {
            match ctx.run_command("libcamera-hello", &["--list-cameras"]) {
                CommandOutput::Success(output) => {
                    (Some("libcamera-hello".to_owned()), Some(output))
                }
                CommandOutput::Failure(_) | CommandOutput::Missing => (None, None),
            }
        } else {
            (None, None)
        };

        let cameras = tool_output
            .as_deref()
            .map(parse_camera_inventory)
            .unwrap_or_default();
        if tool_used.is_none() && !rpicam_present && !libcamera_present {
            debug!("camera probe fallback: no camera inventory tools were detected");
        }

        let summary = CameraSummary {
            tool_used,
            rpicam_hello_present: rpicam_present,
            libcamera_hello_present: libcamera_present,
            video_devices,
            cameras: cameras.clone(),
        };

        let findings = camera_findings(&summary);

        Ok(CameraAnalysis { summary, findings })
    }
}

impl Probe for CameraProbe {
    fn run(&self, ctx: &ProbeContext) -> ProbeResult {
        match self.collect(ctx) {
            Ok(analysis) => analysis.findings,
            Err(error) => {
                warn!("camera probe fallback: {error}");
                camera_findings(&CameraSummary {
                    tool_used: None,
                    rpicam_hello_present: false,
                    libcamera_hello_present: false,
                    video_devices: Vec::new(),
                    cameras: Vec::new(),
                })
            }
        }
    }
}

pub fn parse_camera_inventory(output: &str) -> Vec<CameraDevice> {
    let mut cameras = Vec::new();
    let mut current: Option<CameraDevice> = None;

    for line in output.lines() {
        let trimmed = line.trim();
        if let Some((index, name)) = parse_camera_header(trimmed) {
            if let Some(camera) = current.take() {
                cameras.push(camera);
            }
            current = Some(CameraDevice {
                index,
                name,
                mode_hint: None,
            });
            continue;
        }

        if let Some(camera) = current.as_mut()
            && !trimmed.is_empty()
            && camera.mode_hint.is_none()
        {
            camera.mode_hint = Some(trimmed.to_owned());
        }
    }

    if let Some(camera) = current {
        cameras.push(camera);
    }

    cameras
}

fn parse_camera_header(line: &str) -> Option<(usize, String)> {
    if line.is_empty() || line.starts_with("Available cameras") || line.starts_with('-') {
        return None;
    }

    if let Some(rest) = line.strip_prefix('[') {
        let (index, rest) = rest.split_once(']')?;
        let index = index.trim().parse().ok()?;
        let rest = rest.trim();
        if rest.is_empty() {
            return None;
        }

        let name = if let Some((name, _)) = rest.split_once('(') {
            name.trim()
        } else {
            rest
        };

        return (!name.is_empty()).then(|| (index, name.to_owned()));
    }

    let (index, rest) = line.split_once(':')?;
    let index = index.trim().parse().ok()?;
    let rest = rest.trim();
    if rest.is_empty() {
        return None;
    }
    let name = if let Some((name, _)) = rest.split_once('[') {
        name.trim()
    } else {
        rest
    };
    (!name.is_empty()).then(|| (index, name.to_owned()))
}

fn camera_findings(summary: &CameraSummary) -> Vec<Finding> {
    if summary.tool_used.is_none() {
        return vec![Finding {
            id: "camera.tool_missing",
            severity: Severity::Warning,
            title: "Camera userspace tools are missing".to_owned(),
            summary: "Neither `rpicam-hello` nor `libcamera-hello` appears to be available, so pi-doctor cannot query camera inventory.".to_owned(),
            evidence: vec!["tools checked: rpicam-hello, libcamera-hello".to_owned()],
            suggested_actions: vec![
                "Why this matters: without modern camera CLI tools, pi-doctor cannot tell whether the camera stack is ready or just missing userspace support.".to_owned(),
                "What to run next: install the Raspberry Pi camera userspace tools and rerun `pi-doctor doctor camera`.".to_owned(),
            ],
        }];
    }

    if summary.cameras.is_empty() {
        let summary_text = if summary.video_devices.is_empty() {
            "Camera tool is present, but no cameras were detected."
        } else {
            "Camera tool is present, but no cameras were detected even though video devices exist."
        };

        return vec![Finding {
            id: "camera.no_cameras_detected",
            severity: Severity::Warning,
            title: "Camera tools are present, but no cameras were detected".to_owned(),
            summary: summary_text.to_owned(),
            evidence: vec![format!(
                "tool used: {}",
                summary.tool_used.as_deref().unwrap_or("unknown")
            )],
            suggested_actions: vec![
                "Why this matters: this usually points to detection, cabling, or connector-orientation issues rather than a missing userspace package.".to_owned(),
                "What to run next: reseat the camera cable, verify connector orientation, and rerun `pi-doctor doctor camera`.".to_owned(),
            ],
        }];
    }

    let mut findings = Vec::new();
    findings.push(Finding {
        id: "camera.detected_ready",
        severity: Severity::Info,
        title: if summary.cameras.len() == 1 {
            "Camera detected and ready".to_owned()
        } else {
            "Multiple cameras detected".to_owned()
        },
        summary: format!(
            "{} camera(s) detected via {}.",
            summary.cameras.len(),
            summary.tool_used.as_deref().unwrap_or("camera tool")
        ),
        evidence: summary
            .cameras
            .iter()
            .map(|camera| format!("[{}] {}", camera.index, camera.name))
            .collect(),
        suggested_actions: vec![
            "Why this matters: the modern camera stack can see at least one camera, so the base userspace path is working.".to_owned(),
            "What to run next: test image capture or preview with the same camera toolchain that detected the device.".to_owned(),
        ],
    });

    findings
}

#[cfg(test)]
mod tests {
    use super::parse_camera_inventory;

    #[test]
    fn parses_rpicam_style_inventory() {
        let cameras = parse_camera_inventory(
            "Available cameras\n-----------------\n0 : imx219 [3280x2464 10-bit]\n    /base/soc/i2c0mux/i2c@1/imx219@10\n1 : imx708 [4608x2592 10-bit]\n    /base/soc/i2c0mux/i2c@0/imx708@1a\n",
        );

        assert_eq!(cameras.len(), 2);
        assert_eq!(cameras[0].index, 0);
        assert_eq!(cameras[0].name, "imx219");
    }

    #[test]
    fn parses_libcamera_style_inventory() {
        let cameras = parse_camera_inventory(
            "Available cameras\n-----------------\n[0] imx219\n    Modes: 'SRGGB10_CSI2P' : 640x480\n",
        );

        assert_eq!(cameras.len(), 1);
        assert_eq!(cameras[0].index, 0);
        assert_eq!(cameras[0].name, "imx219");
        assert_eq!(
            cameras[0].mode_hint.as_deref(),
            Some("Modes: 'SRGGB10_CSI2P' : 640x480")
        );
    }
}
