use crate::severity::Severity;
use serde::Serialize;
use std::collections::BTreeMap;
#[cfg(windows)]
use std::ffi::OsString;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

pub const DEFAULT_COMMAND_TIMEOUT: Duration = Duration::from_secs(3);
pub const DEFAULT_COMMAND_OUTPUT_LIMIT: usize = 64 * 1024;

#[derive(Debug, Clone, Serialize)]
pub struct Finding {
    pub id: &'static str,
    pub severity: Severity,
    pub impact: Impact,
    pub title: String,
    pub summary: String,
    pub evidence: Vec<String>,
    pub suggested_actions: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Impact {
    Info,
    Warning,
    Degraded,
    Critical,
}

#[derive(Debug, Clone)]
pub struct ProbeContext {
    root: Option<PathBuf>,
    commands: BTreeMap<String, CommandOutput>,
    command_timeout: Duration,
    command_output_limit: usize,
}

impl Default for ProbeContext {
    fn default() -> Self {
        Self {
            root: None,
            commands: BTreeMap::new(),
            command_timeout: DEFAULT_COMMAND_TIMEOUT,
            command_output_limit: DEFAULT_COMMAND_OUTPUT_LIMIT,
        }
    }
}

impl ProbeContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_root(root: impl Into<PathBuf>) -> Self {
        Self {
            root: Some(root.into()),
            commands: BTreeMap::new(),
            command_timeout: DEFAULT_COMMAND_TIMEOUT,
            command_output_limit: DEFAULT_COMMAND_OUTPUT_LIMIT,
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

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.command_timeout = timeout;
        self
    }

    pub fn with_output_limit(mut self, limit: usize) -> Self {
        self.command_output_limit = limit;
        self
    }

    pub fn read_text(&self, path: impl AsRef<Path>) -> Option<String> {
        self.read_text_result(path).ok()
    }

    pub fn read_text_result(&self, path: impl AsRef<Path>) -> std::io::Result<String> {
        std::fs::read_to_string(self.resolve_path(path.as_ref()))
    }

    pub fn path_exists(&self, path: impl AsRef<Path>) -> bool {
        self.resolve_path(path.as_ref()).exists()
    }

    pub fn list_dir(&self, path: impl AsRef<Path>) -> Vec<String> {
        let mut entries = std::fs::read_dir(self.resolve_path(path.as_ref()))
            .ok()
            .into_iter()
            .flatten()
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| entry.file_name().into_string().ok())
            .collect::<Vec<_>>();
        entries.sort();
        entries
    }

    fn resolve_path(&self, path: &Path) -> PathBuf {
        let relative = strip_root(path);
        match &self.root {
            Some(root) => root.join(relative),
            None => path.to_path_buf(),
        }
    }

    pub fn run_command(&self, program: &str, args: &[&str]) -> CommandOutput {
        if let Some(output) = self.commands.get(&command_key(program, args)) {
            return output.clone();
        }

        let mut child = match Command::new(program)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(child) => child,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return CommandOutput::Missing;
            }
            Err(error) => return CommandOutput::Failure(error.to_string()),
        };

        let stdout = child
            .stdout
            .take()
            .map(|stream| read_limited_output(stream, self.command_output_limit, "stdout"));
        let stderr = child
            .stderr
            .take()
            .map(|stream| read_limited_output(stream, self.command_output_limit, "stderr"));

        let started = Instant::now();
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) if started.elapsed() >= self.command_timeout => {
                    let _ = child.kill();
                    let _ = child.wait();
                    join_reader(stdout);
                    join_reader(stderr);
                    return CommandOutput::TimedOut;
                }
                Ok(None) => thread::sleep(Duration::from_millis(10)),
                Err(error) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    join_reader(stdout);
                    join_reader(stderr);
                    return CommandOutput::Failure(error.to_string());
                }
            }
        };

        let stdout = match join_reader(stdout) {
            Some(Ok(output)) => output,
            Some(Err(error)) => return CommandOutput::Failure(error),
            None => LimitedOutput::default(),
        };
        let stderr = match join_reader(stderr) {
            Some(Ok(output)) => output,
            Some(Err(error)) => return CommandOutput::Failure(error),
            None => LimitedOutput::default(),
        };

        if stdout.exceeded || stderr.exceeded {
            return CommandOutput::OutputLimitExceeded;
        }

        if status.success() {
            CommandOutput::Success(stdout.text.trim().to_owned())
        } else {
            let stderr = stderr.text.trim().to_owned();
            let stdout = stdout.text.trim().to_owned();
            let message = if stderr.is_empty() { stdout } else { stderr };
            CommandOutput::Failure(message)
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
    TimedOut,
    OutputLimitExceeded,
}

#[derive(Debug, Default)]
struct LimitedOutput {
    text: String,
    exceeded: bool,
}

fn read_limited_output<R>(
    reader: R,
    limit: usize,
    stream_name: &'static str,
) -> thread::JoinHandle<Result<LimitedOutput, String>>
where
    R: Read + Send + 'static,
{
    thread::spawn(move || {
        let mut bytes = Vec::new();
        let max = limit.saturating_add(1) as u64;
        reader
            .take(max)
            .read_to_end(&mut bytes)
            .map_err(|error| format!("failed to read {stream_name}: {error}"))?;
        let exceeded = bytes.len() > limit;
        if exceeded {
            bytes.truncate(limit);
        }
        Ok(LimitedOutput {
            text: String::from_utf8_lossy(&bytes).into_owned(),
            exceeded,
        })
    })
}

fn join_reader(
    handle: Option<thread::JoinHandle<Result<LimitedOutput, String>>>,
) -> Option<Result<LimitedOutput, String>> {
    handle.map(|handle| {
        handle
            .join()
            .unwrap_or_else(|_| Err("command output reader panicked".to_owned()))
    })
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
    use std::time::Duration;

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

    #[cfg(unix)]
    #[test]
    fn run_command_times_out() {
        let ctx = ProbeContext::new().with_timeout(Duration::from_millis(50));

        assert_eq!(
            ctx.run_command("sh", &["-c", "sleep 2"]),
            CommandOutput::TimedOut
        );
    }

    #[cfg(unix)]
    #[test]
    fn run_command_enforces_output_limit() {
        let ctx = ProbeContext::new().with_output_limit(8);

        assert_eq!(
            ctx.run_command("sh", &["-c", "printf 123456789"]),
            CommandOutput::OutputLimitExceeded
        );
    }
}
