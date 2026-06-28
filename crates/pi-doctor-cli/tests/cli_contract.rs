use clap::Parser;
use pi_doctor::cli::args::{Cli, Commands, DoctorTarget};
use pi_doctor::output::RenderSettings;
use pi_doctor_core::{CommandOutput, ProbeContext};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn json_contract_exposes_expected_top_level_fields() {
    let output = pi_doctor::output::render_report(
        &pi_doctor::build_check_report(&fixture_ctx("non-pi-debian")),
        RenderSettings::test_json(),
    )
    .expect("fixture-backed json render should succeed");

    assert!(output.ends_with('\n'));

    let value: Value = serde_json::from_str(&output).expect("json should parse");
    let object = value.as_object().expect("top level should be an object");

    for key in [
        "metadata",
        "schema_version",
        "overall_status",
        "system",
        "config",
        "camera",
        "python",
        "probe_health",
        "groups",
        "findings",
    ] {
        assert!(object.contains_key(key), "missing top-level key `{key}`");
    }

    assert_eq!(value["schema_version"], "1.0.0");
    let probe_names = value["probe_health"]
        .as_array()
        .expect("probe health should be an array")
        .iter()
        .map(|health| health["name"].as_str().expect("probe name should be string"))
        .collect::<Vec<_>>();
    let mut sorted_probe_names = probe_names.clone();
    sorted_probe_names.sort_unstable();
    assert_eq!(probe_names, sorted_probe_names);
}

#[test]
fn non_check_commands_return_zero_on_success() {
    for command in [
        Commands::SupportBundle {
            output: ".".into(),
            dry_run: true,
            include_sensitive: false,
            acknowledge_sensitive_data: false,
        },
        Commands::Completions {
            shell: clap_complete::Shell::Bash,
        },
    ] {
        let response = pi_doctor::run(Cli {
            json: false,
            quiet: false,
            verbose: false,
            no_color: true,
            timeout: 3,
            command,
        })
        .expect("command should succeed");

        assert_eq!(response.exit_code, 0);
    }
}

#[test]
fn check_accepts_json_mode() {
    let cli = Cli::try_parse_from(["pi-doctor", "--json", "check"])
        .and_then(Cli::validate)
        .expect("json check should be accepted");

    assert!(cli.json);
    assert!(matches!(cli.command, Commands::Check {}));
}

#[test]
fn doctor_camera_accepts_json_mode() {
    let response = pi_doctor::run(Cli {
        json: true,
        quiet: false,
        verbose: false,
        no_color: false,
        timeout: 1,
        command: Commands::Doctor {
            target: DoctorTarget::Camera,
        },
    })
    .expect("doctor camera json should render");

    let value: Value = serde_json::from_str(&response.output).expect("json should parse");
    assert_eq!(value["schema_version"], "1.0.0");
    assert_eq!(value["target"], "camera");
    assert_eq!(value["metadata"]["command"], "doctor camera");
    assert_eq!(response.exit_code, 0);
}

#[test]
fn support_bundle_dry_run_lists_files_without_writing() {
    let response = pi_doctor::run(Cli {
        json: true,
        quiet: false,
        verbose: false,
        no_color: false,
        timeout: 1,
        command: Commands::SupportBundle {
            output: ".".into(),
            dry_run: true,
            include_sensitive: false,
            acknowledge_sensitive_data: false,
        },
    })
    .expect("support bundle dry-run json should render");

    let value: Value = serde_json::from_str(&response.output).expect("json should parse");
    assert_eq!(value["dry_run"], true);
    assert_eq!(value["privacy_mode"], "sanitized");
    assert_eq!(value["redaction_enabled"], true);
    assert!(value["files"].as_array().expect("files should be an array").iter().any(
        |path| path == "manifest.txt"
    ));
    assert_eq!(response.exit_code, 0);
}

