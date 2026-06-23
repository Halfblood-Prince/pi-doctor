pub mod cli;
pub mod doctor;
pub mod explain;
pub mod output;

use anyhow::{Context, Result};
use clap::CommandFactory;
use clap_complete::generate;
use cli::args::{Cli, Commands, DoctorTarget, ExplainTopic};
use log::warn;
use pi_doctor_bundle::{BundleInput, write_bundle};
use pi_doctor_core::{
    CameraSummary, Finding, FindingDomain, FindingGroup, OverallStatus, Probe, ProbeContext,
    Report, ReportMetadata, SystemSummary,
};
use pi_doctor_probes::{
    board::BoardProbe, camera::CameraProbe, config_txt::ConfigTxtProbe, kernel::KernelProbe,
    os::OsProbe, python::PythonProbe, thermal::ThermalProbe, throttling::ThrottlingProbe,
};
use std::io::IsTerminal;

pub struct CliResponse {
    pub output: String,
    pub exit_code: u8,
}

pub fn run(cli: Cli) -> Result<CliResponse> {
    let settings = output::RenderSettings::from_cli(&cli, std::io::stdout().is_terminal());

    match cli.command {
        Commands::Check {} => execute_check(settings),
        Commands::Explain { topic } => execute_explain(topic),
        Commands::SupportBundle => execute_support_bundle(),
        Commands::Doctor { target } => execute_doctor(target),
        Commands::Completions { shell } => execute_completions(shell),
    }
}

pub fn render_check_json() -> Result<String> {
    let response =
        render_check_with_context(&ProbeContext::new(), output::RenderSettings::test_json())?;
    Ok(response.output)
}

