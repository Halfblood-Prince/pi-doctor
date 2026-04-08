use pi_doctor_core::ProbeContext;
use pi_doctor_probes::python::PythonProbe;

pub fn render(ctx: &ProbeContext) -> String {
    let analysis = PythonProbe.collect(ctx);
    let summary = analysis.summary;
    let mut lines = vec![
        "pi-doctor explain python".to_owned(),
        "Python environment analysis".to_owned(),
        format!(
            "  version: {}",
            summary.version.as_deref().unwrap_or("unknown")
        ),
        format!(
            "  executable: {}",
            summary.executable.as_deref().unwrap_or("unknown")
        ),
        format!(
            "  virtual environment: {}",
            if summary.in_virtualenv {
                "active"
            } else {
                "not active"
            }
        ),
        format!(
            "  externally managed: {}",
            if summary.externally_managed {
                "yes"
            } else {
                "no"
            }
        ),
    ];

    if !summary.detected_packages.is_empty() {
        lines.push(format!(
            "  detected distro packages: {}",
            summary.detected_packages.join(", ")
        ));
    }

    lines.push(String::new());
    lines.push("Guidance".to_owned());
    if analysis.findings.is_empty() {
        lines.push("  No immediate Python packaging warnings were detected.".to_owned());
    } else {
        for finding in &analysis.findings {
            lines.push(format!("  {}.", finding.title));
            lines.push(format!("  {}", finding.summary));
        }
    }

    lines.push(String::new());
    lines.push("Exact next commands".to_owned());
    if summary.externally_managed && !summary.in_virtualenv {
        lines.push("  Create and use a virtual environment for pip-managed packages:".to_owned());
        lines.push("      python3 -m venv .venv".to_owned());
        lines.push("      . .venv/bin/activate".to_owned());
        lines.push("      python -m pip install --upgrade pip".to_owned());
    } else if summary.in_virtualenv {
        lines.push("  Install pip-only packages inside the active venv:".to_owned());
        lines.push("      python -m pip install <package>".to_owned());
    } else {
        lines.push("  System Python is not flagged as externally managed, but a venv is still safer for project packages:".to_owned());
        lines.push("      python3 -m venv .venv".to_owned());
        lines.push("      . .venv/bin/activate".to_owned());
    }

    lines.push("  Prefer apt when Raspberry Pi OS already ships the package:".to_owned());
    lines.push("      sudo apt install python3-picamera2 python3-gpiozero".to_owned());

    format!("{}\n", lines.join("\n"))
}
