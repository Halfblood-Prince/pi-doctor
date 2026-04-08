use insta::assert_snapshot;
use pi_doctor_core::{CommandOutput, ProbeContext};
use std::path::PathBuf;

#[test]
fn explain_python_snapshot_for_externally_managed_system_python() {
    let ctx = fixture_ctx("bookworm-python")
        .with_command_output(
            "python3",
            &["--version"],
            CommandOutput::Success("Python 3.11.2".to_owned()),
        )
        .with_command_output(
            "python3",
            &["-c", "import sys; print(sys.executable)"],
            CommandOutput::Success("/usr/bin/python3".to_owned()),
        )
        .with_command_output(
            "python3",
            &[
                "-c",
                "import sys; print(int(sys.prefix != sys.base_prefix))",
            ],
            CommandOutput::Success("0".to_owned()),
        )
        .with_command_output(
            "python3",
            &[
                "-c",
                "import sysconfig; print(sysconfig.get_path('stdlib'))",
            ],
            CommandOutput::Success("/usr/lib/python3.11".to_owned()),
        )
        .with_command_output(
            "dpkg-query",
            &["-W", "-f=${Status}", "python3-picamera2"],
            CommandOutput::Success("install ok installed".to_owned()),
        )
        .with_command_output(
            "dpkg-query",
            &["-W", "-f=${Status}", "python3-gpiozero"],
            CommandOutput::Failure("package not installed".to_owned()),
        );

    assert_snapshot!(
        "explain_python_externally_managed",
        pi_doctor::explain::python::render(&ctx)
    );
}

#[test]
fn explain_python_snapshot_for_active_venv() {
    let ctx = ProbeContext::new()
        .with_command_output(
            "python3",
            &["--version"],
            CommandOutput::Success("Python 3.12.1".to_owned()),
        )
        .with_command_output(
            "python3",
            &["-c", "import sys; print(sys.executable)"],
            CommandOutput::Success("/home/pi/project/.venv/bin/python3".to_owned()),
        )
        .with_command_output(
            "python3",
            &[
                "-c",
                "import sys; print(int(sys.prefix != sys.base_prefix))",
            ],
            CommandOutput::Success("1".to_owned()),
        )
        .with_command_output(
            "python3",
            &[
                "-c",
                "import sysconfig; print(sysconfig.get_path('stdlib'))",
            ],
            CommandOutput::Success("/home/pi/project/.venv/lib/python3.12".to_owned()),
        )
        .with_command_output(
            "dpkg-query",
            &["-W", "-f=${Status}", "python3-picamera2"],
            CommandOutput::Failure("package not installed".to_owned()),
        )
        .with_command_output(
            "dpkg-query",
            &["-W", "-f=${Status}", "python3-gpiozero"],
            CommandOutput::Failure("package not installed".to_owned()),
        );

    assert_snapshot!(
        "explain_python_active_venv",
        pi_doctor::explain::python::render(&ctx)
    );
}

fn fixture_ctx(name: &str) -> ProbeContext {
    ProbeContext::with_root(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("fixtures")
            .join("milestone6")
            .join(name),
    )
}
