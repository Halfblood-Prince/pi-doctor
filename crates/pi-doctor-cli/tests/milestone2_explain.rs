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

    assert_snapshot!(
        "explain_throttling_active",
        pi_doctor::explain::throttling::render(&ctx)
    );
}

#[test]
fn explain_throttling_snapshot_without_vcgencmd() {
    let ctx = ProbeContext::new().with_command_output(
        "vcgencmd",
        &["get_throttled"],
        CommandOutput::Missing,
    );

    assert_snapshot!(
        "explain_throttling_missing_vcgencmd",
        pi_doctor::explain::throttling::render(&ctx)
    );
}

fn fixture_root(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join("milestone2")
        .join(name)
}
