use crate::severity::Severity;
use serde::Serialize;
use std::collections::BTreeMap;
#[cfg(windows)]
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Serialize)]
pub struct Finding {
    pub id: &'static str,
    pub severity: Severity,
    pub title: String,
    pub summary: String,
    pub evidence: Vec<String>,
    pub suggested_actions: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ProbeContext {
    root: Option<PathBuf>,
    commands: BTreeMap<String, CommandOutput>,
}

impl ProbeContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_root(root: impl Into<PathBuf>) -> Self {
        Self {
            root: Some(root.into()),
            commands: BTreeMap::new(),
        }
    }

    pub fn with_command_output(
        mut self,
        program: &str,
        args: &[&str],
        output: CommandOutput,
    ) -> Self {
        self.commands.insert(command_key(program, args), output);
        self
    }

    pub fn read_text(&self, path: impl AsRef<Path>) -> Option<String> {
        let relative = strip_root(path.as_ref());
        let full_path = match &self.root {
            Some(root) => root.join(relative),
            None => path.as_ref().to_path_buf(),
        };

        std::fs::read_to_string(full_path).ok()
    }

    pub fn path_exists(&self, path: impl AsRef<Path>) -> bool {
        let relative = strip_root(path.as_ref());
        let full_path = match &self.root {
            Some(root) => root.join(relative),
            None => path.as_ref().to_path_buf(),
        };

        full_path.exists()
    }

    pub fn list_dir(&self, path: impl AsRef<Path>) -> Vec<String> {
        let relative = strip_root(path.as_ref());
        let full_path = match &self.root {
            Some(root) => root.join(relative),
            None => path.as_ref().to_path_buf(),
        };

        let mut entries = std::fs::read_dir(full_path)
            .ok()
            .into_iter()
            .flatten()
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| entry.file_name().into_string().ok())
            .collect::<Vec<_>>();
        entries.sort();
        entries
    }

    pub fn run_command(&self, program: &str, args: &[&str]) -> CommandOutput {
        if let Some(output) = self.commands.get(&command_key(program, args)) {
            return output.clone();
        }

        match Command::new(program).args(args).output() {
            Ok(output) if output.status.success() => {
                CommandOutput::Success(String::from_utf8_lossy(&output.stdout).trim().to_owned())
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
                let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
                let message = if stderr.is_empty() { stdout } else { stderr };
                CommandOutput::Failure(message)
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => CommandOutput::Missing,
            Err(error) => CommandOutput::Failure(error.to_string()),
        }
    }

    pub fn command_exists(&self, program: &str) -> bool {
        let mut saw_mock = false;
        let mut found_present_mock = false;

        for (key, output) in &self.commands {
            let mocked_program = key.split('\0').next().unwrap_or_default();
            if mocked_program == program {
                saw_mock = true;
                if !matches!(output, CommandOutput::Missing) {
                    found_present_mock = true;
                }
            }
        }

        if saw_mock {
            return found_present_mock;
        }

        command_in_path(program)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandOutput {
    Success(String),
    Missing,
    Failure(String),
}

fn command_key(program: &str, args: &[&str]) -> String {
    let mut key = program.to_owned();
    for arg in args {
        key.push('\0');
        key.push_str(arg);
    }
    key
}

fn strip_root(path: &Path) -> PathBuf {
    let mut relative = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::RootDir | std::path::Component::Prefix(_) => {}
            other => relative.push(other.as_os_str()),
        }
    }

    relative
}

fn command_in_path(program: &str) -> bool {
    let candidate = Path::new(program);
    if candidate.components().count() > 1 {
        return candidate.is_file();
    }

    let Some(paths) = std::env::var_os("PATH") else {
        return false;
    };

    #[cfg(windows)]
    let exts = windows_path_exts();

    for dir in std::env::split_paths(&paths) {
        let joined = dir.join(program);
        if joined.is_file() {
            return true;
        }

        #[cfg(windows)]
        for ext in &exts {
            let with_ext = dir.join(format!("{program}{ext}"));
            if with_ext.is_file() {
                return true;
            }
        }
    }

    false
}

#[cfg(windows)]
fn windows_path_exts() -> Vec<String> {
    let raw = std::env::var_os("PATHEXT").unwrap_or_else(|| OsString::from(".COM;.EXE;.BAT;.CMD"));
    raw.to_string_lossy()
        .split(';')
        .filter(|ext| !ext.is_empty())
        .map(|ext| ext.to_ascii_lowercase())
        .collect()
}

pub type ProbeResult = Vec<Finding>;

pub trait Probe {
    fn run(&self, ctx: &ProbeContext) -> ProbeResult;
}

#[cfg(test)]
mod tests {
    use super::{CommandOutput, ProbeContext};

    #[test]
    fn command_exists_uses_mocked_outputs() {
        let ctx = ProbeContext::new()
            .with_command_output("vcgencmd", &["version"], CommandOutput::Missing)
            .with_command_output(
                "python3",
                &["--version"],
                CommandOutput::Success("Python 3.11.0".to_owned()),
            );

        assert!(!ctx.command_exists("vcgencmd"));
        assert!(ctx.command_exists("python3"));
    }
}
