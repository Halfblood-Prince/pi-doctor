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
    CameraSummary, Finding, FindingDomain, FindingGroup, Impact, OverallStatus, ProbeContext,
    ProbeHealth, ProbeOutcome, Report, ReportMetadata, Severity, SystemSummary,
};
use pi_doctor_probes::{
    ProbeError,
    board::{BoardProbe, board_findings},
    camera::CameraProbe,
    config_txt::ConfigTxtProbe,
    kernel::KernelProbe,
    os::OsProbe,
    python::PythonProbe,
    thermal::{ThermalProbe, thermal_findings},
    throttling::{ThrottlingProbe, throttling_findings},
};
use std::io::IsTerminal;
use std::time::Duration;

pub struct CliResponse {
    pub output: String,
    pub exit_code: u8,
}

pub fn run(cli: Cli) -> Result<CliResponse> {
    let settings = output::RenderSettings::from_cli(&cli, std::io::stdout().is_terminal());
    let timeout = Duration::from_secs(cli.timeout);

    match cli.command {
        Commands::Check {} => execute_check(settings, timeout),
        Commands::Explain { topic } => execute_explain(topic, timeout),
        Commands::SupportBundle => execute_support_bundle(timeout),
        Commands::Doctor { target } => execute_doctor(target, timeout),
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
    let thermal_probe = ThermalProbe;
    let throttling_probe = ThrottlingProbe;
    let camera_probe = CameraProbe;
    let python_probe = PythonProbe;

    let board = collect_probe("board", board_probe.collect(ctx));
    let os = collect_probe("os", os_probe.collect(ctx));
    let kernel = collect_probe("kernel", kernel_probe.collect(ctx));
    let config = collect_probe("config", config_probe.collect(ctx));
    let thermal = collect_probe("thermal", thermal_probe.collect(ctx));
    let mut throttling = collect_probe("throttling", throttling_probe.collect(ctx));
    let mut camera = collect_probe("camera", camera_probe.collect(ctx));
    let python = collect_probe("python", python_probe.collect(ctx));

    if camera.health.outcome == ProbeOutcome::Success && camera.value.summary.tool_used.is_none() {
        camera.health.outcome = ProbeOutcome::Unavailable;
        camera.health.detail = Some("no camera inventory tool was available".to_owned());
    }
    if throttling.health.outcome == ProbeOutcome::Success && !throttling.value.vcgencmd_available {
        throttling.health.outcome = ProbeOutcome::Unavailable;
        throttling.health.detail = Some("vcgencmd get_throttled was unavailable".to_owned());
    }

    let mut probe_health = vec![
        board.health.clone(),
        os.health.clone(),
        kernel.health.clone(),
        config.health.clone(),
        thermal.health.clone(),
        throttling.health.clone(),
        camera.health.clone(),
        python.health.clone(),
    ];
    probe_health.sort_by_key(|health| health.name);

    let mut findings = Vec::new();
    findings.extend(board_findings(board.value.clone()));
    findings.extend(config.value.findings.clone());
    findings.extend(thermal_findings(&thermal.value));
    findings.extend(throttling_findings(throttling.value.clone()));
    findings.extend(camera.value.findings.clone());
    findings.extend(python.value.findings.clone());
    findings.extend([
        board.unavailable_finding,
        os.unavailable_finding,
        kernel.unavailable_finding,
        config.unavailable_finding,
        thermal.unavailable_finding,
        throttling.unavailable_finding,
        camera.unavailable_finding,
        python.unavailable_finding,
    ]
    .into_iter()
    .flatten());
    sort_findings(&mut findings);
    let groups = group_findings(&findings);

    Report {
        metadata: ReportMetadata {
            command: "check".to_owned(),
        },
        schema_version: "1.0.0",
        overall_status: overall_status(&findings),
        probe_health,
        system: Some(SystemSummary {
            board_model: board.value.model,
            board_revision: board.value.revision,
            architecture: kernel.value.architecture,
            distro_name: os.value.distro_name,
            distro_version: os.value.distro_version,
            distro_codename: os.value.distro_codename,
            kernel_release: kernel.value.release,
            is_raspberry_pi: board.value.is_raspberry_pi,
        }),
        config: Some(config.value.summary),
        camera: Some(camera.value.summary),
        python: Some(python.value.summary),
        groups,
        findings,
    }
}

#[derive(Debug, Clone)]
struct CheckedProbe<T> {
    value: T,
    health: ProbeHealth,
    unavailable_finding: Option<Finding>,
}

fn collect_probe<T>(name: &'static str, result: Result<T, ProbeError>) -> CheckedProbe<T>
where
    T: Default,
{
    match result {
        Ok(value) => CheckedProbe {
            value,
            health: ProbeHealth {
                name,
                outcome: ProbeOutcome::Success,
                detail: None,
            },
            unavailable_finding: None,
        },
        Err(error) => {
            warn!("{name} collection fallback: {error}");
            CheckedProbe {
                value: T::default(),
                health: ProbeHealth {
                    name,
                    outcome: probe_outcome_for_error(&error),
                    detail: Some(error.to_string()),
                },
                unavailable_finding: Some(probe_unavailable_finding(name, &error)),
            }
        }
    }
}

fn probe_outcome_for_error(error: &ProbeError) -> ProbeOutcome {
    match error {
        ProbeError::MissingField { .. }
        | ProbeError::ReadText { .. }
        | ProbeError::MissingTool { .. } => ProbeOutcome::Unavailable,
        ProbeError::PermissionDenied { .. } => ProbeOutcome::PermissionDenied,
        ProbeError::CommandFailure { .. } | ProbeError::CommandOutputLimit { .. } => {
            ProbeOutcome::CommandFailed
        }
        ProbeError::CommandTimedOut { .. } => ProbeOutcome::TimedOut,
        ProbeError::Parse { .. } => ProbeOutcome::ParseFailed,
    }
}

fn probe_unavailable_finding(name: &'static str, error: &ProbeError) -> Finding {
    let (id, title, impact) = match name {
        "board" => (
            "board.unavailable",
            "Board identity could not be inspected",
            Impact::Warning,
        ),
        "os" => (
            "os.unavailable",
            "Operating system identity could not be inspected",
            Impact::Warning,
        ),
        "kernel" => (
            "kernel.unavailable",
            "Kernel identity could not be inspected",
            Impact::Warning,
        ),
        "config" => (
            "config_txt.unavailable",
            "Boot config could not be inspected",
            Impact::Warning,
        ),
        "thermal" => (
            "thermal.unavailable",
            "Thermal state could not be inspected",
            Impact::Warning,
        ),
        "throttling" => (
            "throttling.unavailable",
            "Firmware throttling telemetry could not be inspected",
            Impact::Warning,
        ),
        "camera" => (
            "camera.unavailable",
            "Camera stack could not be inspected",
            Impact::Degraded,
        ),
        "python" => (
            "python.unavailable",
            "Python environment could not be inspected",
            Impact::Warning,
        ),
        _ => (
            "system.probe_unavailable",
            "A probe could not complete",
            Impact::Warning,
        ),
    };

    Finding {
        id,
        severity: Severity::Warning,
        impact,
        title: title.to_owned(),
        summary: format!(
            "The `{name}` probe did not complete, so this part of the report is incomplete."
        ),
        evidence: vec![error.to_string()],
        suggested_actions: vec![
            "Why this matters: unavailable probe data is not the same as a healthy subsystem.".to_owned(),
            format!(
                "What to run next: resolve the `{name}` probe error and rerun `pi-doctor check`."
            ),
        ],
    }
}

pub fn render_help() -> String {
    let mut command = Cli::command();
    command.render_long_help().to_string()
}

fn execute_check(settings: output::RenderSettings, timeout: Duration) -> Result<CliResponse> {
    render_check_with_context(&ProbeContext::new().with_timeout(timeout), settings)
}

fn execute_explain(topic: ExplainTopic, timeout: Duration) -> Result<CliResponse> {
    let ctx = ProbeContext::new().with_timeout(timeout);
    Ok(CliResponse {
        output: explain::render(topic, &ctx),
        exit_code: 0,
    })
}

fn execute_support_bundle(timeout: Duration) -> Result<CliResponse> {
    use std::collections::BTreeMap;
    use std::time::{SystemTime, UNIX_EPOCH};

    let ctx = ProbeContext::new().with_timeout(timeout);
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

fn execute_doctor(target: DoctorTarget, timeout: Duration) -> Result<CliResponse> {
    let ctx = ProbeContext::new().with_timeout(timeout);
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
    let highest = findings
        .iter()
        .map(|finding| finding.impact)
        .max()
        .unwrap_or(Impact::Info);

    match highest {
        Impact::Info => OverallStatus::Healthy,
        Impact::Warning => OverallStatus::Warning,
        Impact::Degraded => OverallStatus::Degraded,
        Impact::Critical => OverallStatus::Critical,
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
        pi_doctor_core::CommandOutput::TimedOut => "timed out\n".to_owned(),
        pi_doctor_core::CommandOutput::OutputLimitExceeded => "output limit exceeded\n".to_owned(),
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
    use super::{build_check_report, exit_code_for_status, overall_status};
    use pi_doctor_core::{
        CommandOutput, Finding, Impact, OverallStatus, ProbeContext, ProbeOutcome, Severity,
    };

    #[test]
    fn exit_code_contract_matches_status_levels() {
        assert_eq!(exit_code_for_status(OverallStatus::Healthy), 0);
        assert_eq!(exit_code_for_status(OverallStatus::Warning), 1);
        assert_eq!(exit_code_for_status(OverallStatus::Degraded), 2);
        assert_eq!(exit_code_for_status(OverallStatus::Critical), 3);
    }

    #[test]
    fn overall_status_uses_explicit_impact_not_finding_id() {
        let findings = vec![Finding {
            id: "example.active_but_only_warning",
            severity: Severity::Warning,
            impact: Impact::Warning,
            title: "Example warning".to_owned(),
            summary: "The id contains active, but the impact controls rollup.".to_owned(),
            evidence: Vec::new(),
            suggested_actions: Vec::new(),
        }];

        assert_eq!(overall_status(&findings), OverallStatus::Warning);
    }

    #[test]
    fn critical_impact_rolls_up_to_critical_status() {
        let findings = vec![Finding {
            id: "example.critical",
            severity: Severity::Warning,
            impact: Impact::Critical,
            title: "Example critical".to_owned(),
            summary: "Critical impact must reach the public status contract.".to_owned(),
            evidence: Vec::new(),
            suggested_actions: Vec::new(),
        }];

        assert_eq!(overall_status(&findings), OverallStatus::Critical);
    }

    #[test]
    fn probe_health_preserves_timed_out_camera_inventory() {
        let ctx = ProbeContext::new()
            .with_command_output(
                "rpicam-hello",
                &["--help"],
                CommandOutput::Success("usage".to_owned()),
            )
            .with_command_output(
                "rpicam-hello",
                &["--list-cameras"],
                CommandOutput::TimedOut,
            )
            .with_command_output("libcamera-hello", &["--help"], CommandOutput::Missing)
            .with_command_output("python3", &["--version"], CommandOutput::Missing);

        let report = build_check_report(&ctx);
        let camera_health = report
            .probe_health
            .iter()
            .find(|health| health.name == "camera")
            .expect("camera health should be present");

        assert_eq!(camera_health.outcome, ProbeOutcome::TimedOut);
    }
}
