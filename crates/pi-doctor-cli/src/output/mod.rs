use crate::error::CliError;
use crate::cli::args::Cli;
use pi_doctor_core::Report;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputMode {
    Human,
    Json,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorPolicy {
    Auto,
    Never,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verbosity {
    Compact,
    Normal,
    Verbose,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderSettings {
    pub mode: OutputMode,
    pub color_policy: ColorPolicy,
    pub verbosity: Verbosity,
    pub is_tty: bool,
}

impl RenderSettings {
    pub fn from_cli(cli: &Cli, is_tty: bool) -> Self {
        let mode = if cli.json {
            OutputMode::Json
        } else {
            OutputMode::Human
        };
        let verbosity = if cli.quiet {
            Verbosity::Compact
        } else if cli.verbose {
            Verbosity::Verbose
        } else {
            Verbosity::Normal
        };
        let color_policy = if cli.no_color {
            ColorPolicy::Never
        } else {
            ColorPolicy::Auto
        };

        Self {
            mode,
            color_policy,
            verbosity,
            is_tty,
        }
    }

    pub fn test_json() -> Self {
        Self {
            mode: OutputMode::Json,
            color_policy: ColorPolicy::Never,
            verbosity: Verbosity::Normal,
            is_tty: false,
        }
    }
}

pub fn render_report(
    report: &Report,
    settings: RenderSettings,
) -> Result<String, CliError> {
    match settings.mode {
        OutputMode::Human => {
            let options = pi_doctor_report::human::RenderOptions {
                verbosity: match settings.verbosity {
                    Verbosity::Compact => pi_doctor_report::human::Verbosity::Compact,
                    Verbosity::Normal => pi_doctor_report::human::Verbosity::Normal,
                    Verbosity::Verbose => pi_doctor_report::human::Verbosity::Verbose,
                },
                color: settings.color_policy == ColorPolicy::Auto && settings.is_tty,
            };
            Ok(format!(
                "{}\n",
                pi_doctor_report::human::render(report, options)
            ))
        }
        OutputMode::Json => Ok(format!("{}\n", pi_doctor_report::json::render(report)?)),
    }
}

#[cfg(test)]
mod tests {
    use super::{ColorPolicy, OutputMode, RenderSettings, Verbosity, render_report};
    use pi_doctor_core::{
        Finding, FindingDomain, FindingGroup, OverallStatus, Report, ReportMetadata, Severity,
    };

    #[test]
    fn tty_human_output_uses_color_by_default() {
        let report = empty_report();
        let output = render_report(
            &report,
            RenderSettings {
                mode: OutputMode::Human,
                color_policy: ColorPolicy::Auto,
                verbosity: Verbosity::Normal,
                is_tty: true,
            },
        )
        .expect("render should succeed");

        assert!(output.contains("\u{1b}["));
    }

    #[test]
    fn non_tty_human_output_does_not_use_color() {
        let report = empty_report();
        let output = render_report(
            &report,
            RenderSettings {
                mode: OutputMode::Human,
                color_policy: ColorPolicy::Auto,
                verbosity: Verbosity::Normal,
                is_tty: false,
            },
        )
        .expect("render should succeed");

        assert!(!output.contains("\u{1b}["));
    }

    #[test]
    fn quiet_mode_is_more_compact_than_verbose_mode() {
        let report = report_with_finding();
        let quiet = render_report(
            &report,
            RenderSettings {
                mode: OutputMode::Human,
                color_policy: ColorPolicy::Never,
                verbosity: Verbosity::Compact,
                is_tty: false,
            },
        )
        .expect("render should succeed");
        let verbose = render_report(
            &report,
            RenderSettings {
                mode: OutputMode::Human,
                color_policy: ColorPolicy::Never,
                verbosity: Verbosity::Verbose,
                is_tty: false,
            },
        )
        .expect("render should succeed");

        assert!(!quiet.contains("evidence:"));
        assert!(verbose.contains("evidence:"));
        assert!(verbose.contains("next:"));
    }

    fn empty_report() -> Report {
        Report {
            metadata: ReportMetadata {
                command: "check".to_owned(),
            },
            schema_version: "1.0.0",
            overall_status: OverallStatus::Healthy,
            system: None,
            config: None,
            camera: None,
            python: None,
            groups: Vec::new(),
            findings: Vec::new(),
        }
    }

    fn report_with_finding() -> Report {
        let finding = Finding {
            id: "thermal.near_throttle",
            severity: Severity::Warning,
            title: "CPU temperature is near throttling range".to_owned(),
            summary: "CPU temperature is 78.2 C.".to_owned(),
            evidence: vec!["temperature classification: near throttle".to_owned()],
            suggested_actions: vec!["improve cooling".to_owned()],
        };

        Report {
            metadata: ReportMetadata {
                command: "check".to_owned(),
            },
            schema_version: "1.0.0",
            overall_status: OverallStatus::Warning,
            system: None,
            config: None,
            camera: None,
            python: None,
            groups: vec![FindingGroup {
                domain: FindingDomain::Thermal,
                findings: vec![finding.clone()],
            }],
            findings: vec![finding],
        }
    }
}