#[test]
fn check_probes_do_not_write_to_fixture_root() {
    let root = temp_fixture_root();
    write_fixture_file(
        &root,
        "proc/device-tree/model",
        "Raspberry Pi 4 Model B Rev 1.5\0",
    );
    write_fixture_file(
        &root,
        "proc/cpuinfo",
        "Hardware\t: BCM2711\nRevision\t: c03115\n",
    );
    write_fixture_file(&root, "proc/sys/kernel/osrelease", "6.6.31-v8+\n");
    write_fixture_file(
        &root,
        "etc/os-release",
        "NAME=\"Debian GNU/Linux\"\nVERSION_ID=\"12\"\nVERSION_CODENAME=bookworm\n",
    );
    write_fixture_file(&root, "boot/firmware/config.txt", "dtparam=i2c_arm=on\n");
    write_fixture_file(&root, "sys/class/thermal/thermal_zone0/temp", "42123\n");

    let before = tree_snapshot(&root);
    let ctx = ProbeContext::with_root(&root)
        .with_command_output(
            "vcgencmd",
            &["get_throttled"],
            CommandOutput::Success("throttled=0x0".to_owned()),
        )
        .with_command_output("rpicam-hello", &["--help"], CommandOutput::Missing)
        .with_command_output("libcamera-hello", &["--help"], CommandOutput::Missing)
        .with_command_output("python3", &["--version"], CommandOutput::Missing);

    let _report = pi_doctor::build_check_report(&ctx);

    assert_eq!(tree_snapshot(&root), before);
    let _ = fs::remove_dir_all(root);
}

fn fixture_ctx(name: &str) -> ProbeContext {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join("milestone1")
        .join(name);

    ProbeContext::with_root(root)
        .with_command_output("vcgencmd", &["get_throttled"], CommandOutput::Missing)
        .with_command_output("rpicam-hello", &["--help"], CommandOutput::Missing)
        .with_command_output("libcamera-hello", &["--help"], CommandOutput::Missing)
        .with_command_output("python3", &["--version"], CommandOutput::Missing)
        .with_command_output(
            "python3",
            &["-c", "import sys; print(sys.executable)"],
            CommandOutput::Missing,
        )
        .with_command_output(
            "python3",
            &[
                "-c",
                "import sys; print(int(sys.prefix != sys.base_prefix))",
            ],
            CommandOutput::Missing,
        )
        .with_command_output(
            "python3",
            &[
                "-c",
                "import sysconfig; print(sysconfig.get_path('stdlib'))",
            ],
            CommandOutput::Missing,
        )
        .with_command_output(
            "dpkg-query",
            &["-W", "-f=${Status}", "python3-picamera2"],
            CommandOutput::Missing,
        )
        .with_command_output(
            "dpkg-query",
            &["-W", "-f=${Status}", "python3-gpiozero"],
            CommandOutput::Missing,
        )
}

fn temp_fixture_root() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after Unix epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("pi-doctor-read-only-{nanos}"));
    let _ = fs::remove_dir_all(&root);
    root
}

fn write_fixture_file(root: &Path, relative: &str, contents: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().expect("fixture path should have parent"))
        .expect("fixture parent should be created");
    fs::write(path, contents).expect("fixture file should be written");
}

fn tree_snapshot(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut snapshot = BTreeMap::new();
    collect_tree(root, root, &mut snapshot);
    snapshot
}

fn collect_tree(root: &Path, path: &Path, snapshot: &mut BTreeMap<PathBuf, Vec<u8>>) {
    let mut entries = fs::read_dir(path)
        .expect("fixture directory should be readable")
        .collect::<Result<Vec<_>, _>>()
        .expect("fixture entries should be readable");
    entries.sort_by_key(|entry| entry.path());

    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            collect_tree(root, &path, snapshot);
        } else {
            let relative = path
                .strip_prefix(root)
                .expect("fixture path should stay under root")
                .to_path_buf();
            snapshot.insert(
                relative,
                fs::read(path).expect("fixture file should be readable"),
            );
        }
    }
}
