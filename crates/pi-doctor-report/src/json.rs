use pi_doctor_core::Report;

pub fn render(report: &Report) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(report)
}
