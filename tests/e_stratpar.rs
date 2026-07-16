use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn real_problem_inputs_match_current_c_invalid_autosched_boundary() {
    for (label, problem) in [
        ("unsat", "cnf(false_axiom, axiom, ($false)).\n"),
        ("sat", "cnf(unit_axiom, axiom, p(a)).\n"),
    ] {
        let path = write_problem(label, problem);
        let output = Command::new(env!("CARGO_BIN_EXE_e_stratpar"))
            .arg("--cpu-limit=10")
            .arg(&path)
            .env("PATH", path_with_built_eprover())
            .output()
            .unwrap();
        let stdout = String::from_utf8(output.stdout).unwrap();
        let stderr = String::from_utf8(output.stderr).unwrap();

        assert_eq!(output.status.code(), Some(11), "{stdout}\n{stderr}");
        assert!(stdout.is_empty(), "{stdout}");
        assert_eq!(
            stderr,
            concat!(
                "eprover: Option -t (--term-ordering) requires LPO, LPO4, KBO or KBO6 as an argument\n",
                "e_stratpar: Cannot read eprover PID line\n",
            )
        );

        std::fs::remove_file(path).unwrap();
    }
}

fn path_with_built_eprover() -> OsString {
    let eprover_dir = Path::new(env!("CARGO_BIN_EXE_eprover"))
        .parent()
        .unwrap()
        .to_path_buf();
    let mut paths = vec![eprover_dir];
    if let Some(path) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&path));
    }
    std::env::join_paths(paths).unwrap()
}

fn write_problem(label: &str, contents: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "e-rust-port-e-stratpar-{label}-{}-{nonce}.p",
        std::process::id()
    ));
    std::fs::write(&path, contents).unwrap();
    path
}
