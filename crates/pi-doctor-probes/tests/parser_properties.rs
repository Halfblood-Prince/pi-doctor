use pi_doctor_core::{CommandOutput, ProbeContext};
use pi_doctor_probes::{
    camera::parse_camera_inventory, config_txt::ConfigTxtProbe, gpio::parse_pinctrl_functions,
    thermal::parse_thermal_millidegrees, throttling::parse_throttled_output,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn parser_corpus_does_not_panic_on_malformed_inputs() {
    let mut corpus = vec![
        "".to_owned(),
        "\0\0\0".to_owned(),
        "not valid".to_owned(),
        "throttled=".to_owned(),
        "throttled=0x".to_owned(),
        "Available cameras\n[not-an-index] broken\n".to_owned(),
        "18: ??? // GPIO18 = PWM0_CHAN2\n".to_owned(),
        "[all]\ndtoverlay\nmalformed\n".to_owned(),
    ];
    corpus.push("x".repeat(4096));

    for input in &corpus {
        let _ = parse_throttled_output(input);
        let _ = parse_camera_inventory(input);
        let _ = parse_pinctrl_functions(input);
        let _ = parse_thermal_millidegrees(input);
    }
}

#[test]
fn config_parser_handles_stale_conditional_and_truncated_sections() {
    let root = temp_fixture_root();
    write_fixture_file(
        &root,
        "boot/firmware/config.txt",
        "[pi4]\ndtoverlay=vc4-kms-v3d\n[pi5]\ndtoverlay=vc4-kms-v3d\ndtparam=i2c_arm=on\ndtparam=i2c_arm=off\n[truncated\n",
    );

    let ctx = ProbeContext::with_root(&root).with_command_output(
        "vcgencmd",
        &["get_throttled"],
        CommandOutput::Missing,
    );
    let analysis = ConfigTxtProbe
        .collect(&ctx)
        .expect("malformed config should still produce a partial analysis");

    assert!(
        analysis
            .findings
            .iter()
            .any(|finding| finding.id == "config_txt.conflicting_dtparam")
    );
    assert!(
        analysis
            .findings
            .iter()
            .any(|finding| finding.id == "config_txt.malformed_line")
    );

    let _ = fs::remove_dir_all(root);
}

fn temp_fixture_root() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after Unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("pi-doctor-parser-properties-{nanos}"))
}

fn write_fixture_file(root: &Path, relative: &str, contents: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().expect("fixture path should have parent"))
        .expect("fixture parent should be created");
    fs::write(path, contents).expect("fixture file should be written");
}
