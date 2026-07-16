use std::io::Write;
use std::process::ExitCode;

use e_rust_port::prover::enormalizer::{run, PROGRAM_NAME};

fn main() -> ExitCode {
    let mut stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    let mut stderr = std::io::stderr();
    let status = match run(std::env::args(), &mut stdin, &mut stdout, &mut stderr) {
        Ok(status) => status,
        Err(error) => {
            if writeln!(stderr, "{PROGRAM_NAME}: {}", error.message()).is_err() {
                return ExitCode::from(error.code().exit_status());
            }
            error.code().exit_status()
        }
    };
    ExitCode::from(status)
}
