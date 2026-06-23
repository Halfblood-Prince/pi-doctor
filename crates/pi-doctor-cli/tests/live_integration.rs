use pi_doctor_core::ProbeContext;
use pi_doctor_probes::{
    board::BoardProbe, camera::CameraProbe, config_txt::ConfigTxtProbe, gpio::GpioProbe,
    kernel::KernelProbe, os::OsProbe, python::PythonProbe, thermal::ThermalProbe,
    throttling::ThrottlingProbe,
};

#[test]
#[ignore = "requires a live host environment; run manually on target hardware"]
fn live_host_check_report_builds_without_panicking() {
    let report = pi_doctor::build_check_report(&live_ctx());

    assert_eq!(report.metadata.command, "check");
    assert_eq!(report.schema_version, "1.0.0");
    assert!(report.system.is_some());
    assert!(report.config.is_some());
    assert!(report.camera.is_some());
    assert!(report.python.is_some());
}

#[test]
#[ignore = "requires a live host environment; run manually on target hardware"]
fn live_host_gpio_doctor_renders_and_mentions_stack() {
    let output = pi_doctor::doctor::gpio::render(&live_ctx());

    assert!(output.contains("pi-doctor doctor gpio"));
    assert!(output.contains("GPIO stack recommendation"));
    assert!(output.contains("gpiochip devices:"));
    assert!(output.contains("Recommended path"));
}

#[test]
#[ignore = "requires a live host environment; run manually on target hardware"]
fn live_host_camera_doctor_renders_inventory_section() {
    let output = pi_doctor::doctor::camera::render(&live_ctx());

    assert!(output.contains("pi-doctor doctor camera"));
    assert!(output.contains("Verdict:"));
    assert!(output.contains("available tools:"));
    assert!(output.contains("Inventory"));
}

#[test]
#[ignore = "requires a live host environment; run manually on target hardware"]
fn live_host_python_explain_renders_summary_and_guidance() {
    let output = pi_doctor::explain::python::render(&live_ctx());

    assert!(output.contains("pi-doctor explain python"));
    assert!(output.contains("Python environment analysis"));
    assert!(output.contains("virtual environment:"));
    assert!(output.contains("Exact next commands"));
}

#[test]
#[ignore = "requires a live host environment; run manually on target hardware"]
fn live_host_throttling_explain_renders_assessment() {
    let output = pi_doctor::explain::throttling::render(&live_ctx());

    assert!(output.contains("pi-doctor explain throttling"));
    assert!(output.contains("Firmware and thermal analysis"));
    assert!(output.contains("Assessment"));
    assert!(output.contains("CPU temperature:"));
}

#[test]
#[ignore = "requires a live host environment; run manually on target hardware"]
fn live_host_probe_collectors_are_resilient() {
    let ctx = live_ctx();

    let _ = BoardProbe.collect(&ctx);
    let _ = OsProbe.collect(&ctx);
    let _ = KernelProbe.collect(&ctx);
    let _ = ConfigTxtProbe.collect(&ctx);
    let _ = CameraProbe.collect(&ctx);
    let _ = GpioProbe.collect(&ctx);
    let _ = PythonProbe.collect(&ctx);
    let _ = ThermalProbe.collect(&ctx);
    let _ = ThrottlingProbe.collect(&ctx);
}

fn live_ctx() -> ProbeContext {
    ProbeContext::new()
}
