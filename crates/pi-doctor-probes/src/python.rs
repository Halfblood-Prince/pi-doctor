use pi_doctor_core::{
    CommandOutput, Finding, Probe, ProbeContext, ProbeResult, PythonSummary, Severity,
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
    pub fn collect(&self, ctx: &ProbeContext) -> PythonAnalysis {
        let version = match ctx.run_command("python3", PYTHON_VERSION_ARGS) {
            CommandOutput::Success(output) => Some(output.trim().to_owned()),
            _ => None,
        };
        let executable = match ctx.run_command("python3", PYTHON_EXECUTABLE_ARGS) {
            CommandOutput::Success(output) => Some(output.trim().to_owned()),
            _ => None,
        };
        let in_virtualenv = matches!(
            ctx.run_command("python3", PYTHON_VENV_ARGS),
            CommandOutput::Success(output) if output.trim() == "1"
        );

        let externally_managed = match ctx.run_command("python3", PYTHON_STDLIB_ARGS) {
            CommandOutput::Success(output) => {
                let path = format!("{}/EXTERNALLY-MANAGED", output.trim().replace('\\', "/"));
                ctx.path_exists(path)
            }
            _ => false,
        };

        let mut detected_packages = Vec::new();
        if is_dpkg_installed(&ctx.run_command("dpkg-query", DPKG_PICAMERA2_ARGS)) {
            detected_packages.push("python3-picamera2".to_owned());
        }
        if is_dpkg_installed(&ctx.run_command("dpkg-query", DPKG_GPIOZERO_ARGS)) {
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

        PythonAnalysis { summary, findings }
    }
}

impl Probe for PythonProbe {
    fn run(&self, ctx: &ProbeContext) -> ProbeResult {
        self.collect(ctx).findings
    }
}

fn is_dpkg_installed(output: &CommandOutput) -> bool {
    matches!(output, CommandOutput::Success(text) if text.contains("install ok installed"))
}

fn python_findings(summary: &PythonSummary) -> Vec<Finding> {
    let mut findings = Vec::new();

    if summary.externally_managed {
        findings.push(Finding {
            id: "python.externally_managed",
            severity: Severity::Warning,
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
