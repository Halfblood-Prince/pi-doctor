use crate::{ProbeError, read_optional_text};
use pi_doctor_core::{
    ConfigEntry, ConfigEntryKind, ConfigSummary, Finding, Impact, Probe, ProbeContext, ProbeResult,
    Severity,
};
use std::collections::BTreeMap;

const MODERN_CONFIG_PATH: &str = "/boot/firmware/config.txt";
const LEGACY_CONFIG_PATH: &str = "/boot/config.txt";
const LEGACY_KEYS: &[&str] = &[
    "start_x",
    "gpu_mem",
    "gpu_mem_256",
    "gpu_mem_512",
    "gpu_mem_1024",
];

#[derive(Debug, Clone, Default)]
pub struct ConfigAnalysis {
    pub summary: ConfigSummary,
    pub findings: Vec<Finding>,
}

pub struct ConfigTxtProbe;

impl ConfigTxtProbe {
    pub fn collect(&self, ctx: &ProbeContext) -> Result<ConfigAnalysis, ProbeError> {
        let modern = read_optional_text(ctx, MODERN_CONFIG_PATH)?;
        let legacy = read_optional_text(ctx, LEGACY_CONFIG_PATH)?;

        let (source_path, source_contents, using_firmware_path) = if let Some(contents) = modern {
            (Some(MODERN_CONFIG_PATH.to_owned()), Some(contents), true)
        } else if let Some(contents) = legacy.clone() {
            (Some(LEGACY_CONFIG_PATH.to_owned()), Some(contents), false)
        } else {
            return Err(ProbeError::ReadText {
                path: "/boot/firmware/config.txt or /boot/config.txt",
            });
        };

        let mut findings = Vec::new();
        let mut entries = Vec::new();

        if let Some(contents) = source_contents {
            entries = parse_config(&contents);
            findings.extend(duplicate_dtoverlay_findings(&entries));
            findings.extend(conflicting_dtparam_findings(&entries));
            findings.extend(legacy_option_findings(&entries));
            findings.extend(malformed_line_findings(&entries));
        }

        let legacy_path_present = legacy.is_some();
        if using_firmware_path && legacy_path_present {
            findings.push(Finding {
                id: "config_txt.stale_legacy_path",
                severity: Severity::Warning,
                impact: Impact::Warning,
                title: "Legacy /boot/config.txt is present alongside the modern config path".to_owned(),
                summary: "This system appears to use /boot/firmware/config.txt, but /boot/config.txt also exists and may be the wrong file to edit.".to_owned(),
                evidence: vec![
                    format!("active config path: {MODERN_CONFIG_PATH}"),
                    format!("legacy config path detected: {LEGACY_CONFIG_PATH}"),
                ],
                suggested_actions: vec![
                    "Why this matters: on modern Raspberry Pi OS releases, editing the old path can make changes appear to do nothing.".to_owned(),
                    "What to run next: confirm your edits are going into /boot/firmware/config.txt and rerun `pi-doctor explain config`.".to_owned(),
                ],
            });
        }

        let diagnostics_count = findings
            .iter()
            .filter(|finding| finding.id.starts_with("config_txt."))
            .count();

        Ok(ConfigAnalysis {
            summary: ConfigSummary {
                source_path,
                using_firmware_path,
                legacy_path_present,
                diagnostics_count,
                entries,
            },
            findings,
        })
    }
}

impl Probe for ConfigTxtProbe {
    fn run(&self, ctx: &ProbeContext) -> ProbeResult {
        self.collect(ctx)
            .map(|analysis| analysis.findings)
            .unwrap_or_default()
    }
}

