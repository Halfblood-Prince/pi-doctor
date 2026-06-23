#[derive(Debug, thiserror::Error)]
pub enum ReportError {
    #[error("failed to serialize report as JSON")]
    Json(#[from] serde_json::Error),
}
