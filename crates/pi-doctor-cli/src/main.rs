use clap::Parser;
use pi_doctor::cli::args::Cli;
use std::io::Write;
use std::process::ExitCode;

fn main() -> ExitCode {
    pi_doctor::init_logging();

    let cli = match Cli::try_parse().and_then(Cli::validate) {
        Ok(cli) => cli,
        Err(error) => {
            let _ = error.print();
            return ExitCode::from(error.exit_code() as u8);
        }
    };
    match pi_doctor::run(cli) {
        Ok(response) => {
            print!("{}", response.output);
            ExitCode::from(response.exit_code)
        }
        Err(error) => {
            let _ = writeln!(std::io::stderr(), "{error:#}");
            ExitCode::from(4)
        }
    }
}
