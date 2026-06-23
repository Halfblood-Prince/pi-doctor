use crate::ProbeError;
use pi_doctor_core::{Probe, ProbeContext, ProbeResult};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct KernelDetails {
    pub architecture: Option<String>,
    pub release: Option<String>,
}

pub struct KernelProbe;

impl KernelProbe {
    pub fn collect(&self, ctx: &ProbeContext) -> Result<KernelDetails, ProbeError> {
        let cpuinfo = ctx.read_text("/proc/cpuinfo");
        let release = ctx
            .read_text("/proc/sys/kernel/osrelease")
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty());
        let architecture = cpuinfo
            .as_deref()
            .and_then(architecture_from_cpuinfo)
            .or_else(|| {
                ctx.read_text("/proc/sys/kernel/arch")
                    .map(|value| value.trim().to_owned())
                    .filter(|value| !value.is_empty())
            })
            .or_else(|| Some(std::env::consts::ARCH.to_owned()));

        if release.is_none() && cpuinfo.is_none() {
            return Err(ProbeError::MissingField {
                probe: "kernel",
                field: "/proc/sys/kernel/osrelease or /proc/cpuinfo",
            });
        }

        Ok(KernelDetails {
            architecture,
            release,
        })
    }
}

impl Probe for KernelProbe {
    fn run(&self, _ctx: &ProbeContext) -> ProbeResult {
        Vec::new()
    }
}

fn architecture_from_cpuinfo(cpuinfo: &str) -> Option<String> {
    let lower = cpuinfo.to_ascii_lowercase();

    if lower.contains("aarch64") || lower.contains("armv8") {
        Some("aarch64".to_owned())
    } else if lower.contains("armv7") || lower.contains("v7l") {
        Some("armv7l".to_owned())
    } else if lower.contains("x86_64") || lower.contains("amd64") {
        Some("x86_64".to_owned())
    } else {
        None
    }
}