pub fn build_check_report(ctx: &ProbeContext) -> Report {
    let board_probe = BoardProbe;
    let os_probe = OsProbe;
    let kernel_probe = KernelProbe;
    let config_probe = ConfigTxtProbe;

    let board = board_probe.collect(ctx).unwrap_or_else(|error| {
        warn!("board collection fallback: {error}");
        Default::default()
    });
    let os = os_probe.collect(ctx).unwrap_or_else(|error| {
        warn!("os collection fallback: {error}");
        Default::default()
    });
    let kernel = kernel_probe.collect(ctx).unwrap_or_else(|error| {
        warn!("kernel collection fallback: {error}");
        pi_doctor_probes::kernel::KernelDetails {
            architecture: Some(std::env::consts::ARCH.to_owned()),
            release: None,
        }
    });
    let config = config_probe.collect(ctx).unwrap_or_else(|error| {
        warn!("config collection fallback: {error}");
        Default::default()
    });
    let camera = CameraProbe.collect(ctx).unwrap_or_else(|error| {
        warn!("camera collection fallback: {error}");
        Default::default()
    });
    let python = PythonProbe.collect(ctx).unwrap_or_else(|error| {
        warn!("python collection fallback: {error}");
        Default::default()
    });
    let mut findings = vec![
        board_probe.run(ctx),
        os_probe.run(ctx),
        kernel_probe.run(ctx),
        config.findings.clone(),
        ThermalProbe.run(ctx),
        ThrottlingProbe.run(ctx),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();
    sort_findings(&mut findings);
    let groups = group_findings(&findings);

    Report {
        metadata: ReportMetadata {
            command: "check".to_owned(),
        },
        schema_version: "1.0.0",
        overall_status: overall_status(&findings),
        system: Some(SystemSummary {
            board_model: board.model,
            board_revision: board.revision,
            architecture: kernel.architecture,
            distro_name: os.distro_name,
            distro_version: os.distro_version,
            distro_codename: os.distro_codename,
            kernel_release: kernel.release,
            is_raspberry_pi: board.is_raspberry_pi,
        }),
        config: Some(config.summary),
        camera: Some(camera.summary),
        python: Some(python.summary),
        groups,
        findings,
    }
}

pub fn render_help() -> String {
    let mut command = Cli::command();
    command.render_long_help().to_string()
}

fn execute_check(settings: output::RenderSettings) -> Result<CliResponse> {
    render_check_with_context(&ProbeContext::new(), settings)
}

fn execute_explain(topic: ExplainTopic) -> Result<CliResponse> {
    let ctx = ProbeContext::new();
    Ok(CliResponse {
        output: explain::render(topic, &ctx),
        exit_code: 0,
    })
}

fn execute_support_bundle() -> Result<CliResponse> {
    use std::collections::BTreeMap;
    use std::time::{SystemTime, UNIX_EPOCH};

    let ctx = ProbeContext::new();
    let report = build_check_report(&ctx);
    let mut extra_files = BTreeMap::new();

    extra_files.insert(
        "summary/system.txt".to_owned(),
        render_system_summary(&report),
    );
    extra_files.insert(
        "summary/config.txt".to_owned(),
        explain::config::render(&ctx),
    );
    extra_files.insert(
        "summary/camera.txt".to_owned(),
        doctor::camera::render(&ctx),
    );
    extra_files.insert("summary/gpio.txt".to_owned(), doctor::gpio::render(&ctx));
    extra_files.insert(
        "summary/python.txt".to_owned(),
        explain::python::render(&ctx),
    );

    extra_files.insert(
        "raw/firmware/version.txt".to_owned(),
        command_output_text(&ctx, "vcgencmd", &["version"]),
    );
    extra_files.insert(
        "raw/firmware/get_throttled.txt".to_owned(),
        command_output_text(&ctx, "vcgencmd", &["get_throttled"]),
    );
    extra_files.insert(
        "raw/thermal/temp.txt".to_owned(),
        ctx.read_text("/sys/class/thermal/thermal_zone0/temp")
            .unwrap_or_else(|| "unavailable\n".to_owned()),
    );
    extra_files.insert(
        "raw/config/source-path.txt".to_owned(),
        report
            .config
            .as_ref()
            .and_then(|config| config.source_path.clone())
            .unwrap_or_else(|| "unavailable".to_owned()),
    );
    extra_files.insert(
        "raw/camera/inventory.txt".to_owned(),
        report
            .camera
            .as_ref()
            .map(render_camera_summary)
            .unwrap_or_else(|| "unavailable\n".to_owned()),
    );
    extra_files.insert(
        "raw/python/summary.txt".to_owned(),
        render_python_summary(&report),
    );

    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("failed to compute support bundle timestamp")?
        .as_secs();
    let bundle_name = format!("pi-doctor-bundle-{seconds}");
    let result = write_bundle(
        ".",
        &bundle_name,
        &BundleInput {
            report,
            extra_files,
        },
    )
    .context("failed to write support bundle")?;

    Ok(CliResponse {
        output: format!(
            "Support bundle written to {}\nFiles:\n{}\n",
            result.bundle_dir.display(),
            result.files.join("\n")
        ),
        exit_code: 0,
    })
}

fn execute_doctor(target: DoctorTarget) -> Result<CliResponse> {
    let ctx = ProbeContext::new();
    let output = match target {
        DoctorTarget::Camera => doctor::camera::render(&ctx),
        DoctorTarget::Gpio => doctor::gpio::render(&ctx),
    };

    Ok(CliResponse {
        output,
        exit_code: 0,
    })
}

fn execute_completions(shell: clap_complete::Shell) -> Result<CliResponse> {
    let mut command = Cli::command();
    let mut buffer = Vec::new();
    generate(shell, &mut command, "pi-doctor", &mut buffer);
    Ok(CliResponse {
        output: String::from_utf8(buffer).context("failed to encode shell completions as UTF-8")?,
        exit_code: 0,
    })
}

fn render_check_with_context(
    ctx: &ProbeContext,
    settings: output::RenderSettings,
) -> Result<CliResponse> {
    let report = build_check_report(ctx);
    Ok(CliResponse {
        output: output::render_report(&report, settings).context("failed to render report")?,
        exit_code: exit_code_for_status(report.overall_status),
    })
}

fn overall_status(findings: &[Finding]) -> OverallStatus {
    if findings.is_empty() {
        OverallStatus::Healthy
    } else if findings.iter().any(|finding| finding.id.contains("active")) {
        OverallStatus::Degraded
    } else if findings
        .iter()
        .any(|finding| matches!(finding.severity, pi_doctor_core::Severity::Warning))
    {
        OverallStatus::Warning
    } else {
        OverallStatus::Healthy
    }
}

fn group_findings(findings: &[Finding]) -> Vec<FindingGroup> {
    let mut groups = Vec::new();
    for domain in [
        FindingDomain::System,
        FindingDomain::Power,
        FindingDomain::Thermal,
        FindingDomain::Config,
        FindingDomain::Gpio,
        FindingDomain::Camera,
        FindingDomain::Python,
    ] {
        let domain_findings = findings
            .iter()
            .filter(|finding| finding_domain(finding) == domain)
            .cloned()
            .collect::<Vec<_>>();
        if !domain_findings.is_empty() {
            groups.push(FindingGroup {
                domain,
                findings: domain_findings,
            });
        }
    }
    groups
}

fn sort_findings(findings: &mut [Finding]) {
    findings.sort_by(|left, right| {
        finding_domain(left)
            .cmp(&finding_domain(right))
            .then_with(|| right.severity.cmp(&left.severity))
            .then_with(|| left.id.cmp(right.id))
    });
}

fn finding_domain(finding: &Finding) -> FindingDomain {
    if finding.id.starts_with("board.")
        || finding.id.starts_with("os.")
        || finding.id.starts_with("kernel.")
    {
        FindingDomain::System
    } else if finding.id.starts_with("throttling.") {
        FindingDomain::Power
    } else if finding.id.starts_with("thermal.") {
        FindingDomain::Thermal
    } else if finding.id.starts_with("config_txt.") {
        FindingDomain::Config
    } else if finding.id.starts_with("gpio.") {
        FindingDomain::Gpio
    } else if finding.id.starts_with("camera.") {
        FindingDomain::Camera
    } else if finding.id.starts_with("python.") {
        FindingDomain::Python
    } else {
        FindingDomain::System
    }
}

fn exit_code_for_status(status: OverallStatus) -> u8 {
    match status {
        OverallStatus::Healthy => 0,
        OverallStatus::Warning => 1,
        OverallStatus::Degraded => 2,
        OverallStatus::Critical => 3,
    }
}

fn command_output_text(ctx: &ProbeContext, program: &str, args: &[&str]) -> String {
    match ctx.run_command(program, args) {
        pi_doctor_core::CommandOutput::Success(output) => format!("{output}\n"),
        pi_doctor_core::CommandOutput::Missing => "missing\n".to_owned(),
        pi_doctor_core::CommandOutput::Failure(error) => format!("failure: {error}\n"),
    }
}

fn render_system_summary(report: &Report) -> String {
    if let Some(system) = &report.system {
        format!(
            "board_model={}\nboard_revision={}\narchitecture={}\ndistro={}\nversion={}\nkernel={}\nis_raspberry_pi={}\n",
            system.board_model.as_deref().unwrap_or("unknown"),
            system.board_revision.as_deref().unwrap_or("unknown"),
            system.architecture.as_deref().unwrap_or("unknown"),
            system.distro_name.as_deref().unwrap_or("unknown"),
            system.distro_version.as_deref().unwrap_or("unknown"),
            system.kernel_release.as_deref().unwrap_or("unknown"),
            system.is_raspberry_pi
        )
    } else {
        "unavailable\n".to_owned()
    }
}

fn render_camera_summary(camera: &CameraSummary) -> String {
    if camera.cameras.is_empty() {
        return "no cameras detected\n".to_owned();
    }
    let mut lines = Vec::new();
    for device in &camera.cameras {
        lines.push(format!("[{}] {}", device.index, device.name));
        if let Some(mode_hint) = &device.mode_hint {
            lines.push(mode_hint.clone());
        }
    }
    format!("{}\n", lines.join("\n"))
}

fn render_python_summary(report: &Report) -> String {
    let Some(python) = &report.python else {
        return "unavailable\n".to_owned();
    };
    format!(
        "version={}\nexecutable={}\nin_virtualenv={}\nexternally_managed={}\ndetected_packages={}\n",
        python.version.as_deref().unwrap_or("unknown"),
        python.executable.as_deref().unwrap_or("unknown"),
        python.in_virtualenv,
        python.externally_managed,
        if python.detected_packages.is_empty() {
            "none".to_owned()
        } else {
            python.detected_packages.join(",")
        }
    )
}

#[cfg(test)]
mod tests {
    use super::exit_code_for_status;
    use pi_doctor_core::OverallStatus;

    #[test]
    fn exit_code_contract_matches_status_levels() {
        assert_eq!(exit_code_for_status(OverallStatus::Healthy), 0);
        assert_eq!(exit_code_for_status(OverallStatus::Warning), 1);
        assert_eq!(exit_code_for_status(OverallStatus::Degraded), 2);
        assert_eq!(exit_code_for_status(OverallStatus::Critical), 3);
    }
}
