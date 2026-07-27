use std::io;
use std::process::ExitCode;

use e_rust_port::basics::error::{init_error_from_invocation, report_fatal_diagnostic};
use e_rust_port::simple_apps::term2dag::{run, PROGRAM_NAME};

fn main() -> ExitCode {
    init_error_from_invocation(PROGRAM_NAME);
    let mut stdin = io::stdin().lock();
    let mut stdout = io::stdout().lock();
    let mut stderr = io::stderr().lock();
    let status = match run(std::env::args(), &mut stdin, &mut stdout, &mut stderr) {
        Ok(status) => status,
        Err(error) => report_fatal_diagnostic(&mut stderr, error.code(), error.message()),
    };
    ExitCode::from(status)
}
