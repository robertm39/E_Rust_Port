use std::process::Command;

#[test]
fn auto_schedule_runs_worker_process_and_replays_winner() {
    let path = write_false_problem("auto-schedule");

    let output = Command::new(env!("CARGO_BIN_EXE_eprover"))
        .arg("--auto-schedule=1")
        .arg("--tstp-in")
        .arg(&path)
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert_eq!(output.status.code(), Some(0), "{stdout}\n{stderr}");
    assert!(stdout.contains("% Preprocessing class: FSSSSMSSSSSNFFN.\n"));
    assert!(
        stdout
            .matches("% Scheduled 1 strats onto 1 cores with ")
            .count()
            >= 2,
        "{stdout}"
    );
    assert!(stdout.contains("% Starting G-E--_302_C18_F1_URBAN_RG_S04BN"));
    assert!(stdout.contains("% Result found by G-E--_302_C18_F1_URBAN_RG_S04BN\n"));
    assert!(stdout.contains("% Search class: FUHPF-FFSF00-SFFFFFNN\n"));
    assert!(stdout.contains("% Starting SAT001_MinMin_p005000_rr_RG"));
    assert!(stdout.contains("% Result found by SAT001_MinMin_p005000_rr_RG\n"));
    assert!(stdout.contains("% Proof found!\n% SZS status Unsatisfiable\n"));
    assert!(!stdout.contains("e-rust-port-schedule"), "{stdout}");
    assert!(!stdout.contains("strategy scheduling process execution is not ported yet"));
    assert!(stderr.is_empty(), "{stderr}");

    std::fs::remove_file(path).unwrap();
}

#[test]
fn auto_schedule_resources_info_replays_worker_preprocessing_time() {
    let path = write_false_problem("auto-schedule-resources");

    let output = Command::new(env!("CARGO_BIN_EXE_eprover"))
        .arg("--auto-schedule=1")
        .arg("--resources-info")
        .arg("--tstp-in")
        .arg(&path)
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert_eq!(output.status.code(), Some(0), "{stdout}\n{stderr}");
    assert!(stdout.contains("% Result found by G-E--_302_C18_F1_URBAN_RG_S04BN\n"));
    assert!(stdout.contains("% Result found by SAT001_MinMin_p005000_rr_RG\n"));
    assert!(stdout.contains("% Preprocessing time       : "));
    assert!(stdout.contains("% Proof found!\n% SZS status Unsatisfiable\n"));
    assert!(
        stdout.matches("% User time                : ").count() >= 3,
        "{stdout}"
    );
    assert!(stderr.is_empty(), "{stderr}");

    std::fs::remove_file(path).unwrap();
}

fn write_false_problem(name: &str) -> std::path::PathBuf {
    let path = std::env::current_dir()
        .unwrap()
        .join("target")
        .join(format!("eprover-{name}-{}.p", std::process::id()));
    std::fs::write(&path, "cnf(a, axiom, ($false)).\n").unwrap();
    path
}
