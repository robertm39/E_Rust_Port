use std::process::ExitCode;

use e_rust_port::basics::error::{init_error, report_fatal_diagnostic};
use e_rust_port::prover::epatternize::{run, PROGRAM_NAME};

fn main() -> ExitCode {
    init_error(PROGRAM_NAME);
    let mut stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    let mut stderr = std::io::stderr();
    let status = match run(std::env::args(), &mut stdin, &mut stdout, &mut stderr) {
        Ok(status) => status,
        Err(error) => report_fatal_diagnostic(&mut stderr, error.code(), error.message()),
    };
    ExitCode::from(status)
}
