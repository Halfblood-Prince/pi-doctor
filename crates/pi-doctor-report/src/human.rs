use pi_doctor_core::{Finding, FindingDomain, OverallStatus, Report, Severity};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verbosity {
    Compact,
    Normal,
    Verbose,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderOptions {
    pub verbosity: Verbosity,
    pub color: bool,
}

pub fn render(report: &Report, options: RenderOptions) -> String {
    let mut lines = vec![
        format_title(
            &format!("pi-doctor {}", report.metadata.command),
            options.color,
        ),
        format!(
            "Overall status: {}",
            colorize_status(
                overall_status_label(report.overall_status),
                report.overall_status,
                options.color
            )
        ),
        String::new(),
    ];

    if let Some(system) = &report.system {
        lines.push("System summary".to_owned());
        lines.push(format!(
            "  Board model: {}",
            display(system.board_model.as_deref())
        ));
        lines.push(format!(
            "  Board revision: {}",
            display(system.board_revision.as_deref())
        ));
        lines.push(format!(
            "  Architecture: {}",
            display(system.architecture.as_deref())
        ));
        lines.push(format!(
            "  Distro: {}",
            display(system.distro_name.as_deref())
        ));
        lines.push(format!(
            "  Version: {}",
            display_version(
                system.distro_version.as_deref(),
                system.distro_codename.as_deref()
            )
        ));
        lines.push(format!(
            "  Kernel: {}",
            display(system.kernel_release.as_deref())
        ));
        lines.push(format!(
            "  Raspberry Pi: {}",
            if system.is_raspberry_pi { "yes" } else { "no" }
        ));
        lines.push(String::new());
    }

    for group in &report.groups {
        lines.push(format!("{} findings", domain_label(group.domain)));
        for finding in &group.findings {
            render_finding(&mut lines, finding, options);
        }
    }

    lines.join("\n").trim_end().to_owned()
}

fn render_finding(lines: &mut Vec<String>, finding: &Finding, options: RenderOptions) {
    lines.push(format!(
        "[{}] {}",
        colorize_severity(
            severity_label(finding.severity),
            finding.severity,
            options.color
        ),
        finding.title
    ));
    lines.push(format!("  {}", finding.summary));

    if matches!(options.verbosity, Verbosity::Verbose) {
        for evidence in &finding.evidence {
            lines.push(format!("  evidence: {evidence}"));
        }
        for action in &finding.suggested_actions {
            lines.push(format!("  next: {action}"));
        }
    } else if matches!(options.verbosity, Verbosity::Normal) {
        for action in &finding.suggested_actions {
            lines.push(format!("  next: {action}"));
        }
    }

    lines.push(String::new());
}

fn severity_label(severity: Severity) -> &'static str {
    match severity {
        Severity::Info => "info",
        Severity::Warning => "warning",
        Severity::Error => "error",
    }
}

fn overall_status_label(status: OverallStatus) -> &'static str {
    match status {
        OverallStatus::Healthy => "healthy",
        OverallStatus::Warning => "warning",
        OverallStatus::Degraded => "degraded",
        OverallStatus::Critical => "critical",
    }
}

fn domain_label(domain: FindingDomain) -> &'static str {
    match domain {
        FindingDomain::System => "System",
        FindingDomain::Power => "Power",
        FindingDomain::Thermal => "Thermal",
        FindingDomain::Config => "Config",
        FindingDomain::Gpio => "GPIO",
        FindingDomain::Camera => "Camera",
        FindingDomain::Python => "Python",
    }
}

fn colorize_severity(label: &str, severity: Severity, color: bool) -> String {
    if !color {
        return label.to_owned();
    }

    match severity {
        Severity::Info => format!("\x1b[36m{label}\x1b[0m"),
        Severity::Warning => format!("\x1b[33m{label}\x1b[0m"),
        Severity::Error => format!("\x1b[31m{label}\x1b[0m"),
    }
}

fn colorize_status(label: &str, status: OverallStatus, color: bool) -> String {
    if !color {
        return label.to_owned();
    }

    match status {
        OverallStatus::Healthy => format!("\x1b[32m{label}\x1b[0m"),
        OverallStatus::Warning => format!("\x1b[33m{label}\x1b[0m"),
        OverallStatus::Degraded | OverallStatus::Critical => format!("\x1b[31m{label}\x1b[0m"),
    }
}

fn format_title(title: &str, color: bool) -> String {
    if color {
        format!("\x1b[1m{title}\x1b[0m")
    } else {
        title.to_owned()
    }
}

fn display(value: Option<&str>) -> &str {
    value.unwrap_or("unknown")
}

fn display_version(version: Option<&str>, codename: Option<&str>) -> String {
    match (version, codename) {
        (Some(version), Some(codename)) => format!("{version} ({codename})"),
        (Some(version), None) => version.to_owned(),
        (None, Some(codename)) => codename.to_owned(),
        (None, None) => "unknown".to_owned(),
    }
}
