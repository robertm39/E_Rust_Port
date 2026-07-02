use std::process::ExitCode;

use e_rust_port::prover::eground::{run, PROGRAM_NAME};

fn main() -> ExitCode {
    let mut stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    let mut stderr = std::io::stderr();
    match run(std::env::args(), &mut stdin, &mut stdout, &mut stderr) {
        Ok(status) => ExitCode::from(status),
        Err(error) => {
            eprintln!("{PROGRAM_NAME}: {error}");
            ExitCode::from(1)
        }
    }
}
