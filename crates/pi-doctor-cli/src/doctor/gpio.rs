use pi_doctor_core::ProbeContext;
use pi_doctor_probes::gpio::GpioProbe;

pub fn render(ctx: &ProbeContext) -> String {
    let analysis = GpioProbe.collect(ctx).unwrap_or_default();
    let mut lines = vec![
        "pi-doctor doctor gpio".to_owned(),
        "GPIO stack recommendation".to_owned(),
        format!(
            "  gpiochip devices: {}",
            if analysis.gpiochips.is_empty() {
                "none detected".to_owned()
            } else {
                analysis.gpiochips.join(", ")
            }
        ),
        format!(
            "  pinctrl: {}",
            if analysis.pinctrl_present {
                "present"
            } else {
                "missing"
            }
        ),
        format!(
            "  libgpiod tools: {}",
            match (analysis.gpioinfo_present, analysis.gpiodetect_present) {
                (true, true) => "gpioinfo, gpiodetect".to_owned(),
                (true, false) => "gpioinfo".to_owned(),
                (false, true) => "gpiodetect".to_owned(),
                (false, false) => "missing".to_owned(),
            }
        ),
    ];

    if analysis.raspi_gpio_present {
        lines.push("  raspi-gpio: present (deprecated)".to_owned());
    }

    lines.push(String::new());
    lines.push("Recommended path".to_owned());
    if analysis.pinctrl_present {
        lines.push(
            "  Use pinctrl for Raspberry Pi-specific hardware-state inspection and alternate-function visibility."
                .to_owned(),
        );
    }
    if analysis.gpioinfo_present || analysis.gpiodetect_present {
        lines.push(
            "  Use libgpiod tools for generic Linux GPIO line ownership and userspace line work."
                .to_owned(),
        );
    }
    if !analysis.pinctrl_present && !analysis.gpioinfo_present && !analysis.gpiodetect_present {
        lines.push(
            "  GPIO inspection tooling is sparse here; install pinctrl or libgpiod tools before deep GPIO debugging."
                .to_owned(),
        );
    }

    if !analysis.overlay_hints.is_empty() || !analysis.alternate_functions.is_empty() {
        lines.push(String::new());
        lines.push("Potential conflicts".to_owned());
        for hint in &analysis.overlay_hints {
            lines.push(format!(
                "  Overlay `{hint}` likely enables a bus or peripheral already."
            ));
        }
        for pin in analysis.alternate_functions.iter().take(3) {
            lines.push(format!(
                "  GPIO{} currently appears owned by alternate function {}.",
                pin.pin, pin.function
            ));
        }
    }

    format!("{}\n", lines.join("\n"))
}
