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
    pub fn collect(&self, ctx: &ProbeContext) -> ThermalDetails {
        let celsius = ctx
            .read_text("/sys/class/thermal/thermal_zone0/temp")
            .and_then(|raw| parse_thermal_millidegrees(&raw));
        let band = celsius.map(classify_temperature);

        ThermalDetails { celsius, band }
    }
}

impl Probe for ThermalProbe {
    fn run(&self, ctx: &ProbeContext) -> ProbeResult {
        let details = self.collect(ctx);

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

pub fn parse_thermal_millidegrees(raw: &str) -> Option<f32> {
    let millidegrees: f32 = raw.trim().parse().ok()?;
    Some(millidegrees / 1000.0)
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
