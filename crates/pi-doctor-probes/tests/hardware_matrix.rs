use pi_doctor_core::{CommandOutput, ProbeContext};
use pi_doctor_probes::{
    board::BoardProbe,
    camera::{CameraProbe, parse_camera_inventory},
    config_txt::ConfigTxtProbe,
    gpio::{GpioProbe, parse_pinctrl_functions},
    kernel::KernelProbe,
    os::OsProbe,
    python::PythonProbe,
    thermal::{TemperatureBand, ThermalProbe, classify_temperature, parse_thermal_millidegrees},
    throttling::{ThrottlingProbe, parse_throttled_output},
};
use std::fs;
use std::path::PathBuf;

#[test]
fn pi4_lite_fixture_covers_identity_config_gpio_and_empty_camera_inventory() {
    let root = fixture_root("pi4-bookworm-lite-no-camera");
    let ctx = ProbeContext::with_root(&root)
        .with_command_output(
            "pinctrl",
            &["--help"],
            CommandOutput::Success("usage".to_owned()),
        )
        .with_command_output("raspi-gpio", &["help"], CommandOutput::Missing)
        .with_command_output("gpioinfo", &["--help"], CommandOutput::Missing)
        .with_command_output("gpiodetect", &["--help"], CommandOutput::Missing)
        .with_command_output(
            "pinctrl",
            &[],
            CommandOutput::Success(capture("pi4-bookworm-lite-no-camera", "pinctrl.txt")),
        )
        .with_command_output(
            "rpicam-hello",
            &["--help"],
            CommandOutput::Success("usage".to_owned()),
        )
        .with_command_output("libcamera-hello", &["--help"], CommandOutput::Missing)
        .with_command_output(
            "rpicam-hello",
            &["--list-cameras"],
            CommandOutput::Success(capture(
                "pi4-bookworm-lite-no-camera",
                "rpicam-hello-list-cameras.txt",
            )),
        );

    let board = BoardProbe
        .collect(&ctx)
        .expect("board fixture should parse");
    let os = OsProbe.collect(&ctx).expect("os fixture should parse");
    let kernel = KernelProbe
        .collect(&ctx)
        .expect("kernel fixture should parse");
    let config = ConfigTxtProbe
        .collect(&ctx)
        .expect("config fixture should parse");
    let gpio = GpioProbe.collect(&ctx).expect("gpio fixture should parse");
    let camera = CameraProbe
        .collect(&ctx)
        .expect("camera fixture should parse");
    let thermal = ThermalProbe
        .collect(&ctx)
        .expect("thermal fixture should parse");

    assert_eq!(
        board.model.as_deref(),
        Some("Raspberry Pi 4 Model B Rev 1.5")
    );
    assert!(board.is_raspberry_pi);
    assert_eq!(os.distro_codename.as_deref(), Some("bookworm"));
    assert_eq!(kernel.architecture.as_deref(), Some("aarch64"));
    assert!(config.summary.using_firmware_path);
    assert_eq!(config.summary.entries.len(), 4);
    assert_eq!(gpio.overlay_hints.len(), 0);
    assert_eq!(gpio.alternate_functions.len(), 2);
    assert!(camera.summary.cameras.is_empty());
    assert_eq!(thermal.band, Some(TemperatureBand::Normal));
}

