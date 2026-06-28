use crate::ProbeError;
use log::warn;
use pi_doctor_core::{Finding, Impact, Probe, ProbeContext, ProbeResult, Severity};

const THERMAL_CLASS_PATH: &str = "/sys/class/thermal";
const THERMAL_TEMP_GLOB: &str = "/sys/class/thermal/thermal_zone*/temp";
const THERMAL_TYPE_GLOB: &str = "/sys/class/thermal/thermal_zone*/type";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemperatureBand {
    Normal,
    Warm,
    NearThrottle,
    ThrottlingLikely,
}

impl TemperatureBand {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Warm => "warm",
            Self::NearThrottle => "near throttle",
            Self::ThrottlingLikely => "throttling likely",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ThermalDetails {
    pub celsius: Option<f32>,
    pub band: Option<TemperatureBand>,
    pub source_path: Option<String>,
    pub zone_type: Option<String>,
}

pub struct ThermalProbe;

impl ThermalProbe {
    pub fn collect(&self, ctx: &ProbeContext) -> Result<ThermalDetails, ProbeError> {
        let mut candidates = thermal_zone_candidates(ctx)?;
        if candidates.is_empty() {
            return Err(ProbeError::ReadText {
                path: THERMAL_TEMP_GLOB,
            });
        }

        candidates.sort_by_key(|candidate| {
            (
                thermal_zone_rank(candidate.zone_type.as_deref()),
                candidate.index,
            )
        });

        let mut first_parse_error = None;
        for candidate in candidates {
            match parse_thermal_millidegrees(&candidate.raw_temp) {
                Ok(celsius) => {
                    let band = celsius.map(classify_temperature);
                    return Ok(ThermalDetails {
                        celsius,
                        band,
                        source_path: Some(candidate.temp_path),
                        zone_type: candidate.zone_type,
                    });
                }
                Err(error) if first_parse_error.is_none() => first_parse_error = Some(error),
                Err(_) => {}
            }
        }

        Err(first_parse_error.unwrap_or_else(|| ProbeError::Parse {
            probe: "thermal",
            detail: "no parseable thermal zones were found".to_owned(),
        }))
    }
}

impl Probe for ThermalProbe {
    fn run(&self, ctx: &ProbeContext) -> ProbeResult {
        let details = match self.collect(ctx) {
            Ok(details) => details,
            Err(error) => {
                warn!("thermal probe fallback: {error}");
                ThermalDetails::default()
            }
        };
        thermal_findings(&details)
    }
}

pub fn thermal_findings(details: &ThermalDetails) -> Vec<Finding> {
    match details.band {
        Some(TemperatureBand::NearThrottle) => vec![Finding {
            id: "thermal.near_throttle",
            severity: Severity::Warning,
            impact: Impact::Warning,
            title: "CPU temperature is near throttling range".to_owned(),
            summary: format!(
                "CPU temperature is {:.1} C, which is close to the Raspberry Pi throttling threshold.",
                details.celsius.unwrap_or_default()
            ),
            evidence: thermal_evidence(details, "near throttle"),
            suggested_actions: vec![
                "Why this matters: sustained heat can reduce performance before full throttling becomes obvious.".to_owned(),
                "What to run next: inspect airflow, heatsink contact, and active cooling while rerunning `pi-doctor explain throttling`.".to_owned(),
            ],
        }],
        Some(TemperatureBand::ThrottlingLikely) => vec![Finding {
            id: "thermal.throttling_likely",
            severity: Severity::Warning,
            impact: Impact::Critical,
            title: "CPU temperature is in throttling territory".to_owned(),
            summary: format!(
                "CPU temperature is {:.1} C, which is hot enough that thermal throttling is likely or already active.",
                details.celsius.unwrap_or_default()
            ),
            evidence: thermal_evidence(details, "throttling likely"),
            suggested_actions: vec![
                "Why this matters: Raspberry Pi boards reduce performance when they overheat.".to_owned(),
                "What to run next: improve cooling, lower sustained load, and rerun `pi-doctor explain throttling` once temperatures fall.".to_owned(),
            ],
        }],
        _ => Vec::new(),
    }
}

#[derive(Debug)]
struct ThermalZoneCandidate {
    index: usize,
    temp_path: String,
    raw_temp: String,
    zone_type: Option<String>,
}

fn thermal_zone_candidates(ctx: &ProbeContext) -> Result<Vec<ThermalZoneCandidate>, ProbeError> {
    let mut names = ctx
        .list_dir(THERMAL_CLASS_PATH)
        .into_iter()
        .filter(|name| name.starts_with("thermal_zone"))
        .collect::<Vec<_>>();

    if names.is_empty() && ctx.path_exists("/sys/class/thermal/thermal_zone0/temp") {
        names.push("thermal_zone0".to_owned());
    }

    let mut candidates = Vec::new();
    for name in names {
        let Some(index) = thermal_zone_index(&name) else {
            continue;
        };
        let base_path = format!("{THERMAL_CLASS_PATH}/{name}");
        let temp_path = format!("{base_path}/temp");
        let type_path = format!("{base_path}/type");
        let Some(raw_temp) = read_optional_dynamic(ctx, &temp_path, THERMAL_TEMP_GLOB)? else {
            continue;
        };
        let zone_type = read_optional_dynamic(ctx, &type_path, THERMAL_TYPE_GLOB)?
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty());

        candidates.push(ThermalZoneCandidate {
            index,
            temp_path,
            raw_temp,
            zone_type,
        });
    }

