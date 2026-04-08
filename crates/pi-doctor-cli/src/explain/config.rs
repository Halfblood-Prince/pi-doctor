use pi_doctor_core::ProbeContext;
use pi_doctor_probes::config_txt::{ConfigTxtProbe, summarize_entries_by_key};

pub fn render(ctx: &ProbeContext) -> String {
    let analysis = ConfigTxtProbe.collect(ctx);
    let summary = analysis.summary;
    let mut lines = vec![
        "pi-doctor explain config".to_owned(),
        "Boot configuration analysis".to_owned(),
        format!(
            "  source path: {}",
            summary.source_path.as_deref().unwrap_or("unavailable")
        ),
        format!(
            "  layout: {}",
            if summary.using_firmware_path {
                "modern /boot/firmware layout"
            } else {
                "legacy /boot layout or unknown"
            }
        ),
        format!("  diagnostics: {}", summary.diagnostics_count),
    ];

    if summary.legacy_path_present && summary.using_firmware_path {
        lines.push("  legacy /boot/config.txt also exists".to_owned());
    }

    lines.push(String::new());
    lines.push("Findings".to_owned());
    if analysis.findings.is_empty() {
        lines.push("  No config.txt warnings were detected.".to_owned());
    } else {
        for finding in &analysis.findings {
            lines.push(format!("  {}.", finding.title));
            lines.push(format!("  {}", finding.summary));
            for evidence in &finding.evidence {
                lines.push(format!("  Evidence: {evidence}"));
            }
        }
    }

    let key_summary = summarize_entries_by_key(&summary.entries);
    if !key_summary.is_empty() {
        lines.push(String::new());
        lines.push("Parsed keys".to_owned());
        for (key, lines_for_key) in key_summary {
            let joined = lines_for_key
                .iter()
                .map(|line| line.to_string())
                .collect::<Vec<_>>()
                .join(", ");
            lines.push(format!("  {key}: lines {joined}"));
        }
    }

    format!("{}\n", lines.join("\n"))
}
