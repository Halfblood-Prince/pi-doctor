use insta::assert_snapshot;
use pi_doctor_core::{CommandOutput, ProbeContext};
use std::path::PathBuf;

#[test]
fn doctor_gpio_snapshot_with_tools_and_alt_functions() {
    let ctx = fixture_ctx("gpio-ready")
        .with_command_output(
            "pinctrl",
            &["--help"],
            CommandOutput::Success("usage".to_owned()),
        )
        .with_command_output(
            "pinctrl",
            &[],
            CommandOutput::Success(
                "3: a0 pd | hi // GPIO3 = SDA1\n18: a3 pd | hi // GPIO18 = PWM0_CHAN2\n".to_owned(),
            ),
        )
        .with_command_output(
            "gpioinfo",
            &["--help"],
            CommandOutput::Success("usage".to_owned()),
        )
        .with_command_output(
            "gpiodetect",
            &["--help"],
            CommandOutput::Success("usage".to_owned()),
        )
        .with_command_output(
            "raspi-gpio",
            &["help"],
            CommandOutput::Success("usage".to_owned()),
        );
    let output = pi_doctor::doctor::gpio::render(&ctx);

    assert!(output.contains("pinctrl: present"));
    assert!(output.contains("raspi-gpio: present (deprecated)"));
    assert!(output.contains("GPIO3 currently appears owned by alternate function SDA1."));
    assert_snapshot!("doctor_gpio_ready", output);
}

#[test]
fn doctor_gpio_snapshot_without_tools() {
    let ctx = fixture_ctx("gpio-empty")
        .with_command_output("pinctrl", &["--help"], CommandOutput::Missing)
        .with_command_output("pinctrl", &[], CommandOutput::Missing)
        .with_command_output("gpioinfo", &["--help"], CommandOutput::Missing)
        .with_command_output("gpiodetect", &["--help"], CommandOutput::Missing)
        .with_command_output("raspi-gpio", &["help"], CommandOutput::Missing);
    let output = pi_doctor::doctor::gpio::render(&ctx);

    assert!(output.contains("pinctrl: missing"));
    assert!(output.contains("libgpiod tools: missing"));
    assert!(output.contains("GPIO inspection tooling is sparse here"));
    assert_snapshot!("doctor_gpio_missing_tools", output);
}

fn fixture_ctx(name: &str) -> ProbeContext {
    ProbeContext::with_root(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("fixtures")
            .join("milestone4")
            .join(name),
    )
}
