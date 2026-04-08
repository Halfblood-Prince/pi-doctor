use crate::severity::Severity;
use serde::Serialize;
use std::collections::BTreeMap;
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

pub type ProbeResult = Vec<Finding>;

pub trait Probe {
    fn run(&self, ctx: &ProbeContext) -> ProbeResult;
}
