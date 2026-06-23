use crate::ProbeError;
use log::warn;
use pi_doctor_core::{Finding, Probe, ProbeContext, ProbeResult, Severity};

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
}

pub struct ThermalProbe;

impl ThermalProbe {
    pub fn collect(&self, ctx: &ProbeContext) -> Result<ThermalDetails, ProbeError> {
        let raw = ctx
            .read_text("/sys/class/thermal/thermal_zone0/temp")
            .ok_or(ProbeError::ReadText {
                path: "/sys/class/thermal/thermal_zone0/temp",
            })?;
        let celsius = parse_thermal_millidegrees(&raw)?;
        let band = celsius.map(classify_temperature);

        Ok(ThermalDetails { celsius, band })
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

        match details.band {
            Some(TemperatureBand::NearThrottle) => vec![Finding {
                id: "thermal.near_throttle",
                severity: Severity::Warning,
                title: "CPU temperature is near throttling range".to_owned(),
                summary: format!(
                    "CPU temperature is {:.1} C, which is close to the Raspberry Pi throttling threshold.",
                    details.celsius.unwrap_or_default()
                ),
                evidence: vec!["temperature classification: near throttle".to_owned()],
                suggested_actions: vec![
                    "Why this matters: sustained heat can reduce performance before full throttling becomes obvious.".to_owned(),
                    "What to run next: inspect airflow, heatsink contact, and active cooling while rerunning `pi-doctor explain throttling`.".to_owned(),
                ],
            }],
            Some(TemperatureBand::ThrottlingLikely) => vec![Finding {
                id: "thermal.throttling_likely",
                severity: Severity::Warning,
                title: "CPU temperature is in throttling territory".to_owned(),
                summary: format!(
                    "CPU temperature is {:.1} C, which is hot enough that thermal throttling is likely or already active.",
                    details.celsius.unwrap_or_default()
                ),
                evidence: vec!["temperature classification: throttling likely".to_owned()],
                suggested_actions: vec![
                    "Why this matters: Raspberry Pi boards reduce performance when they overheat.".to_owned(),
                    "What to run next: improve cooling, lower sustained load, and rerun `pi-doctor explain throttling` once temperatures fall.".to_owned(),
                ],
            }],
            _ => Vec::new(),
        }
    }
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
    use super::parse_thermal_millidegrees;

    #[test]
    fn parses_celsius_fallback_format() {
        let parsed = parse_thermal_millidegrees("temp=54.2'C")
            .expect("temperature should parse")
            .expect("temperature should exist");

        assert_eq!(parsed, 54.2);
    }
}
