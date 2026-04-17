use crate::ProbeError;
use log::warn;
use pi_doctor_core::{Finding, Probe, ProbeContext, ProbeResult, Severity};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BoardDetails {
    pub model: Option<String>,
    pub revision: Option<String>,
    pub is_raspberry_pi: bool,
}

pub struct BoardProbe;

impl BoardProbe {
    pub fn collect(&self, ctx: &ProbeContext) -> Result<BoardDetails, ProbeError> {
        let model = ctx
            .read_text("/proc/device-tree/model")
            .map(|raw| raw.trim_matches(char::from(0)).trim().to_owned())
            .filter(|value| !value.is_empty());
        let cpuinfo = ctx.read_text("/proc/cpuinfo").unwrap_or_default();
        let revision = cpuinfo_value(&cpuinfo, "Revision");
        let hardware = cpuinfo_value(&cpuinfo, "Hardware");
        let model_name = cpuinfo_value(&cpuinfo, "Model");
        let is_raspberry_pi = model
            .as_deref()
            .or(model_name.as_deref())
            .is_some_and(|value| value.contains("Raspberry Pi"))
            || hardware
                .as_deref()
                .is_some_and(|value| value.contains("BCM") || value.contains("Raspberry Pi"));

        if model.is_none() && cpuinfo.trim().is_empty() {
            return Err(ProbeError::ReadText {
                path: "/proc/device-tree/model or /proc/cpuinfo",
            });
        }

        Ok(BoardDetails {
            model: model.or(model_name),
            revision,
            is_raspberry_pi,
        })
    }
}

impl Probe for BoardProbe {
    fn run(&self, ctx: &ProbeContext) -> ProbeResult {
        let details = match self.collect(ctx) {
            Ok(details) => details,
            Err(error) => {
                warn!("board probe fallback: {error}");
                BoardDetails::default()
            }
        };
        if details.is_raspberry_pi {
            Vec::new()
        } else {
            vec![Finding {
                id: "board.non_raspberry_pi",
                severity: Severity::Warning,
                title: "Host does not look like a Raspberry Pi".to_owned(),
                summary: "pi-doctor detected a Linux-like system identity, but it could not confirm Raspberry Pi hardware.".to_owned(),
                evidence: vec![
                    format!(
                        "board model: {}",
                        details.model.unwrap_or_else(|| "unavailable".to_owned())
                    ),
                    format!(
                        "board revision: {}",
                        details.revision.unwrap_or_else(|| "unavailable".to_owned())
                    ),
                ],
                suggested_actions: vec![
                    "Why this matters: later diagnostics rely on Raspberry Pi-specific firmware and device metadata.".to_owned(),
                    "What to run next: confirm `/proc/device-tree/model` exists on the target Raspberry Pi and rerun `pi-doctor check` there.".to_owned(),
                ],
            }]
        }
    }
}

fn cpuinfo_value(cpuinfo: &str, key: &str) -> Option<String> {
    cpuinfo.lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        if name.trim() == key {
            let trimmed = value.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_owned())
        } else {
            None
        }
    })
}
