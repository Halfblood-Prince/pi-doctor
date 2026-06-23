pub mod board;
pub mod camera;
pub mod config_txt;
pub mod error;
pub mod gpio;
pub mod kernel;
pub mod os;
pub mod python;
pub mod thermal;
pub mod throttling;

pub use error::ProbeError;

pub(crate) fn read_optional_text(
    ctx: &pi_doctor_core::ProbeContext,
    path: &'static str,
) -> Result<Option<String>, ProbeError> {
    match ctx.read_text_result(path) {
        Ok(contents) => Ok(Some(contents)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
            Err(ProbeError::PermissionDenied { path })
        }
        Err(_) => Err(ProbeError::ReadText { path }),
    }
}
