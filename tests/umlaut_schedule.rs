use std::io::Write;
use std::process::{Command, Stdio};

#[test]
fn auto_schedule_runs_worker_process_and_replays_winner() {
    let path = write_false_problem("auto-schedule");

    let output = Command::new(env!("CARGO_BIN_EXE_umlaut"))
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
            == 1,
        "{stdout}"
    );
    assert!(stdout.contains("% Starting G-E--_302_C18_F1_URBAN_RG_S04BN"));
    assert!(stdout.contains("% Result found by G-E--_302_C18_F1_URBAN_RG_S04BN\n"));
    assert!(stdout.contains("% Search class: FUHPF-FFSF00-SFFFFFNN\n"));
    assert!(stdout.contains("% Scheduled 6 strats onto 1 cores with 300 seconds (300 total)\n"));
    assert!(stdout.contains("% Starting SAT001_MinMin_p005000_rr_RG"));
    assert!(stdout.contains("% Result found by SAT001_MinMin_p005000_rr_RG\n"));
    assert!(stdout.contains("% Proof found!\n% SZS status Unsatisfiable\n"));
    assert!(!stdout.contains("umlaut-schedule"), "{stdout}");
    assert!(!stdout.contains("strategy scheduling process execution is not ported yet"));
    assert!(stderr.is_empty(), "{stderr}");

    std::fs::remove_file(path).unwrap();
}

#[test]
fn auto_schedule_resources_info_replays_worker_preprocessing_time() {
    let path = write_false_problem("auto-schedule-resources");

    let output = Command::new(env!("CARGO_BIN_EXE_umlaut"))
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
    let proof_position = stdout.find("% Proof found!\n").unwrap();
    let resource_positions = stdout
        .match_indices("% User time                : ")
        .map(|(position, _)| position)
        .collect::<Vec<_>>();
    assert_eq!(resource_positions.len(), 2, "{stdout}");
    assert!(
        resource_positions
            .iter()
            .all(|position| *position > proof_position),
        "{stdout}"
    );
    let total_times = stdout
        .lines()
        .filter_map(|line| {
            line.strip_prefix("% Total time               : ")?
                .strip_suffix(" s")?
                .parse::<f64>()
                .ok()
        })
        .collect::<Vec<_>>();
    assert_eq!(total_times.len(), 2, "{stdout}");
    assert!(total_times[1] >= total_times[0], "{stdout}");
    assert!(stderr.is_empty(), "{stderr}");

    std::fs::remove_file(path).unwrap();
}

#[test]
fn auto_schedule_cnf_allows_nested_search_preprocessing_proof() {
    let path = write_false_problem("auto-schedule-cnf");

    let output = Command::new(env!("CARGO_BIN_EXE_umlaut"))
        .arg("--auto-schedule=1")
        .arg("--resources-info")
        .arg("--cnf")
        .arg("--tstp-in")
        .arg(&path)
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert_eq!(output.status.code(), Some(0), "{stdout}\n{stderr}");
    assert!(stdout.contains("% Scheduled 6 strats onto 1 cores with 300 seconds (300 total)\n"));
    assert!(stdout.contains("% Result found by SAT001_MinMin_p005000_rr_RG\n"));
    assert!(stdout.contains("% Proof found!\n% SZS status Unsatisfiable\n"));
    assert_eq!(
        stdout.matches("% User time                : ").count(),
        2,
        "{stdout}"
    );
    assert!(stderr.is_empty(), "{stderr}");

    std::fs::remove_file(path).unwrap();
}

#[test]
fn auto_schedule_replays_snapshotted_standard_input_in_nested_workers() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_umlaut"))
        .arg("--auto-schedule=1")
        .arg("--tstp-in")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"cnf(stdin_false, axiom, ($false)).\n")
        .unwrap();

    let output = child.wait_with_output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();

    assert_eq!(output.status.code(), Some(0), "{stdout}\n{stderr}");
    assert!(stdout.contains("% Result found by G-E--_302_C18_F1_URBAN_RG_S04BN\n"));
    assert!(stdout.contains("% Result found by SAT001_MinMin_p005000_rr_RG\n"));
    assert!(stdout.contains("% Proof found!\n% SZS status Unsatisfiable\n"));
    assert!(!stdout.contains("% No proof found!"), "{stdout}");
    assert!(stderr.is_empty(), "{stderr}");
}

#[test]
fn auto_mode_classifies_cnf_inputs_as_pre_cnf_formula_owners() {
    let path = write_problem(
        "auto-mixed-owner-class",
        "cnf(identity, axiom, (i(X1)=i(X2))).\n\
         cnf(comm_f, axiom, (f(X1,X2)=f(X2,X1))).\n\
         cnf(comm_g, axiom, (g(X1,X2)=g(X2,X1))).\n\
         cnf(ass_f, axiom, (f(f(X1,X2),X3)=f(X1,f(X2,X3)))).\n\
         cnf(p_holds, axiom, (p(X1))).\n\
         cnf(consts1, axiom, (a=b|c=a|e=a)).\n\
         cnf(consts2, axiom, (a=b|c=a|e!=a)).\n\
         cnf(split_or_condense, axiom, (c=b|X3!=X4|X1!=X2|d!=c)).\n\
         fof(guarded_eq, axiom, ((d=c|d=c)<=>h(i(e))=h(i(a)))).\n\
         fof(conj, conjecture, (?[X1,X2,X3,X4,X5]:((k(a,b)=k(X1,X1)&f(f(g(X4,X5),X3),f(X2,X1))=f(f(X1,X2),f(X3,g(X4,X5))))))&![X6]:p(X6)).\n",
    );

    let output = Command::new(env!("CARGO_BIN_EXE_umlaut"))
        .arg("--auto")
        .arg("--cnf")
        .arg("--silent")
        .arg("--tstp-in")
        .arg(&path)
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert_eq!(output.status.code(), Some(0), "{stdout}\n{stderr}");
    assert!(
        stdout.contains("% Preprocessing class: FSMSSMSSSSSNFFN.\n"),
        "{stdout}"
    );
    assert!(stdout.contains("% Configuration: G-E--_208_C18_F1_SE_CS_SOS_SP_PS_S5PRR_RG_S04AN\n"));
    assert!(stderr.is_empty(), "{stderr}");

    std::fs::remove_file(path).unwrap();
}

fn write_false_problem(name: &str) -> std::path::PathBuf {
    write_problem(name, "cnf(a, axiom, ($false)).\n")
}

fn write_problem(name: &str, contents: &str) -> std::path::PathBuf {
    let path = std::env::current_dir()
        .unwrap()
        .join("target")
        .join(format!("umlaut-{name}-{}.p", std::process::id()));
    std::fs::write(&path, contents).unwrap();
    path
}
