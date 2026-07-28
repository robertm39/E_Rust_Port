#![cfg(target_os = "linux")]

use std::io::Write;
use std::process::{Command, Stdio};
use std::time::Duration;

const HARD_TIMEOUT_OUTPUT: &str =
    "\n%% Failure: Resource limit exceeded (time)\n%% SZS status ResourceOut\n";
const HARD_TIMEOUT_ERROR: &str = "umlaut: CPU time limit exceeded, terminating\n";

#[test]
fn zero_cpu_limit_reports_hard_timeout_once() {
    for attempt in 0..8 {
        let path = std::env::current_dir()
            .unwrap()
            .join("target")
            .join(format!(
                "umlaut-hard-limit-{}-{attempt}.p",
                std::process::id()
            ));
        std::fs::write(&path, "p(a).\n").unwrap();

        let output = Command::new(env!("CARGO_BIN_EXE_umlaut"))
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

#[test]
fn external_alarm_reports_hard_timeout_without_allocator_reentry() {
    let path = std::env::current_dir()
        .unwrap()
        .join("target")
        .join(format!("umlaut-alarm-limit-{}.p", std::process::id()));
    let mut problem = std::fs::File::create(&path).unwrap();
    problem.write_all(b"%").unwrap();
    let comment_chunk = vec![b'x'; 64 * 1024].into_boxed_slice();
    for _ in 0..1024 {
        problem.write_all(&comment_chunk).unwrap();
    }
    problem
        .write_all(b"\nfof(alarm_goal, conjecture, $true).\n")
        .unwrap();
    drop(problem);

    let child = Command::new(env!("CARGO_BIN_EXE_umlaut"))
        .arg("--auto")
        .arg("--output-level=0")
        .arg(&path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    std::thread::sleep(Duration::from_millis(50));
    let kill = Command::new("kill")
        .arg("-ALRM")
        .arg(child.id().to_string())
        .output()
        .unwrap();
    assert!(
        kill.status.success(),
        "kill failed: {}",
        String::from_utf8_lossy(&kill.stderr)
    );
    let output = child.wait_with_output().unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert_eq!(output.status.code(), Some(8), "{stdout}\n{stderr}");
    assert_eq!(stdout.matches(HARD_TIMEOUT_OUTPUT).count(), 1, "{stdout}");
    assert_eq!(stderr, HARD_TIMEOUT_ERROR);

    std::fs::remove_file(path).unwrap();
}
