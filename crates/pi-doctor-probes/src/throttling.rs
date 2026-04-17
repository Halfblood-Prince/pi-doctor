use crate::ProbeError;
use log::warn;
use pi_doctor_core::{CommandOutput, Finding, Probe, ProbeContext, ProbeResult, Severity};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ThrottlingDetails {
    pub raw_value: Option<u32>,
    pub undervoltage_now: bool,
    pub arm_frequency_capped_now: bool,
    pub throttled_now: bool,
    pub soft_temperature_limit_now: bool,
    pub undervoltage_happened: bool,
    pub arm_frequency_capped_happened: bool,
    pub throttling_happened: bool,
    pub soft_temperature_limit_happened: bool,
    pub vcgencmd_available: bool,
    pub error: Option<String>,
}

pub struct ThrottlingProbe;

impl ThrottlingProbe {
    pub fn collect(&self, ctx: &ProbeContext) -> Result<ThrottlingDetails, ProbeError> {
        match ctx.run_command("vcgencmd", &["get_throttled"]) {
            CommandOutput::Success(output) => match parse_throttled_output(&output) {
                Ok(details) => Ok(ThrottlingDetails {
                    vcgencmd_available: true,
                    ..details
                }),
                Err(error) => Err(error),
            },
            CommandOutput::Missing => Ok(ThrottlingDetails {
                vcgencmd_available: false,
                ..ThrottlingDetails::default()
            }),
            CommandOutput::Failure(detail) => Err(ProbeError::CommandFailure {
                program: "vcgencmd",
                args: "get_throttled".to_owned(),
                detail,
            }),
        }
    }
}

impl Probe for ThrottlingProbe {
    fn run(&self, ctx: &ProbeContext) -> ProbeResult {
        let details = match self.collect(ctx) {
            Ok(details) => details,
            Err(error) => {
                warn!("throttling probe fallback: {error}");
                ThrottlingDetails {
                    vcgencmd_available: true,
                    error: Some(error.to_string()),
                    ..ThrottlingDetails::default()
                }
            }
        };
        let mut findings = Vec::new();

        if !details.vcgencmd_available {
            findings.push(Finding {
                id: "throttling.vcgencmd_missing",
                severity: Severity::Warning,
                title: "vcgencmd is not available".to_owned(),
                summary: "pi-doctor could not read Raspberry Pi firmware throttling telemetry because `vcgencmd` is missing.".to_owned(),
                evidence: vec!["command: vcgencmd get_throttled".to_owned()],
                suggested_actions: vec![
                    "Why this matters: undervoltage and firmware throttling signals come from Raspberry Pi firmware telemetry.".to_owned(),
                    "What to run next: verify you are on Raspberry Pi OS or install the Raspberry Pi firmware utilities before rerunning `pi-doctor explain throttling`.".to_owned(),
                ],
            });
            return findings;
        }

        if let Some(error) = details.error {
            findings.push(Finding {
                id: "throttling.vcgencmd_failed",
                severity: Severity::Warning,
                title: "vcgencmd throttling query failed".to_owned(),
                summary: "pi-doctor tried to read throttling telemetry, but the firmware command did not return a usable result.".to_owned(),
                evidence: vec![error],
                suggested_actions: vec![
                    "Why this matters: without a valid throttle bitmask, undervoltage and thermal history cannot be decoded reliably.".to_owned(),
                    "What to run next: run `vcgencmd get_throttled` directly and compare the output before rerunning `pi-doctor explain throttling`.".to_owned(),
                ],
            });
            return findings;
        }

        push_flag_finding(
            &mut findings,
            details.undervoltage_now,
            "throttling.undervoltage_now",
            "Under-voltage is active now",
            "Firmware telemetry reports an active under-voltage condition.",
            "Why this matters: unstable power can cause throttling, crashes, and peripheral instability.",
            "What to run next: check the power supply, cable quality, and inline voltage drops, then rerun `pi-doctor explain throttling`.",
        );
        push_flag_finding(
            &mut findings,
            details.undervoltage_happened,
            "throttling.undervoltage_happened",
            "Under-voltage happened historically",
            "Firmware telemetry shows the board experienced under-voltage since boot.",
            "Why this matters: even if power looks stable now, historical undervoltage can explain intermittent slowdowns or resets.",
            "What to run next: inspect recent load spikes, attached peripherals, and power headroom.",
        );
        push_flag_finding(
            &mut findings,
            details.throttled_now,
            "throttling.active",
            "Throttling is active",
            "Firmware telemetry reports active throttling right now.",
            "Why this matters: the board is reducing performance to protect itself or stay within power limits.",
            "What to run next: inspect power quality and thermals, then compare with `vcgencmd get_throttled` after the system cools down.",
        );
        push_flag_finding(
            &mut findings,
            details.soft_temperature_limit_now,
            "throttling.soft_temp_limit_now",
            "Soft thermal limit is active",
            "Firmware telemetry reports the soft thermal limit is currently active.",
            "Why this matters: the system is hot enough that performance may already be reduced.",
            "What to run next: improve cooling and airflow, then watch whether the bitmask clears.",
        );

        findings
    }
}

