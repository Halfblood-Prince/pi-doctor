use clap::{Parser, Subcommand, ValueEnum};
use clap_complete::Shell;

#[derive(Debug, Parser)]
#[command(
    name = "pi-doctor",
    version,
    about = "Human-first Raspberry Pi diagnostics",
    long_about = None
)]
pub struct Cli {
    #[arg(long, global = true)]
    pub json: bool,

    #[arg(long, global = true, conflicts_with = "verbose")]
    pub quiet: bool,

    #[arg(long, global = true, conflicts_with = "quiet")]
    pub verbose: bool,

    #[arg(long, global = true)]
    pub no_color: bool,

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
