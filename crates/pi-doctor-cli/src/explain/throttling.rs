use pi_doctor_core::ProbeContext;
use pi_doctor_probes::{
    ProbeError,
    thermal::{TemperatureBand, ThermalProbe},
    throttling::{ThrottlingDetails, ThrottlingProbe},
};

pub fn render(ctx: &ProbeContext) -> String {
    let throttling = ThrottlingProbe
        .collect(ctx)
        .unwrap_or_else(|error| ThrottlingDetails {
            vcgencmd_available: true,
            error: Some(error),
            ..ThrottlingDetails::default()
        });
    let thermal = ThermalProbe.collect(ctx).unwrap_or_default();

    let mut lines = vec![
        "pi-doctor explain throttling".to_owned(),
        "Firmware and thermal analysis".to_owned(),
    ];

    match throttling.raw_value {
        Some(raw) => lines.push(format!("  throttled bitmask: 0x{raw:x}")),
        None if throttling.vcgencmd_available && throttling.error.is_none() => {
            lines.push("  throttled bitmask: unavailable".to_owned())
        }
        _ => {}
    }

    match thermal.celsius {
        Some(celsius) => lines.push(format!(
            "  CPU temperature: {:.1} C ({})",
            celsius,
            thermal.band.map(|band| band.as_str()).unwrap_or("unknown")
        )),
        None => lines.push("  CPU temperature: unavailable".to_owned()),
    }

    lines.push(String::new());
    lines.push("Assessment".to_owned());

    if !throttling.vcgencmd_available {
        lines.push(
            "  `vcgencmd` is missing, so firmware throttle telemetry could not be read.".to_owned(),
        );
        lines.push("  Why this matters: under-voltage and firmware throttling history live in Raspberry Pi firmware telemetry.".to_owned());
        lines.push("  What to run next: install Raspberry Pi firmware utilities or run this on Raspberry Pi OS, then retry.".to_owned());
        return finish(lines);
    }

    if let Some(error) = &throttling.error {
        lines.push("  `vcgencmd get_throttled` returned unusable output.".to_owned());
        lines.push(format!("  Evidence: {error}"));
        if let ProbeError::MissingTool { .. } = error {
            lines.push(
                "  This looks like a missing firmware utility rather than a malformed firmware response."
                    .to_owned(),
            );
        }
        lines.push(
            "  What to run next: run `vcgencmd get_throttled` directly and compare the raw output."
                .to_owned(),
        );
        return finish(lines);
    }

    let mut problem_lines = throttle_problem_lines(&throttling);
    if problem_lines.is_empty() {
        problem_lines.push("  No active under-voltage or throttling flags are set.".to_owned());
    }
    lines.extend(problem_lines);

    if let Some(band) = thermal.band {
        lines.push(format!(
            "  Temperature classification: {}.",
            describe_temperature_band(band)
        ));
    }

    lines.push(String::new());
    lines.push("Next steps".to_owned());
    lines.extend(next_steps(&throttling, thermal.band));

    finish(lines)
}

fn throttle_problem_lines(details: &ThrottlingDetails) -> Vec<String> {
    let mut lines = Vec::new();

    if details.undervoltage_now {
        lines.push("  Under-voltage is active now.".to_owned());
    }
    if details.undervoltage_happened {
        lines.push("  Under-voltage occurred historically.".to_owned());
    }
    if details.throttled_now {
        lines.push("  Throttling is active.".to_owned());
    }
    if details.throttling_happened {
        lines.push("  Throttling occurred historically.".to_owned());
    }
    if details.soft_temperature_limit_now {
        lines.push("  Soft thermal limit is active.".to_owned());
    }
    if details.soft_temperature_limit_happened {
        lines.push("  Soft thermal limit occurred historically.".to_owned());
    }
    if details.arm_frequency_capped_now {
        lines.push("  ARM frequency capping is active.".to_owned());
    }
    if details.arm_frequency_capped_happened {
        lines.push("  ARM frequency capping occurred historically.".to_owned());
    }

    lines
}

fn describe_temperature_band(band: TemperatureBand) -> &'static str {
    match band {
        TemperatureBand::Normal => "normal",
        TemperatureBand::Warm => "warm",
        TemperatureBand::NearThrottle => "near throttle",
        TemperatureBand::ThrottlingLikely => "throttling likely",
    }
}

fn next_steps(details: &ThrottlingDetails, band: Option<TemperatureBand>) -> Vec<String> {
    let mut steps = Vec::new();

    if details.undervoltage_now || details.undervoltage_happened {
        steps.push("  Check the power supply rating, USB-C or micro-USB cable quality, and attached peripheral load.".to_owned());
    }
    if details.throttled_now || details.soft_temperature_limit_now {
        steps.push("  Improve cooling and airflow, then compare `vcgencmd get_throttled` before and after temperatures drop.".to_owned());
    }
    if matches!(
        band,
        Some(TemperatureBand::NearThrottle | TemperatureBand::ThrottlingLikely)
    ) {
        steps.push("  Watch `/sys/class/thermal/thermal_zone0/temp` during sustained load to confirm the heat trend.".to_owned());
    }
    if steps.is_empty() {
        steps.push("  No immediate remediation is suggested; rerun this command if you see performance drops under load.".to_owned());
    }

    steps
}

fn finish(lines: Vec<String>) -> String {
    format!("{}\n", lines.join("\n"))
}
