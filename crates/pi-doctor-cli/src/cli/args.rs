use clap::{CommandFactory, Parser, Subcommand, ValueEnum, error::ErrorKind};
use clap_complete::Shell;

#[derive(Debug, Parser)]
#[command(
    name = "pi-doctor",
    version,
    about = "Human-first Raspberry Pi diagnostics",
    long_about = None,
    arg_required_else_help = true
)]
pub struct Cli {
    #[arg(long, global = true)]
    pub json: bool,

    #[arg(long, global = true, conflicts_with_all = ["verbose", "json"])]
    pub quiet: bool,

    #[arg(long, global = true, conflicts_with_all = ["quiet", "json"])]
    pub verbose: bool,

    #[arg(long, global = true, conflicts_with = "json")]
    pub no_color: bool,

    #[arg(long, global = true, value_name = "SECONDS", default_value_t = 3)]
    pub timeout: u64,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    #[command(about = "Run the full diagnostic probe set")]
    Check {},
    #[command(about = "Explain a Raspberry Pi diagnostic topic")]
    Explain {
        #[arg(value_enum)]
        topic: ExplainTopic,
    },
    #[command(about = "Create a support bundle with reports and raw captures")]
    SupportBundle,
    #[command(about = "Run a focused diagnostic doctor")]
    Doctor {
        #[arg(value_enum)]
        target: DoctorTarget,
    },
    #[command(about = "Generate shell completions")]
    Completions {
        #[arg(value_enum)]
        shell: Shell,
    },
}

#[derive(Clone, Debug, ValueEnum)]
pub enum ExplainTopic {
    Throttling,
    Config,
    Python,
}

impl ExplainTopic {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Throttling => "throttling",
            Self::Config => "config",
            Self::Python => "python",
        }
    }
}

#[derive(Clone, Debug, ValueEnum)]
pub enum DoctorTarget {
    Camera,
    Gpio,
}

impl DoctorTarget {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Camera => "camera",
            Self::Gpio => "gpio",
        }
    }
}

impl Cli {
    pub fn validate(self) -> clap::error::Result<Self> {
        if self.json && !matches!(self.command, Commands::Check {}) {
            return Err(Self::command().error(
                ErrorKind::ArgumentConflict,
                "`--json` is only supported with `pi-doctor check`",
            ));
        }

        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::Cli;
    use clap::Parser;

    #[test]
    fn rejects_json_for_non_check_commands() {
        let error = Cli::try_parse_from(["pi-doctor", "--json", "doctor", "gpio"])
            .and_then(Cli::validate)
            .expect_err("json should be rejected for non-check commands");

        assert_eq!(error.kind(), clap::error::ErrorKind::ArgumentConflict);
        assert!(
            error
                .to_string()
                .contains("only supported with `pi-doctor check`")
        );
    }

    #[test]
    fn rejects_quiet_with_json() {
        let error = Cli::try_parse_from(["pi-doctor", "--json", "--quiet", "check"])
            .expect_err("quiet should conflict with json");

        assert_eq!(error.kind(), clap::error::ErrorKind::ArgumentConflict);
    }

    #[test]
    fn rejects_no_color_with_json() {
        let error = Cli::try_parse_from(["pi-doctor", "--json", "--no-color", "check"])
            .expect_err("no-color should conflict with json");

        assert_eq!(error.kind(), clap::error::ErrorKind::ArgumentConflict);
    }

    #[test]
    fn parses_global_timeout() {
        let cli = Cli::try_parse_from(["pi-doctor", "--timeout", "9", "check"])
            .and_then(Cli::validate)
            .expect("timeout should parse");

        assert_eq!(cli.timeout, 9);
    }
}
