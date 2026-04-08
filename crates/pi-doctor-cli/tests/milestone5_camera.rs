use insta::assert_snapshot;
use pi_doctor_core::{CommandOutput, ProbeContext};
use std::path::PathBuf;

#[test]
fn doctor_camera_detected_ready_snapshot() {
    let ctx = fixture_ctx("camera-ready")
        .with_command_output(
            "rpicam-hello",
            &["--help"],
            CommandOutput::Success("usage".to_owned()),
        )
        .with_command_output(
            "rpicam-hello",
            &["--list-cameras"],
            CommandOutput::Success(
                "Available cameras\n-----------------\n0 : imx219 [3280x2464 10-bit]\n    /base/soc/i2c0mux/i2c@1/imx219@10\n".to_owned(),
            ),
        )
        .with_command_output(
            "libcamera-hello",
            &["--help"],
            CommandOutput::Missing,
        );

    assert_snapshot!(
        "doctor_camera_ready",
        pi_doctor::doctor::camera::render(&ctx)
    );
}

#[test]
fn doctor_camera_no_cameras_snapshot() {
    let ctx = fixture_ctx("no-camera")
        .with_command_output(
            "rpicam-hello",
            &["--help"],
            CommandOutput::Success("usage".to_owned()),
        )
        .with_command_output(
            "rpicam-hello",
            &["--list-cameras"],
            CommandOutput::Success("Available cameras\n-----------------\n".to_owned()),
        )
        .with_command_output("libcamera-hello", &["--help"], CommandOutput::Missing);

    assert_snapshot!(
        "doctor_camera_no_cameras",
        pi_doctor::doctor::camera::render(&ctx)
    );
}

#[test]
fn doctor_camera_missing_tools_snapshot() {
    let ctx = ProbeContext::new()
        .with_command_output("rpicam-hello", &["--help"], CommandOutput::Missing)
        .with_command_output("rpicam-hello", &["--list-cameras"], CommandOutput::Missing)
        .with_command_output("libcamera-hello", &["--help"], CommandOutput::Missing)
        .with_command_output(
            "libcamera-hello",
            &["--list-cameras"],
            CommandOutput::Missing,
        );

    assert_snapshot!(
        "doctor_camera_missing_tools",
        pi_doctor::doctor::camera::render(&ctx)
    );
}

fn fixture_ctx(name: &str) -> ProbeContext {
    ProbeContext::with_root(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("fixtures")
            .join("milestone5")
            .join(name),
    )
}