fn parse_config(contents: &str) -> Vec<ConfigEntry> {
    let mut current_section: Option<String> = None;
    let mut entries = Vec::new();

    for (index, raw_line) in contents.lines().enumerate() {
        let line_number = index + 1;
        let trimmed = raw_line.trim();

        if trimmed.is_empty() {
            entries.push(ConfigEntry {
                line_number,
                kind: ConfigEntryKind::Blank,
                raw: raw_line.to_owned(),
                section: current_section.clone(),
                key: None,
                value: None,
                comment: None,
            });
            continue;
        }

        if trimmed.starts_with('#') {
            entries.push(ConfigEntry {
                line_number,
                kind: ConfigEntryKind::Comment,
                raw: raw_line.to_owned(),
                section: current_section.clone(),
                key: None,
                value: None,
                comment: Some(trimmed.trim_start_matches('#').trim().to_owned()),
            });
            continue;
        }

        if trimmed.starts_with('[') && trimmed.ends_with(']') && trimmed.len() > 2 {
            let section = trimmed[1..trimmed.len() - 1].trim().to_owned();
            current_section = Some(section.clone());
            entries.push(ConfigEntry {
                line_number,
                kind: ConfigEntryKind::Section,
                raw: raw_line.to_owned(),
                section: Some(section),
                key: None,
                value: None,
                comment: None,
            });
            continue;
        }

        let (core, comment) = match raw_line.split_once('#') {
            Some((core, comment)) => (core.trim_end(), Some(comment.trim().to_owned())),
            None => (raw_line, None),
        };
        let core = core.trim();

        if let Some((key, value)) = core.split_once('=') {
            let key = key.trim();
            let value = value.trim();
            if !key.is_empty() {
                entries.push(ConfigEntry {
                    line_number,
                    kind: ConfigEntryKind::Setting,
                    raw: raw_line.to_owned(),
                    section: current_section.clone(),
                    key: Some(key.to_owned()),
                    value: Some(value.to_owned()),
                    comment,
                });
                continue;
            }
        }

        entries.push(ConfigEntry {
            line_number,
            kind: ConfigEntryKind::Malformed,
            raw: raw_line.to_owned(),
            section: current_section.clone(),
            key: None,
            value: None,
            comment,
        });
    }

    entries
}

fn duplicate_dtoverlay_findings(entries: &[ConfigEntry]) -> Vec<Finding> {
    let mut overlays_by_scope = BTreeMap::<(Option<String>, String), Vec<&ConfigEntry>>::new();
    for entry in entries
        .iter()
        .filter(|entry| {
            matches!(entry.kind, ConfigEntryKind::Setting)
                && entry.key.as_deref() == Some("dtoverlay")
        })
    {
        let Some(overlay) = entry.value.as_deref().and_then(overlay_name) else {
            continue;
        };
        overlays_by_scope
            .entry((entry.section.clone(), overlay))
            .or_default()
            .push(entry);
    }

    overlays_by_scope
        .into_iter()
        .filter_map(|((section, overlay), mut matches)| {
            if matches.len() <= 1 {
                return None;
            }
            matches.sort_by_key(|entry| entry.line_number);
            let line_list = line_list(&matches);
            Some(Finding {
                id: "config_txt.duplicate_dtoverlay",
                severity: Severity::Warning,
                impact: Impact::Warning,
                title: format!("Duplicate `{overlay}` dtoverlay entries detected"),
                summary: format!(
                    "`dtoverlay={overlay}` appears more than once in the {} config section.",
                    section_label(section.as_deref())
                ),
                evidence: vec![format!("lines: {line_list}")],
                suggested_actions: vec![
                    "Why this matters: repeating the same overlay in one section is rarely intentional and can make the effective boot configuration harder to reason about.".to_owned(),
                    format!("What to run next: review lines {line_list} and keep a single `{overlay}` overlay entry unless you have a board-specific reason."),
                ],
            })
        })
        .collect()
}

