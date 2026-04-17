use insta::assert_snapshot;
use pi_doctor_core::{CommandOutput, ProbeContext};
use std::path::PathBuf;

#[test]
fn explain_throttling_snapshot_with_active_flags() {
    let ctx = ProbeContext::with_root(fixture_root("hot-system")).with_command_output(
        "vcgencmd",
        &["get_throttled"],
        CommandOutput::Success("throttled=0x50005".to_owned()),
    );
    let output = pi_doctor::explain::throttling::render(&ctx);

    assert!(output.contains("Under-voltage is active now."));
    assert!(output.contains("Throttling is active."));
    assert!(output.contains("CPU temperature:"));
    assert_snapshot!("explain_throttling_active", output);
}

#[test]
fn explain_throttling_snapshot_without_vcgencmd() {
    let ctx = ProbeContext::new().with_command_output(
        "vcgencmd",
        &["get_throttled"],
        CommandOutput::Missing,
    );
    let output = pi_doctor::explain::throttling::render(&ctx);

    assert!(output.contains("`vcgencmd` is missing"));
    assert!(output.contains("firmware throttle telemetry could not be read"));
    assert_snapshot!("explain_throttling_missing_vcgencmd", output);
}

fn fixture_root(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join("milestone2")
        .join(name)
}
