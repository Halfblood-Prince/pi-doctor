#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum ProbeError {
    #[error("missing required field `{field}` for {probe}")]
    MissingField {
        probe: &'static str,
        field: &'static str,
    },
    #[error("failed to read {path}")]
    ReadText { path: &'static str },
    #[error("permission denied reading {path}")]
    PermissionDenied { path: &'static str },
    #[error("failed to run `{program}` with args `{args}`: {detail}")]
    CommandFailure {
        program: &'static str,
        args: String,
        detail: String,
    },
    #[error("timed out running `{program}` with args `{args}`")]
    CommandTimedOut {
        program: &'static str,
        args: String,
    },
    #[error("output limit exceeded while running `{program}` with args `{args}`")]
    CommandOutputLimit {
        program: &'static str,
        args: String,
    },
    #[error("missing required tool `{program}`")]
    MissingTool { program: &'static str },
    #[error("{probe} parse error: {detail}")]
    Parse { probe: &'static str, detail: String },
}
