use crate::{ProbeError, read_optional_text};
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
        let os_release = match read_optional_text(ctx, "/etc/os-release")? {
            Some(contents) => contents,
            None => read_optional_text(ctx, "/usr/lib/os-release")?.ok_or(ProbeError::ReadText {
                path: "/etc/os-release or /usr/lib/os-release",
            })?,
        };

        let version_field = os_release_value(&os_release, "VERSION");
        let pretty_name = os_release_value(&os_release, "PRETTY_NAME");

        Ok(OsDetails {
            distro_name: os_release_value(&os_release, "NAME"),
            distro_version: os_release_value(&os_release, "VERSION_ID")
                .or_else(|| version_field.as_deref().and_then(extract_version_id))
                .or_else(|| pretty_name.as_deref().and_then(extract_version_id)),
            distro_codename: os_release_value(&os_release, "VERSION_CODENAME")
                .or_else(|| os_release_value(&os_release, "DEBIAN_CODENAME"))
                .or_else(|| os_release_value(&os_release, "UBUNTU_CODENAME"))
                .or_else(|| version_field.as_deref().and_then(extract_codename))
                .or_else(|| pretty_name.as_deref().and_then(extract_codename)),
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

fn extract_version_id(value: &str) -> Option<String> {
    let token = value
        .split_whitespace()
        .find(|part| part.chars().any(|ch| ch.is_ascii_digit()))?;
    let normalized = token
        .trim_matches(|ch: char| !ch.is_ascii_alphanumeric() && ch != '.')
        .trim_end_matches('.');
    (!normalized.is_empty()
        && normalized
            .chars()
            .all(|ch| ch.is_ascii_digit() || ch == '.'))
    .then(|| normalized.to_owned())
}

fn extract_codename(value: &str) -> Option<String> {
    let start = value.find('(')?;
    let end = value[start + 1..].find(')')?;
    let codename = value[start + 1..start + 1 + end].trim();
    (!codename.is_empty()).then(|| codename.to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::{extract_codename, extract_version_id, os_release_value};

    #[test]
    fn extracts_version_and_codename_from_pretty_name_style_values() {
        assert_eq!(
            extract_version_id("Debian GNU/Linux 12 (bookworm)").as_deref(),
            Some("12")
        );
        assert_eq!(
            extract_codename("Debian GNU/Linux 12 (bookworm)").as_deref(),
            Some("bookworm")
        );
    }

    #[test]
    fn parses_quoted_os_release_values() {
        let contents = "PRETTY_NAME=\"Debian GNU/Linux 12 (bookworm)\"\n";
        assert_eq!(
            os_release_value(contents, "PRETTY_NAME").as_deref(),
            Some("Debian GNU/Linux 12 (bookworm)")
        );
    }
}