fn conflicting_dtparam_findings(entries: &[ConfigEntry]) -> Vec<Finding> {
    let mut params_by_scope = BTreeMap::<(Option<String>, String), Vec<&ConfigEntry>>::new();
    for entry in entries
        .iter()
        .filter(|entry| {
            matches!(entry.kind, ConfigEntryKind::Setting)
                && entry.key.as_deref() == Some("dtparam")
        })
    {
        let Some(param) = entry.value.as_deref().and_then(dtparam_name) else {
            continue;
        };
        params_by_scope
            .entry((entry.section.clone(), param))
            .or_default()
            .push(entry);
    }

    params_by_scope
        .into_iter()
        .filter_map(|((section, param), mut matches)| {
            let values = matches
                .iter()
                .filter_map(|entry| entry.value.as_deref().and_then(dtparam_value))
                .collect::<std::collections::BTreeSet<_>>();
            if values.len() <= 1 {
                return None;
            }
            matches.sort_by_key(|entry| entry.line_number);
            let line_list = line_list(&matches);
            Some(Finding {
                id: "config_txt.conflicting_dtparam",
                severity: Severity::Warning,
                impact: Impact::Warning,
                title: format!("Conflicting `{param}` dtparam entries detected"),
                summary: format!(
                    "`dtparam={param}=...` has multiple values in the {} config section.",
                    section_label(section.as_deref())
                ),
                evidence: vec![
                    format!("lines: {line_list}"),
                    format!("values: {}", values.into_iter().collect::<Vec<_>>().join(", ")),
                ],
                suggested_actions: vec![
                    "Why this matters: only the effective value matters at boot, but conflicting repeated parameters hide intent.".to_owned(),
                    format!("What to run next: review lines {line_list} and keep the intended `{param}` value in that section."),
                ],
            })
        })
        .collect()
}

fn legacy_option_findings(entries: &[ConfigEntry]) -> Vec<Finding> {
    entries
        .iter()
        .filter(|entry| matches!(entry.kind, ConfigEntryKind::Setting))
        .filter_map(|entry| {
            let key = entry.key.as_deref()?;
            LEGACY_KEYS.contains(&key).then(|| Finding {
                id: "config_txt.legacy_option",
                severity: Severity::Warning,
                impact: Impact::Warning,
                title: format!("`{key}` looks like a legacy boot option"),
                summary: format!("`{key}` is commonly associated with older Raspberry Pi boot or camera guidance and may not be needed on current Raspberry Pi OS releases."),
                evidence: vec![format!("line {}: {}", entry.line_number, entry.raw.trim())],
                suggested_actions: vec![
                    "Why this matters: legacy options can keep old troubleshooting advice alive long after the underlying stack has changed.".to_owned(),
                    format!("What to run next: verify whether `{key}` is still required for your hardware and OS release before keeping it in config.txt."),
                ],
            })
        })
        .collect()
}

fn malformed_line_findings(entries: &[ConfigEntry]) -> Vec<Finding> {
    entries
        .iter()
        .filter(|entry| matches!(entry.kind, ConfigEntryKind::Malformed))
        .map(|entry| Finding {
            id: "config_txt.malformed_line",
            severity: Severity::Warning,
            impact: Impact::Warning,
            title: "Malformed config.txt line detected".to_owned(),
            summary: "A config.txt line could not be parsed as a section, comment, or key=value setting.".to_owned(),
            evidence: vec![format!("line {}: {}", entry.line_number, entry.raw.trim())],
            suggested_actions: vec![
                "Why this matters: malformed lines are easy to overlook and may not be interpreted the way you expect at boot.".to_owned(),
                format!("What to run next: correct or remove line {} and rerun `pi-doctor explain config`.", entry.line_number),
            ],
        })
        .collect()
}

fn overlay_name(value: &str) -> Option<String> {
    value
        .split(',')
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_ascii_lowercase())
}

fn dtparam_name(value: &str) -> Option<String> {
    value
        .split_once('=')
        .map(|(name, _)| name)
        .unwrap_or(value)
        .split(',')
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_ascii_lowercase())
}

fn dtparam_value(value: &str) -> Option<String> {
    let (_, value) = value.split_once('=')?;
    let value = value.split(',').next().unwrap_or(value).trim();
    (!value.is_empty()).then(|| value.to_ascii_lowercase())
}

fn section_label(section: Option<&str>) -> String {
    section
        .map(|section| format!("[{section}]"))
        .unwrap_or_else(|| "global".to_owned())
}

