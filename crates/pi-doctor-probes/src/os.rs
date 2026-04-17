use crate::ProbeError;
use pi_doctor_core::{Probe, ProbeContext, ProbeResult};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OsDetails {
    pub distro_name: Option<String>,
    pub distro_version: Option<String>,
    pub distro_codename: Option<String>,
}

pub struct OsProbe;

impl OsProbe {
    pub fn collect(&self, ctx: &ProbeContext) -> Result<OsDetails, ProbeError> {
        let os_release = ctx
            .read_text("/etc/os-release")
            .ok_or(ProbeError::ReadText {
                path: "/etc/os-release",
            })?;

        Ok(OsDetails {
            distro_name: os_release_value(&os_release, "NAME"),
            distro_version: os_release_value(&os_release, "VERSION_ID"),
            distro_codename: os_release_value(&os_release, "VERSION_CODENAME")
                .or_else(|| os_release_value(&os_release, "DEBIAN_CODENAME")),
        })
    }
}

impl Probe for OsProbe {
    fn run(&self, _ctx: &ProbeContext) -> ProbeResult {
        Vec::new()
    }
}

fn os_release_value(contents: &str, key: &str) -> Option<String> {
    contents.lines().find_map(|line| {
        let (name, value) = line.split_once('=')?;
        if name.trim() == key {
            let trimmed = value.trim().trim_matches('"');
            (!trimmed.is_empty()).then(|| trimmed.to_owned())
        } else {
            None
        }
    })
}
