#[derive(Debug, thiserror::Error)]
pub enum CliError {
    #[error("bundle generation failed")]
    Bundle(#[from] pi_doctor_bundle::BundleError),
    #[error("report rendering failed")]
    Report(#[from] pi_doctor_report::ReportError),
    #[error("failed to encode completions as UTF-8")]
    Utf8(#[from] std::string::FromUtf8Error),
    #[error("failed to read system time")]
    SystemTime(#[from] std::time::SystemTimeError),
}