fn line_list(entries: &[&ConfigEntry]) -> String {
    entries
        .iter()
        .map(|entry| entry.line_number.to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn summarize_entries_by_key(entries: &[ConfigEntry]) -> BTreeMap<String, Vec<usize>> {
    let mut by_key = BTreeMap::new();
    for entry in entries {
        if matches!(entry.kind, ConfigEntryKind::Setting)
            && let Some(key) = &entry.key
        {
            by_key
                .entry(key.clone())
                .or_insert_with(Vec::new)
                .push(entry.line_number);
        }
    }
    by_key
}

#[cfg(test)]
mod tests {
    use super::{
        ConfigTxtProbe, conflicting_dtparam_findings, duplicate_dtoverlay_findings, parse_config,
        summarize_entries_by_key,
    };
    use pi_doctor_core::ProbeContext;
    use std::path::PathBuf;

    #[test]
    fn parses_sections_comments_duplicates_and_line_numbers() {
        let entries = parse_config(
            "# note\n[all]\ndtoverlay=spi0-1cs\ndtoverlay=i2c-gpio\n\nbad line\ndtparam=i2c_arm=on # inline\n",
        );

        assert_eq!(entries.len(), 7);
        assert_eq!(entries[0].line_number, 1);
        assert!(matches!(
            entries[0].kind,
            pi_doctor_core::ConfigEntryKind::Comment
        ));
        assert!(matches!(
            entries[1].kind,
            pi_doctor_core::ConfigEntryKind::Section
        ));
        assert_eq!(entries[2].section.as_deref(), Some("all"));
        assert_eq!(entries[5].line_number, 6);
        assert!(matches!(
            entries[5].kind,
            pi_doctor_core::ConfigEntryKind::Malformed
        ));

        let by_key = summarize_entries_by_key(&entries);
        assert_eq!(by_key.get("dtoverlay"), Some(&vec![3, 4]));
        assert_eq!(by_key.get("dtparam"), Some(&vec![7]));
    }

    #[test]
    fn allows_multiple_different_dtoverlays_in_one_section() {
        let entries = parse_config("[all]\ndtoverlay=spi0-1cs\ndtoverlay=i2c-gpio\n");

        assert!(duplicate_dtoverlay_findings(&entries).is_empty());
    }

    #[test]
    fn warns_for_repeated_dtoverlay_in_same_section() {
        let entries = parse_config("[all]\ndtoverlay=spi0-1cs\ndtoverlay=spi0-1cs,cs0_pin=8\n");
        let findings = duplicate_dtoverlay_findings(&entries);

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].id, "config_txt.duplicate_dtoverlay");
        assert!(findings[0].evidence[0].contains("2, 3"));
    }

    #[test]
    fn treats_same_dtoverlay_in_different_sections_independently() {
        let entries = parse_config("[pi4]\ndtoverlay=vc4-kms-v3d\n[pi5]\ndtoverlay=vc4-kms-v3d\n");

        assert!(duplicate_dtoverlay_findings(&entries).is_empty());
    }

    #[test]
    fn allows_repeated_dtparam_when_value_matches() {
        let entries = parse_config("[all]\ndtparam=i2c_arm=on\ndtparam=i2c_arm=on\n");

        assert!(conflicting_dtparam_findings(&entries).is_empty());
    }

    #[test]
    fn warns_for_conflicting_dtparam_values_in_same_section() {
        let entries = parse_config("[all]\ndtparam=i2c_arm=on\ndtparam=i2c_arm=off\n");
        let findings = conflicting_dtparam_findings(&entries);

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].id, "config_txt.conflicting_dtparam");
        assert!(findings[0].evidence[0].contains("2, 3"));
    }

    #[test]
    fn treats_same_dtparam_in_different_sections_independently() {
        let entries = parse_config("[pi4]\ndtparam=i2c_arm=on\n[pi5]\ndtparam=i2c_arm=off\n");

        assert!(conflicting_dtparam_findings(&entries).is_empty());
    }

    #[test]
    fn prefers_modern_firmware_path() {
        let root = fixture_root("modern-layout");
        let analysis = ConfigTxtProbe
            .collect(&ProbeContext::with_root(root))
            .expect("fixture should parse");

        assert_eq!(
            analysis.summary.source_path.as_deref(),
            Some("/boot/firmware/config.txt")
        );
        assert!(analysis.summary.using_firmware_path);
        assert!(analysis.summary.legacy_path_present);
    }

    fn fixture_root(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("fixtures")
            .join("milestone3")
            .join(name)
    }
}