    Ok(candidates)
}

fn read_optional_dynamic(
    ctx: &ProbeContext,
    path: &str,
    error_path: &'static str,
) -> Result<Option<String>, ProbeError> {
    match ctx.read_text_result(path) {
        Ok(contents) => Ok(Some(contents)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
            Err(ProbeError::PermissionDenied { path: error_path })
        }
        Err(_) => Err(ProbeError::ReadText { path: error_path }),
    }
}

fn thermal_zone_index(name: &str) -> Option<usize> {
    name.strip_prefix("thermal_zone")?.parse().ok()
}

fn thermal_zone_rank(zone_type: Option<&str>) -> u8 {
    let Some(zone_type) = zone_type else {
        return 4;
    };
    let normalized = zone_type.to_ascii_lowercase();
    if normalized.contains("cpu") || normalized.contains("soc") {
        0
    } else if normalized.contains("bcm") || normalized.contains("vc4") {
        1
    } else if normalized.contains("thermal") {
        2
    } else {
        3
    }
}

fn thermal_evidence(details: &ThermalDetails, classification: &str) -> Vec<String> {
    let mut evidence = vec![format!("temperature classification: {classification}")];
    if let Some(source_path) = &details.source_path {
        evidence.push(format!("temperature source: {source_path}"));
    }
    if let Some(zone_type) = &details.zone_type {
        evidence.push(format!("thermal zone type: {zone_type}"));
    }
    evidence
}

pub fn parse_thermal_millidegrees(raw: &str) -> Result<Option<f32>, ProbeError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    if let Ok(millidegrees) = trimmed.parse::<f32>() {
        return Ok(Some(millidegrees / 1000.0));
    }

    if let Some(celsius) = parse_thermal_celsius_fallback(trimmed) {
        return Ok(Some(celsius));
    }

    Err(ProbeError::Parse {
        probe: "thermal",
        detail: "invalid temperature value".to_owned(),
    })
}

pub fn classify_temperature(celsius: f32) -> TemperatureBand {
    if celsius < 65.0 {
        TemperatureBand::Normal
    } else if celsius < 75.0 {
        TemperatureBand::Warm
    } else if celsius < 80.0 {
        TemperatureBand::NearThrottle
    } else {
        TemperatureBand::ThrottlingLikely
    }
}

fn parse_thermal_celsius_fallback(raw: &str) -> Option<f32> {
    let normalized = raw.trim().trim_start_matches("temp=").trim();
    let normalized = normalized
        .trim_end_matches("'C")
        .trim_end_matches('C')
        .trim();
    normalized.parse::<f32>().ok()
}

#[cfg(test)]
mod tests {
    use super::{ThermalProbe, parse_thermal_millidegrees};
    use pi_doctor_core::ProbeContext;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn parses_celsius_fallback_format() {
        let parsed = parse_thermal_millidegrees("temp=54.2'C")
            .expect("temperature should parse")
            .expect("temperature should exist");

        assert_eq!(parsed, 54.2);
    }

    #[test]
    fn selects_cpu_thermal_zone_by_type_instead_of_assuming_zone_zero() {
        let root = temp_fixture_root();
        write_fixture_file(
            &root,
            "sys/class/thermal/thermal_zone0/type",
            "battery\n",
        );
        write_fixture_file(&root, "sys/class/thermal/thermal_zone0/temp", "90000\n");
        write_fixture_file(
            &root,
            "sys/class/thermal/thermal_zone1/type",
            "cpu-thermal\n",
        );
        write_fixture_file(&root, "sys/class/thermal/thermal_zone1/temp", "42123\n");

        let details = ThermalProbe
            .collect(&ProbeContext::with_root(&root))
            .expect("thermal zone should collect");
        let celsius = details.celsius.expect("temperature should be available");

        assert!((celsius - 42.123).abs() < 0.001);
        assert_eq!(
            details.source_path.as_deref(),
            Some("/sys/class/thermal/thermal_zone1/temp")
        );
        assert_eq!(details.zone_type.as_deref(), Some("cpu-thermal"));

        let _ = fs::remove_dir_all(root);
    }

    fn temp_fixture_root() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after Unix epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("pi-doctor-thermal-{nanos}"));
        let _ = fs::remove_dir_all(&root);
        root
    }

    fn write_fixture_file(root: &Path, relative: &str, contents: &str) {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().expect("fixture path should have parent"))
            .expect("fixture parent should be created");
        fs::write(path, contents).expect("fixture file should be written");
    }
}
