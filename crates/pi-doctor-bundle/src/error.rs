use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum BundleError {
    #[error("failed to create bundle directory at {path}")]
    CreateDir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to render report content")]
    Report(#[from] pi_doctor_report::ReportError),
    #[error("failed to write bundle file at {path}")]
    WriteFile {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}
