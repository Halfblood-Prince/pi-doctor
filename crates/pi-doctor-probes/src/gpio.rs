use crate::config_txt::ConfigTxtProbe;
use pi_doctor_core::{CommandOutput, Finding, Probe, ProbeContext, ProbeResult, Severity};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GpioAnalysis {
    pub pinctrl_present: bool,
    pub raspi_gpio_present: bool,
    pub gpioinfo_present: bool,
    pub gpiodetect_present: bool,
    pub gpiochips: Vec<String>,
    pub overlay_hints: Vec<String>,
    pub alternate_functions: Vec<PinFunction>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PinFunction {
    pub pin: u32,
    pub function: String,
}

pub struct GpioProbe;

impl GpioProbe {
    pub fn collect(&self, ctx: &ProbeContext) -> GpioAnalysis {
        let pinctrl_output = ctx.run_command("pinctrl", &["--help"]);
        let raspi_gpio_output = ctx.run_command("raspi-gpio", &["help"]);
        let gpioinfo_output = ctx.run_command("gpioinfo", &["--help"]);
        let gpiodetect_output = ctx.run_command("gpiodetect", &["--help"]);
        let pinctrl_state = ctx.run_command("pinctrl", &[]);

        GpioAnalysis {
            pinctrl_present: is_present(&pinctrl_output),
            raspi_gpio_present: is_present(&raspi_gpio_output),
            gpioinfo_present: is_present(&gpioinfo_output),
            gpiodetect_present: is_present(&gpiodetect_output),
            gpiochips: ctx
                .list_dir("/dev")
                .into_iter()
                .filter(|name| name.starts_with("gpiochip"))
                .collect(),
            overlay_hints: config_overlay_hints(ctx),
            alternate_functions: match pinctrl_state {
                CommandOutput::Success(output) | CommandOutput::Failure(output) => {
                    parse_pinctrl_functions(&output)
                }
                CommandOutput::Missing => Vec::new(),
            },
        }
    }
}

impl Probe for GpioProbe {
    fn run(&self, ctx: &ProbeContext) -> ProbeResult {
        let analysis = self.collect(ctx);
        let mut findings = Vec::new();

        if analysis.pinctrl_present {
            findings.push(Finding {
                id: "gpio.pinctrl_present",
                severity: Severity::Info,
                title: "Use pinctrl for hardware-state inspection".to_owned(),
                summary: "The `pinctrl` tool is available for Raspberry Pi-specific pin state inspection.".to_owned(),
                evidence: vec!["tool detected: pinctrl".to_owned()],
                suggested_actions: vec![
                    "Why this matters: `pinctrl` is the modern Raspberry Pi tool for inspecting low-level pin mux and hardware state.".to_owned(),
                    "What to run next: use `pinctrl` when you need Raspberry Pi-specific alternate-function visibility.".to_owned(),
                ],
            });
        }

        if analysis.raspi_gpio_present {
            findings.push(Finding {
                id: "gpio.raspi_gpio_deprecated",
                severity: Severity::Warning,
                title: "`raspi-gpio` is present but deprecated".to_owned(),
                summary: "`raspi-gpio` still exists on this system, but newer Raspberry Pi guidance favors `pinctrl`.".to_owned(),
                evidence: vec!["tool detected: raspi-gpio".to_owned()],
                suggested_actions: vec![
                    "Why this matters: following older GPIO advice can send you to tools that no longer match the current Raspberry Pi stack.".to_owned(),
                    "What to run next: prefer `pinctrl` for Pi-specific inspection and `libgpiod` tools for generic Linux GPIO work.".to_owned(),
                ],
            });
        }

        if analysis.gpioinfo_present || analysis.gpiodetect_present {
            findings.push(Finding {
                id: "gpio.libgpiod_present",
                severity: Severity::Info,
                title: "Use libgpiod for generic Linux GPIO line work".to_owned(),
                summary: "At least one libgpiod CLI tool is available for userspace GPIO line inspection.".to_owned(),
                evidence: vec![format!(
                    "tools detected: {}{}",
                    if analysis.gpioinfo_present { "gpioinfo" } else { "" },
                    if analysis.gpiodetect_present {
                        if analysis.gpioinfo_present {
                            ", gpiodetect"
                        } else {
                            "gpiodetect"
                        }
                    } else {
                        ""
                    }
                )],
                suggested_actions: vec![
                    "Why this matters: libgpiod tools follow the generic Linux GPIO character-device model instead of Raspberry Pi-specific firmware views.".to_owned(),
                    "What to run next: use `gpiodetect` and `gpioinfo` when you need chip and line ownership details.".to_owned(),
                ],
            });
        } else {
            findings.push(Finding {
                id: "gpio.no_libgpiod_tools",
                severity: Severity::Warning,
                title: "No libgpiod GPIO CLI tools were detected".to_owned(),
                summary: "Neither `gpioinfo` nor `gpiodetect` appears to be available on this system.".to_owned(),
                evidence: vec!["tools missing: gpioinfo, gpiodetect".to_owned()],
                suggested_actions: vec![
                    "Why this matters: without libgpiod tools, generic Linux GPIO ownership and line-state inspection gets much harder.".to_owned(),
                    "What to run next: install libgpiod tools and rerun `pi-doctor doctor gpio`.".to_owned(),
                ],
            });
        }

        for hint in &analysis.overlay_hints {
            findings.push(Finding {
                id: "gpio.overlay_claims_bus",
                severity: Severity::Warning,
                title: "An overlay likely claims a GPIO-backed peripheral".to_owned(),
                summary: format!("The boot config suggests `{hint}` may already enable a bus or peripheral."),
                evidence: vec![format!("overlay hint: {hint}")],
                suggested_actions: vec![
                    "Why this matters: overlays can reserve pins for alternate functions before your userspace GPIO tooling ever sees them as free.".to_owned(),
                    "What to run next: compare the overlay intent with `pinctrl` or `gpioinfo` before repurposing those pins.".to_owned(),
                ],
            });
        }

        if let Some(pin) = analysis.alternate_functions.first() {
            findings.push(Finding {
                id: "gpio.pin_owned_by_alt_function",
                severity: Severity::Warning,
                title: format!("GPIO{} appears owned by {}", pin.pin, pin.function),
                summary: "The current pinctrl output suggests at least one pin is already muxed to an alternate hardware function.".to_owned(),
                evidence: vec![format!("GPIO{} = {}", pin.pin, pin.function)],
                suggested_actions: vec![
                    "Why this matters: pins already assigned to SPI, I2C, UART, PWM, or other functions are not safely available for ad-hoc GPIO use.".to_owned(),
                    "What to run next: inspect the owning overlay, driver, or hardware function before trying to toggle that pin from userspace.".to_owned(),
                ],
            });
        }

        findings
    }
}

fn is_present(output: &CommandOutput) -> bool {
    !matches!(output, CommandOutput::Missing)
}

fn config_overlay_hints(ctx: &ProbeContext) -> Vec<String> {
    let analysis = ConfigTxtProbe.collect(ctx);
    let mut hints = Vec::new();

    for entry in analysis.summary.entries {
        if entry.key.as_deref() != Some("dtoverlay") {
            continue;
        }

        if let Some(value) = entry.value {
            let lower = value.to_ascii_lowercase();
            if lower.contains("spi") || lower.contains("i2c") || lower.contains("uart") {
                hints.push(value);
            }
        }
    }

    hints.sort();
    hints.dedup();
    hints
}

pub fn parse_pinctrl_functions(output: &str) -> Vec<PinFunction> {
    let mut functions = Vec::new();

    for line in output.lines() {
        let Some((left, right)) = line.split_once("//") else {
            continue;
        };
        let Some(pin) = left
            .split(':')
            .next()
            .and_then(|pin| pin.trim().parse::<u32>().ok())
        else {
            continue;
        };
        let Some((gpio_label, function)) = right.split_once('=') else {
            continue;
        };
        if !gpio_label.contains("GPIO") {
            continue;
        }

        let function = function.trim();
        let upper = function.to_ascii_uppercase();
        if matches!(upper.as_str(), "INPUT" | "OUTPUT" | "NONE") {
            continue;
        }

        functions.push(PinFunction {
            pin,
            function: function.to_owned(),
        });
    }

    functions
}

#[cfg(test)]
mod tests {
    use super::parse_pinctrl_functions;

    #[test]
    fn parses_pi4_style_pinctrl_output() {
        let functions = parse_pinctrl_functions(
            "3: a0 pd | hi // GPIO3 = SDA1\n4: ip pn | lo // GPIO4 = INPUT\n",
        );

        assert_eq!(functions.len(), 1);
        assert_eq!(functions[0].pin, 3);
        assert_eq!(functions[0].function, "SDA1");
    }

    #[test]
    fn parses_pi5_style_pinctrl_output() {
        let functions = parse_pinctrl_functions(
            "18: a3 pd | hi // GPIO18 = PWM0_CHAN2\n19: no pd | lo // GPIO19 = NONE\n",
        );

        assert_eq!(functions.len(), 1);
        assert_eq!(functions[0].pin, 18);
        assert_eq!(functions[0].function, "PWM0_CHAN2");
    }
}
