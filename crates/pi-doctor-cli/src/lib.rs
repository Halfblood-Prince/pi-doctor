pub mod cli;
pub mod doctor;
pub mod explain;
pub mod output;

use anyhow::{Context, Result};
use clap::CommandFactory;
use clap_complete::generate;
use cli::args::{Cli, Commands, DoctorTarget, ExplainTopic};
use log::warn;
use pi_doctor_bundle::{BundleInput, BundlePrivacyMode, write_bundle};
use pi_doctor_core::{
    CameraSummary, Finding, FindingDomain, FindingGroup, Impact, OverallStatus, ProbeContext,
    ProbeAvailabilitySummary, ProbeHealth, ProbeOutcome, Report, ReportMetadata, Severity,
    SupportedOs, SystemSummary,
};
use pi_doctor_probes::{
    ProbeError,
    board::{BoardProbe, board_findings},
    camera::CameraProbe,
    config_txt::ConfigTxtProbe,
    gpio::{GpioAnalysis, GpioProbe, gpio_findings},
    kernel::KernelProbe,
    os::OsProbe,
    python::PythonProbe,
    thermal::{ThermalProbe, thermal_findings},
    throttling::{ThrottlingProbe, throttling_findings},
};
use serde::Serialize;
use std::io::IsTerminal;
use std::path::PathBuf;
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
        Commands::SupportBundle {
            output,
            dry_run,
            include_sensitive,
            acknowledge_sensitive_data: _,
        } => execute_support_bundle(
            settings,
            timeout,
            SupportBundleOptions {
                output,
                dry_run,
                privacy_mode: if include_sensitive {
                    BundlePrivacyMode::Sensitive
                } else {
                    BundlePrivacyMode::Sanitized
                },
            },
        ),
        Commands::Doctor { target } => execute_doctor(target, settings, timeout),
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

    let system = SystemSummary {
        board_model: board.value.model,
        board_revision: board.value.revision,
        architecture: kernel.value.architecture,
        distro_name: os.value.distro_name,
        distro_version: os.value.distro_version,
        distro_codename: os.value.distro_codename,
        kernel_release: kernel.value.release,
        is_raspberry_pi: board.value.is_raspberry_pi,
    };
    let metadata = ReportMetadata::new("check")
        .with_supported_os(supported_os_detection(&system))
        .with_probe_availability(probe_availability_summary(&probe_health));

    Report {
        metadata,
        schema_version: "1.0.0",
        overall_status: overall_status(&findings),
        probe_health,
        system: Some(system),
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

fn probe_availability_summary(probe_health: &[ProbeHealth]) -> ProbeAvailabilitySummary {
    let mut summary = ProbeAvailabilitySummary {
        total: probe_health.len(),
        ..ProbeAvailabilitySummary::default()
    };

    for health in probe_health {
        match health.outcome {
            ProbeOutcome::Success => summary.success += 1,
            ProbeOutcome::Unavailable => summary.unavailable += 1,
            ProbeOutcome::PermissionDenied => summary.permission_denied += 1,
            ProbeOutcome::CommandFailed => summary.command_failed += 1,
            ProbeOutcome::ParseFailed => summary.parse_failed += 1,
            ProbeOutcome::TimedOut => summary.timed_out += 1,
        }
    }

    summary
}

fn supported_os_detection(system: &SystemSummary) -> SupportedOs {
    let family = system.distro_name.clone();
    let version = system.distro_version.clone();
    let codename = system.distro_codename.clone();
    let architecture = system.architecture.as_deref().unwrap_or_default();
    let supported_arch = architecture == "aarch64" || architecture.starts_with("arm");

    if !system.is_raspberry_pi {
        return SupportedOs {
            supported: false,
            family,
            version,
            codename,
            reason: Some("host is not detected as Raspberry Pi hardware".to_owned()),
        };
    }

    if !supported_arch {
        return SupportedOs {
            supported: false,
            family,
            version,
            codename,
            reason: Some(format!("architecture `{architecture}` is outside the supported matrix")),
        };
    }

    match codename.as_deref() {
        Some("bookworm") | Some("trixie") => SupportedOs {
            supported: true,
            family,
            version,
            codename,
            reason: None,
        },
        Some(value) => SupportedOs {
            supported: false,
            family,
            version,
            codename,
            reason: Some(format!("OS codename `{value}` is outside the supported matrix")),
        },
        None => SupportedOs {
            supported: false,
            family,
            version,
            codename,
            reason: Some("OS codename was unavailable".to_owned()),
        },
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
        "gpio" => (
            "gpio.unavailable",
            "GPIO state could not be inspected",
            Impact::Warning,
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

#[derive(Debug)]
struct SupportBundleOptions {
    output: PathBuf,
    dry_run: bool,
    privacy_mode: BundlePrivacyMode,
}

fn execute_support_bundle(
    settings: output::RenderSettings,
    timeout: Duration,
    options: SupportBundleOptions,
) -> Result<CliResponse> {
    use std::collections::BTreeMap;

    let plan = support_bundle_collection_plan();
    if options.dry_run {
        let response = SupportBundleJson {
            metadata: ReportMetadata::new("support-bundle"),
            schema_version: "1.0.0",
            dry_run: true,
            output_root: options.output.display().to_string(),
            bundle_dir: String::new(),
            privacy_mode: privacy_mode_label(options.privacy_mode),
            redaction_enabled: options.privacy_mode == BundlePrivacyMode::Sanitized,
            files: plan.iter().map(|item| item.path.to_owned()).collect(),
            collection_plan: plan.clone(),
            manifest: Vec::new(),
            report_schema_version: "1.0.0",
        };

        return if matches!(settings.mode, output::OutputMode::Json) {
            Ok(CliResponse {
                output: format!("{}\n", serde_json::to_string_pretty(&response)?),
                exit_code: 0,
            })
        } else {
            Ok(CliResponse {
                output: render_support_bundle_preview(&options, &plan),
                exit_code: 0,
            })
        };
    }

    let ctx = ProbeContext::new().with_timeout(timeout);
    let report = build_check_report(&ctx);
    let bundle_metadata = ReportMetadata::new("support-bundle")
        .with_supported_os(report.metadata.supported_os.clone())
        .with_probe_availability(report.metadata.probe_availability.clone());
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

    let bundle_name = support_bundle_name()?;
    let result = write_bundle(
        &options.output,
        &bundle_name,
        &BundleInput {
            report,
            extra_files,
            privacy_mode: options.privacy_mode,
        },
    )
    .context("failed to write support bundle")?;

    if matches!(settings.mode, output::OutputMode::Json) {
        let response = SupportBundleJson {
            metadata: bundle_metadata,
            schema_version: "1.0.0",
            dry_run: false,
            output_root: options.output.display().to_string(),
            bundle_dir: result.bundle_dir.display().to_string(),
            privacy_mode: privacy_mode_label(result.privacy_mode),
            redaction_enabled: result.privacy_mode == BundlePrivacyMode::Sanitized,
            files: result.files.clone(),
            collection_plan: plan,
            manifest: result
                .manifest
                .iter()
                .map(|entry| ManifestEntryJson {
                    path: entry.path.clone(),
                    sha256: entry.sha256.clone(),
                    bytes: entry.bytes,
                })
                .collect(),
            report_schema_version: "1.0.0",
        };
        Ok(CliResponse {
            output: format!("{}\n", serde_json::to_string_pretty(&response)?),
            exit_code: 0,
        })
    } else {
        Ok(CliResponse {
            output: render_support_bundle_result(&result, &options, &plan),
            exit_code: 0,
        })
    }
}

fn support_bundle_name() -> Result<String> {
    use std::time::{SystemTime, UNIX_EPOCH};

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("failed to compute support bundle timestamp")?
        .as_nanos();
    Ok(format!("pi-doctor-bundle-{nanos}-{}", std::process::id()))
}

#[derive(Debug, Clone, Serialize)]
struct SupportBundlePlanItem {
    path: &'static str,
    source: &'static str,
}

fn support_bundle_collection_plan() -> Vec<SupportBundlePlanItem> {
    vec![
        SupportBundlePlanItem {
            path: "report.json",
            source: "machine-readable check report",
        },
        SupportBundlePlanItem {
            path: "report.txt",
            source: "human-readable check report",
        },
        SupportBundlePlanItem {
            path: "privacy.txt",
            source: "bundle privacy mode and redaction notice",
        },
        SupportBundlePlanItem {
            path: "summary/system.txt",
            source: "board, OS, kernel, and architecture summary",
        },
        SupportBundlePlanItem {
            path: "summary/config.txt",
            source: "config.txt diagnostic explanation",
        },
        SupportBundlePlanItem {
            path: "summary/camera.txt",
            source: "camera diagnostic explanation",
        },
        SupportBundlePlanItem {
            path: "summary/gpio.txt",
            source: "GPIO diagnostic explanation",
        },
        SupportBundlePlanItem {
            path: "summary/python.txt",
            source: "Python environment explanation",
        },
        SupportBundlePlanItem {
            path: "raw/firmware/version.txt",
            source: "`vcgencmd version` output when available",
        },
        SupportBundlePlanItem {
            path: "raw/firmware/get_throttled.txt",
            source: "`vcgencmd get_throttled` output when available",
        },
        SupportBundlePlanItem {
            path: "raw/thermal/temp.txt",
            source: "/sys/class/thermal/thermal_zone0/temp",
        },
        SupportBundlePlanItem {
            path: "raw/config/source-path.txt",
            source: "detected active config.txt path",
        },
        SupportBundlePlanItem {
            path: "raw/camera/inventory.txt",
            source: "parsed camera inventory summary",
        },
        SupportBundlePlanItem {
            path: "raw/python/summary.txt",
            source: "Python executable, venv, and package summary",
        },
        SupportBundlePlanItem {
            path: "manifest.txt",
            source: "SHA-256 hashes for bundle payload files",
        },
    ]
}

fn render_support_bundle_preview(
    options: &SupportBundleOptions,
    plan: &[SupportBundlePlanItem],
) -> String {
    let mut lines = vec![
        "Support bundle dry run".to_owned(),
        format!("Output directory: {}", options.output.display()),
        format!(
            "Privacy mode: {}{}",
            privacy_mode_label(options.privacy_mode),
            if options.privacy_mode == BundlePrivacyMode::Sanitized {
                " (redaction enabled)"
            } else {
                " (redaction disabled)"
            }
        ),
        "Files to collect:".to_owned(),
    ];
    for item in plan {
        lines.push(format!("  {} - {}", item.path, item.source));
    }
    lines.push("No files were written.".to_owned());
    format!("{}\n", lines.join("\n"))
}

fn render_support_bundle_result(
    result: &pi_doctor_bundle::BundleResult,
    options: &SupportBundleOptions,
    plan: &[SupportBundlePlanItem],
) -> String {
    let mut lines = vec![
        format!("Support bundle written to {}", result.bundle_dir.display()),
        format!("Output directory: {}", options.output.display()),
        format!(
            "Privacy mode: {}{}",
            privacy_mode_label(result.privacy_mode),
            if result.privacy_mode == BundlePrivacyMode::Sanitized {
                " (redaction enabled)"
            } else {
                " (redaction disabled)"
            }
        ),
        "Collected files:".to_owned(),
    ];
    for item in plan {
        lines.push(format!("  {} - {}", item.path, item.source));
    }
    lines.push("Manifest hashes:".to_owned());
    for entry in &result.manifest {
        lines.push(format!("  {}  {}  {}", entry.sha256, entry.bytes, entry.path));
    }
    format!("{}\n", lines.join("\n"))
}

fn privacy_mode_label(mode: BundlePrivacyMode) -> &'static str {
    match mode {
        BundlePrivacyMode::Sanitized => "sanitized",
        BundlePrivacyMode::Sensitive => "sensitive",
    }
}

fn execute_doctor(
    target: DoctorTarget,
    settings: output::RenderSettings,
    timeout: Duration,
) -> Result<CliResponse> {
    let ctx = ProbeContext::new().with_timeout(timeout);
    if matches!(settings.mode, output::OutputMode::Json) {
        return render_doctor_json(target, &ctx);
    }

    let output = match target {
        DoctorTarget::Camera => doctor::camera::render(&ctx),
        DoctorTarget::Gpio => doctor::gpio::render(&ctx),
    };

    Ok(CliResponse {
        output,
        exit_code: 0,
    })
}

#[derive(Debug, Serialize)]
struct DoctorJson<T>
where
    T: Serialize,
{
    metadata: ReportMetadata,
    schema_version: &'static str,
    target: &'static str,
    summary: T,
    findings: Vec<Finding>,
}

#[derive(Debug, Serialize)]
struct SupportBundleJson {
    metadata: ReportMetadata,
    schema_version: &'static str,
    dry_run: bool,
    output_root: String,
    bundle_dir: String,
    privacy_mode: &'static str,
    redaction_enabled: bool,
    files: Vec<String>,
    collection_plan: Vec<SupportBundlePlanItem>,
    manifest: Vec<ManifestEntryJson>,
    report_schema_version: &'static str,
}

#[derive(Debug, Serialize)]
struct ManifestEntryJson {
    path: String,
    sha256: String,
    bytes: usize,
}

fn render_doctor_json(target: DoctorTarget, ctx: &ProbeContext) -> Result<CliResponse> {
    match target {
        DoctorTarget::Camera => {
            let (analysis, mut outcome, findings) = match CameraProbe.collect(ctx) {
                Ok(analysis) => {
                    let findings = analysis.findings.clone();
                    (analysis, ProbeOutcome::Success, findings)
                }
                Err(error) => (
                    Default::default(),
                    probe_outcome_for_error(&error),
                    vec![probe_unavailable_finding("camera", &error)],
                ),
            };
            if outcome == ProbeOutcome::Success && analysis.summary.tool_used.is_none() {
                outcome = ProbeOutcome::Unavailable;
            }
            render_focused_json(DoctorJson {
                metadata: metadata_for_single_probe("doctor camera", outcome),
                schema_version: "1.0.0",
                target: "camera",
                summary: analysis.summary,
                findings,
            })
        }
        DoctorTarget::Gpio => {
            let (analysis, outcome, findings) = match GpioProbe.collect(ctx) {
                Ok(analysis) => {
                    let findings = gpio_findings(&analysis);
                    (analysis, ProbeOutcome::Success, findings)
                }
                Err(error) => (
                    Default::default(),
                    probe_outcome_for_error(&error),
                    vec![probe_unavailable_finding("gpio", &error)],
                ),
            };
            render_focused_json(DoctorJson::<GpioAnalysis> {
                metadata: metadata_for_single_probe("doctor gpio", outcome),
                schema_version: "1.0.0",
                target: "gpio",
                summary: analysis,
                findings,
            })
        }
    }
}

fn metadata_for_single_probe(command: &'static str, outcome: ProbeOutcome) -> ReportMetadata {
    ReportMetadata::new(command).with_probe_availability(probe_availability_summary(&[
        ProbeHealth {
            name: command,
            outcome,
            detail: None,
        },
    ]))
}

fn render_focused_json<T>(response: T) -> Result<CliResponse>
where
    T: Serialize,
{
    Ok(CliResponse {
        output: format!("{}\n", serde_json::to_string_pretty(&response)?),
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
