#![cfg(target_os = "linux")]

use std::process::Command;

const HARD_TIMEOUT_OUTPUT: &str =
    "\n%% Failure: Resource limit exceeded (time)\n%% SZS status ResourceOut\n";
const HARD_TIMEOUT_ERROR: &str = "eprover: CPU time limit exceeded, terminating\n";

#[test]
fn zero_cpu_limit_reports_hard_timeout_once() {
    for attempt in 0..8 {
        let path = std::env::current_dir()
            .unwrap()
            .join("target")
            .join(format!(
                "eprover-hard-limit-{}-{attempt}.p",
                std::process::id()
            ));
        std::fs::write(&path, "p(a).\n").unwrap();

        let output = Command::new(env!("CARGO_BIN_EXE_eprover"))
            .arg("--lop-in")
            .arg("--auto")
            .arg("--cpu-limit=0")
            .arg(&path)
            .output()
            .unwrap();

        let stdout = String::from_utf8(output.stdout).unwrap();
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert_eq!(output.status.code(), Some(8), "{stdout}\n{stderr}");
        assert_eq!(stdout.matches(HARD_TIMEOUT_OUTPUT).count(), 1, "{stdout}");
        assert_eq!(stderr, HARD_TIMEOUT_ERROR);

        std::fs::remove_file(path).unwrap();
    }
}
