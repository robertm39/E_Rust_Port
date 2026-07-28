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

#[test]
fn external_alarm_does_not_orphan_scheduled_workers() {
    let path = std::env::current_dir()
        .unwrap()
        .join("target")
        .join(format!("umlaut-schedule-alarm-{}.p", std::process::id()));
    let mut problem = std::fs::File::create(&path).unwrap();
    problem.write_all(b"%").unwrap();
    let comment_chunk = vec![b'x'; 64 * 1024].into_boxed_slice();
    for _ in 0..16 {
        problem.write_all(&comment_chunk).unwrap();
    }
    problem
        .write_all(b"\nfof(schedule_alarm_goal, conjecture, p(a)).\n")
        .unwrap();
    drop(problem);

    let mut child = Command::new(env!("CARGO_BIN_EXE_umlaut"))
        .arg("--auto-schedule=8")
        .arg("--memory-limit=512")
        .arg("--output-level=0")
        .arg(&path)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let workers = wait_for_linux_children(&mut child, Duration::from_secs(10));
    assert!(!workers.is_empty());
    let expected_address_limit = 512_u64 * 1024 * 1024;
    for process_id in std::iter::once(child.id()).chain(workers.iter().copied()) {
        assert_eq!(
            linux_address_space_limit(process_id),
            Some(expected_address_limit),
            "process {process_id} did not inherit the configured address-space limit"
        );
    }

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
    let status = child.wait().unwrap();
    assert_eq!(status.code(), Some(8));

    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    while std::time::Instant::now() < deadline
        && workers.iter().any(|worker| linux_process_exists(*worker))
    {
        std::thread::sleep(Duration::from_millis(10));
    }
    let surviving_workers = workers
        .iter()
        .copied()
        .filter(|worker| linux_process_exists(*worker))
        .collect::<Vec<_>>();
    assert!(
        surviving_workers.is_empty(),
        "scheduled workers survived parent hard timeout: {surviving_workers:?}"
    );

    std::fs::remove_file(path).unwrap();
}

fn wait_for_linux_children(child: &mut std::process::Child, timeout: Duration) -> Vec<u32> {
    let deadline = std::time::Instant::now() + timeout;
    loop {
        let children_path = format!("/proc/{}/task/{}/children", child.id(), child.id());
        if let Ok(children) = std::fs::read_to_string(children_path) {
            let process_ids = children
                .split_whitespace()
                .filter_map(|value| value.parse::<u32>().ok())
                .collect::<Vec<_>>();
            if !process_ids.is_empty() {
                return process_ids;
            }
        }
        if let Some(status) = child.try_wait().unwrap() {
            panic!("scheduler exited before spawning workers: {status}");
        }
        assert!(
            std::time::Instant::now() < deadline,
            "scheduler did not spawn a worker within {timeout:?}"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
}

fn linux_process_exists(process_id: u32) -> bool {
    std::path::Path::new(&format!("/proc/{process_id}")).exists()
}

fn linux_address_space_limit(process_id: u32) -> Option<u64> {
    let limits = std::fs::read_to_string(format!("/proc/{process_id}/limits")).ok()?;
    let line = limits
        .lines()
        .find(|line| line.starts_with("Max address space"))?;
    line.split_whitespace().nth(3)?.parse().ok()
}