#[test]
fn pi5_desktop_camera_fixture_covers_camera_python_and_gpio_matrix_paths() {
    let root = fixture_root("pi5-bookworm-desktop-camera");
    let ctx = ProbeContext::with_root(&root)
        .with_command_output(
            "pinctrl",
            &["--help"],
            CommandOutput::Success("usage".to_owned()),
        )
        .with_command_output(
            "raspi-gpio",
            &["help"],
            CommandOutput::Success("usage".to_owned()),
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
            "pinctrl",
            &[],
            CommandOutput::Success(capture("pi5-bookworm-desktop-camera", "pinctrl.txt")),
        )
        .with_command_output(
            "rpicam-hello",
            &["--help"],
            CommandOutput::Success("usage".to_owned()),
        )
        .with_command_output(
            "libcamera-hello",
            &["--help"],
            CommandOutput::Success("usage".to_owned()),
        )
        .with_command_output(
            "rpicam-hello",
            &["--list-cameras"],
            CommandOutput::Success(capture(
                "pi5-bookworm-desktop-camera",
                "rpicam-hello-list-cameras.txt",
            )),
        )
        .with_command_output(
            "python3",
            &["--version"],
            CommandOutput::Success(capture("pi5-bookworm-desktop-camera", "python-version.txt")),
        )
        .with_command_output(
            "python3",
            &["-c", "import sys; print(sys.executable)"],
            CommandOutput::Success(capture(
                "pi5-bookworm-desktop-camera",
                "python-executable.txt",
            )),
        )
        .with_command_output(
            "python3",
            &[
                "-c",
                "import sys; print(int(sys.prefix != sys.base_prefix))",
            ],
            CommandOutput::Success(capture(
                "pi5-bookworm-desktop-camera",
                "python-venv-flag.txt",
            )),
        )
        .with_command_output(
            "python3",
            &[
                "-c",
                "import sysconfig; print(sysconfig.get_path('stdlib'))",
            ],
            CommandOutput::Success(capture("pi5-bookworm-desktop-camera", "python-stdlib.txt")),
        )
        .with_command_output(
            "dpkg-query",
            &["-W", "-f=${Status}", "python3-picamera2"],
            CommandOutput::Success(capture("pi5-bookworm-desktop-camera", "dpkg-picamera2.txt")),
        )
        .with_command_output(
            "dpkg-query",
            &["-W", "-f=${Status}", "python3-gpiozero"],
            CommandOutput::Success(capture("pi5-bookworm-desktop-camera", "dpkg-gpiozero.txt")),
        );

    let camera = CameraProbe
        .collect(&ctx)
        .expect("camera fixture should parse");
    let gpio = GpioProbe.collect(&ctx).expect("gpio fixture should parse");
    let python = PythonProbe
        .collect(&ctx)
        .expect("python fixture should parse");
    let thermal = ThermalProbe
        .collect(&ctx)
        .expect("thermal fixture should parse");

    assert_eq!(camera.summary.cameras.len(), 1);
    assert_eq!(camera.summary.cameras[0].name, "imx708_wide");
    assert_eq!(camera.summary.video_devices, vec!["video0".to_owned()]);
    assert!(gpio.pinctrl_present);
    assert!(gpio.raspi_gpio_present);
    assert_eq!(gpio.overlay_hints, vec!["i2c0,pins_44_45".to_owned()]);
    assert_eq!(python.summary.version.as_deref(), Some("Python 3.11.2"));
    assert!(python.summary.externally_managed);
    assert_eq!(
        python.summary.detected_packages,
        vec![
            "python3-picamera2".to_owned(),
            "python3-gpiozero".to_owned()
        ]
    );
    assert_eq!(thermal.band, Some(TemperatureBand::Warm));
}

#[test]
fn stressed_fixture_covers_throttling_and_spacing_tolerant_parsers() {
    let root = fixture_root("pi5-stressed-lab-rig");
    let ctx = ProbeContext::with_root(&root).with_command_output(
        "vcgencmd",
        &["get_throttled"],
        CommandOutput::Success(capture(
            "pi5-stressed-lab-rig",
            "vcgencmd-get_throttled.txt",
        )),
    );

    let throttling = ThrottlingProbe
        .collect(&ctx)
        .expect("throttling fixture should parse");
    let thermal = ThermalProbe
        .collect(&ctx)
        .expect("thermal fixture should parse");
    let pin_functions = parse_pinctrl_functions(&capture("pi5-stressed-lab-rig", "pinctrl.txt"));

    assert!(throttling.undervoltage_now);
    assert!(throttling.throttled_now);
    assert!(throttling.undervoltage_happened);
    assert!(throttling.throttling_happened);
    assert_eq!(thermal.band, Some(TemperatureBand::ThrottlingLikely));
    assert_eq!(pin_functions.len(), 3);
    assert_eq!(pin_functions[1].function, "SPI0_MISO");
}

#[test]
fn raw_capture_parsers_are_exercised_from_fixture_files() {
    let clear = parse_throttled_output(&capture(
        "pi5-bookworm-desktop-camera",
        "vcgencmd-get_throttled.txt",
    ))
    .expect("desktop throttle capture should parse");
    assert_eq!(clear.raw_value, Some(0));

    let active = parse_throttled_output(&capture(
        "pi5-stressed-lab-rig",
        "vcgencmd-get_throttled.txt",
    ))
    .expect("stressed throttle capture should parse");
    assert!(active.throttled_now);

    let pi5_cameras = parse_camera_inventory(&capture(
        "pi5-bookworm-desktop-camera",
        "libcamera-hello-list-cameras.txt",
    ));
    assert_eq!(
        pi5_cameras[0].mode_hint.as_deref(),
        Some("Modes: 'SRGGB10_CSI2P' : 2304x1296 [30.00 fps]")
    );

    let no_camera = parse_camera_inventory(&capture(
        "pi4-bookworm-lite-no-camera",
        "rpicam-hello-list-cameras.txt",
    ));
    assert!(no_camera.is_empty());

    let pi4_temp = parse_thermal_millidegrees(
        &fs::read_to_string(
            fixture_root("pi4-bookworm-lite-no-camera")
                .join("sys")
                .join("class")
                .join("thermal")
                .join("thermal_zone0")
                .join("temp"),
        )
        .expect("pi4 temperature fixture should read"),
    )
    .expect("pi4 temperature should parse")
    .expect("pi4 temperature should exist");
    assert_eq!(classify_temperature(pi4_temp), TemperatureBand::Normal);
}

fn fixture_root(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join("hardware-matrix")
        .join(name)
}

fn capture(fixture: &str, file: &str) -> String {
    fs::read_to_string(fixture_root(fixture).join("captures").join(file))
        .expect("fixture capture should exist")
}