pub fn parse_throttled_output(raw: &str) -> Result<ThrottlingDetails, ProbeError> {
    let value = raw
        .trim()
        .strip_prefix("throttled=")
        .ok_or_else(|| ProbeError::Parse {
            probe: "throttling",
            detail: "expected output in the form `throttled=0x...`".to_owned(),
        })?;
    let value = value.strip_prefix("0x").ok_or_else(|| ProbeError::Parse {
        probe: "throttling",
        detail: "expected hexadecimal throttle value".to_owned(),
    })?;
    let mask = u32::from_str_radix(value, 16).map_err(|_| ProbeError::Parse {
        probe: "throttling",
        detail: "invalid hexadecimal throttle value".to_owned(),
    })?;

    Ok(ThrottlingDetails {
        raw_value: Some(mask),
        undervoltage_now: bit(mask, 0),
        arm_frequency_capped_now: bit(mask, 1),
        throttled_now: bit(mask, 2),
        soft_temperature_limit_now: bit(mask, 3),
        undervoltage_happened: bit(mask, 16),
        arm_frequency_capped_happened: bit(mask, 17),
        throttling_happened: bit(mask, 18),
        soft_temperature_limit_happened: bit(mask, 19),
        vcgencmd_available: true,
        error: None,
    })
}

fn bit(mask: u32, shift: u8) -> bool {
    mask & (1_u32 << shift) != 0
}

fn push_flag_finding(
    findings: &mut Vec<Finding>,
    active: bool,
    id: &'static str,
    title: &str,
    summary: &str,
    why: &str,
    next: &str,
) {
    if active {
        findings.push(Finding {
            id,
            severity: Severity::Warning,
            title: title.to_owned(),
            summary: summary.to_owned(),
            evidence: Vec::new(),
            suggested_actions: vec![why.to_owned(), next.to_owned()],
        });
    }
}

#[cfg(test)]
mod tests {
    use super::parse_throttled_output;

    #[test]
    fn parses_clear_bitmask() {
        let details = parse_throttled_output("throttled=0x0").expect("bitmask should parse");
        assert_eq!(details.raw_value, Some(0));
        assert!(!details.undervoltage_now);
        assert!(!details.undervoltage_happened);
    }

    #[test]
    fn parses_current_and_historical_flags() {
        let details = parse_throttled_output("throttled=0x50005").expect("bitmask should parse");
        assert!(details.undervoltage_now);
        assert!(details.throttled_now);
        assert!(details.undervoltage_happened);
        assert!(details.throttling_happened);
        assert!(!details.soft_temperature_limit_now);
    }

    #[test]
    fn rejects_malformed_output() {
        assert!(parse_throttled_output("oops").is_err());
    }
}
