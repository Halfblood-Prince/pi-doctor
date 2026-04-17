use crate::ReportError;
use pi_doctor_core::Report;

pub fn render(report: &Report) -> Result<String, ReportError> {
    Ok(serde_json::to_string_pretty(report)?)
}
