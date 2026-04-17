use clap::Parser;
use pi_doctor::cli::args::Cli;
use std::io::Write;
use std::process::ExitCode;

fn main() -> ExitCode {
    let _ = env_logger::Builder::from_env(
        env_logger::Env::default().filter_or("PI_DOCTOR_LOG", "warn"),
    )
    .format_timestamp(None)
    .try_init();

    let cli = Cli::parse();
    match pi_doctor::run(cli) {
        Ok(response) => {
            print!("{}", response.output);
            ExitCode::from(response.exit_code)
        }
        Err(error) => {
            let _ = writeln!(std::io::stderr(), "{error}");
            ExitCode::from(4)
        }
    }
}
