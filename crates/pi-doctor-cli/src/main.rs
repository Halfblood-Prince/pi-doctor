use clap::Parser;
use pi_doctor::cli::args::Cli;
use std::io::Write;
use std::process::ExitCode;

fn main() -> ExitCode {
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
