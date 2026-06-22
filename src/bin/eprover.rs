use std::io;
use std::io::Write;
use std::process::ExitCode;

use e_rust_port::prover::eprover::run;
use e_rust_port::prover::version::PROGRAM_NAME;

fn main() -> ExitCode {
    let mut stdout = io::stdout().lock();
    let mut stderr = io::stderr().lock();
    let status = match run(std::env::args(), &mut stdout, &mut stderr) {
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
