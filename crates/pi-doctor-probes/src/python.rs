use crate::ProbeError;
use pi_doctor_core::{
    CommandOutput, Finding, Impact, Probe, ProbeContext, ProbeResult, PythonSummary, Severity,
};

const PYTHON_VERSION_ARGS: &[&str] = &["--version"];
const PYTHON_EXECUTABLE_ARGS: &[&str] = &["-c", "import sys; print(sys.executable)"];
const PYTHON_VENV_ARGS: &[&str] = &[
    "-c",
    "import sys; print(int(sys.prefix != sys.base_prefix))",
];
const PYTHON_STDLIB_ARGS: &[&str] = &[
    "-c",
    "import sysconfig; print(sysconfig.get_path('stdlib'))",
];
const DPKG_PICAMERA2_ARGS: &[&str] = &["-W", "-f=${Status}", "python3-picamera2"];
const DPKG_GPIOZERO_ARGS: &[&str] = &["-W", "-f=${Status}", "python3-gpiozero"];

#[derive(Debug, Clone, Default)]
pub struct PythonAnalysis {
    pub summary: PythonSummary,
    pub findings: Vec<Finding>,
}

pub struct PythonProbe;

impl PythonProbe {
    pub fn collect(&self, ctx: &ProbeContext) -> Result<PythonAnalysis, ProbeError> {
        let python_present = ctx.command_exists("python3");
        if !python_present {
            return Err(ProbeError::MissingTool { program: "python3" });
        }
        let version = python_single_line(ctx, PYTHON_VERSION_ARGS)?;
        let executable = python_single_line(ctx, PYTHON_EXECUTABLE_ARGS)?;
        let in_virtualenv = python_single_line(ctx, PYTHON_VENV_ARGS)?.as_deref() == Some("1");

        let externally_managed = if let Some(stdlib) = python_single_line(ctx, PYTHON_STDLIB_ARGS)?
        {
            let path = format!("{}/EXTERNALLY-MANAGED", stdlib.replace('\\', "/"));
            ctx.path_exists(path)
        } else {
            false
        };

        let mut detected_packages = Vec::new();
        let dpkg_present = ctx.command_exists("dpkg-query");
        if dpkg_present && dpkg_package_installed(ctx, DPKG_PICAMERA2_ARGS)? {
            detected_packages.push("python3-picamera2".to_owned());
        }
        if dpkg_present && dpkg_package_installed(ctx, DPKG_GPIOZERO_ARGS)? {
            detected_packages.push("python3-gpiozero".to_owned());
        }

        let summary = PythonSummary {
            version,
            executable,
            in_virtualenv,
            externally_managed,
            detected_packages,
        };
        let findings = python_findings(&summary);

        Ok(PythonAnalysis { summary, findings })
    }
}

impl Probe for PythonProbe {
    fn run(&self, ctx: &ProbeContext) -> ProbeResult {
        self.collect(ctx)
            .map(|analysis| analysis.findings)
            .unwrap_or_default()
    }
}

fn python_single_line(
    ctx: &ProbeContext,
    args: &'static [&'static str],
) -> Result<Option<String>, ProbeError> {
    command_single_line(ctx, "python3", args)
}

fn dpkg_package_installed(
    ctx: &ProbeContext,
    args: &'static [&'static str],
) -> Result<bool, ProbeError> {
    match ctx.run_command("dpkg-query", args) {
        CommandOutput::Success(text) => {
            Ok(text.split_whitespace().eq(["install", "ok", "installed"]))
        }
        CommandOutput::Failure(_) | CommandOutput::Missing => Ok(false),
        CommandOutput::TimedOut => Err(ProbeError::CommandTimedOut {
            program: "dpkg-query",
            args: args.join(" "),
        }),
        CommandOutput::OutputLimitExceeded => Err(ProbeError::CommandOutputLimit {
            program: "dpkg-query",
            args: args.join(" "),
        }),
    }
}

fn command_single_line(
    ctx: &ProbeContext,
    program: &'static str,
    args: &'static [&'static str],
) -> Result<Option<String>, ProbeError> {
    match ctx.run_command(program, args) {
        CommandOutput::Success(output) => Ok(normalize_single_line(&output)),
        CommandOutput::Missing => Err(ProbeError::MissingTool { program }),
        CommandOutput::Failure(detail) => Err(ProbeError::CommandFailure {
            program,
            args: args.join(" "),
            detail,
        }),
        CommandOutput::TimedOut => Err(ProbeError::CommandTimedOut {
            program,
            args: args.join(" "),
        }),
        CommandOutput::OutputLimitExceeded => Err(ProbeError::CommandOutputLimit {
            program,
            args: args.join(" "),
        }),
    }
}

fn normalize_single_line(output: &str) -> Option<String> {
    output
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(str::to_owned)
}

fn python_findings(summary: &PythonSummary) -> Vec<Finding> {
    let mut findings = Vec::new();

    if summary.externally_managed {
        findings.push(Finding {
            id: "python.externally_managed",
            severity: Severity::Warning,
            impact: Impact::Warning,
            title: "System Python is externally managed".to_owned(),
            summary: "This Python installation is marked EXTERNALLY-MANAGED, which means pip installs should generally happen inside a virtual environment.".to_owned(),
            evidence: vec![format!(
                "python executable: {}",
                summary.executable.as_deref().unwrap_or("unknown")
            )],
            suggested_actions: vec![
                "Why this matters: Bookworm and newer Raspberry Pi OS releases discourage mutating the system Python environment with plain `pip install`.".to_owned(),
                "What to run next: create a virtual environment for pip packages, or prefer `apt install python3-...` when a distro package exists.".to_owned(),
            ],
        });
    }

    if !summary.in_virtualenv {
        findings.push(Finding {
            id: "python.no_virtualenv",
            severity: Severity::Warning,
            impact: Impact::Warning,
            title: "No active virtual environment detected".to_owned(),
            summary: "Python appears to be running outside a virtual environment.".to_owned(),
            evidence: vec![format!(
                "python version: {}",
                summary.version.as_deref().unwrap_or("unknown")
            )],
            suggested_actions: vec![
                "Why this matters: without a venv, project-specific pip installs can collide with system packages or Bookworm external-management rules.".to_owned(),
                "What to run next: run `python3 -m venv .venv` and activate it before installing pip-only packages.".to_owned(),
            ],
        });
    }

    if summary
        .detected_packages
        .iter()
        .any(|package| package == "python3-picamera2")
    {
        findings.push(Finding {
            id: "python.picamera2_apt_present",
            severity: Severity::Info,
            impact: Impact::Info,
            title: "Picamera2 is installed from the distro package".to_owned(),
            summary: "The `python3-picamera2` package is present, which is usually the preferred Raspberry Pi OS path.".to_owned(),
            evidence: vec!["package detected: python3-picamera2".to_owned()],
            suggested_actions: vec![
                "Why this matters: Raspberry Pi OS usually packages Picamera2 directly, which avoids fragile source installs.".to_owned(),
                "What to run next: import Picamera2 from the system package or install it with apt on similar systems.".to_owned(),
            ],
        });
    }

    findings
}
