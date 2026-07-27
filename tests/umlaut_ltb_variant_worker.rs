use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static TEMP_DIR_SERIAL: AtomicU64 = AtomicU64::new(0);

struct TempDirGuard {
    path: PathBuf,
}

impl TempDirGuard {
    fn new(name: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should follow the Unix epoch")
            .as_nanos();
        let serial = TEMP_DIR_SERIAL.fetch_add(1, Ordering::Relaxed);
        let path = std::env::current_dir()
            .expect("test should have a current directory")
            .join("target")
            .join(format!("{name}-{}-{nonce}-{serial}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("temporary test directory should be creatable");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDirGuard {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn run_real_hidden_variant_child(worker: usize) {
    let dir = TempDirGuard::new(&format!("umlaut-ltb-variant-worker-{worker}"));
    fs::create_dir_all(dir.path().join("Problems")).unwrap();
    fs::create_dir_all(dir.path().join("Results")).unwrap();
    fs::copy(
        env!("CARGO_BIN_EXE_umlaut"),
        dir.path()
            .join(format!("umlaut{}", std::env::consts::EXE_SUFFIX)),
    )
    .unwrap();
    fs::write(
        dir.path().join("batch.spec"),
        "division.category LTB.SAT\n\
         output.required Proof\n\
         limit.time.problem.wc 15\n\
         limit.time.overall.wc 60\n\
         Problems/prob_*ignored.p Results/prob.out\n",
    )
    .unwrap();
    for variant in ["+1", "_1"] {
        fs::write(
            dir.path()
                .join("Problems")
                .join(format!("prob_{variant}.p")),
            "cnf(goal_clause, axiom, $false).\n",
        )
        .unwrap();
    }

    let result = Command::new(env!("CARGO_BIN_EXE_umlaut-ltb-runner"))
        .current_dir(dir.path())
        .args(["--variants28", "batch.spec"])
        .output()
        .unwrap();

    let stderr = String::from_utf8(result.stderr).unwrap();
    let stdout = String::from_utf8(result.stdout).unwrap();
    assert!(
        result.status.success(),
        "worker {worker}, status: {}\nstdout:\n{stdout}\nstderr:\n{stderr}",
        result.status
    );
    assert!(
        stderr.lines().all(|line| line
            == "umlaut: Output stream to be closed reports error (probably broken pipe, file system full or quota exceeded)"),
        "worker {worker}\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(stdout.contains("% Initial: 1 abstract problems, 2 variants, 2 concrete problems\n"));
    assert!(stdout.contains("% Round 0, working on variant +1,"));
    assert!(stdout.contains("% Starting Umlaut LTB wrapper with 1000000s (1) cores\n"));
    assert!(stdout.contains("% Umlaut LTB wrapper with pid "));
    assert!(stdout.contains("% SZS status Unsatisfiable for Problems/prob_+1.p\n"));
    assert!(stdout.contains("% SZS status Ended for Problems/prob_+1.p\n\n"));
    assert!(stdout.contains("% Round 1, working on variant _1,"));
    assert!(stdout.contains("% Abstract problem Problems/prob_*ignored.p already solved\n"));
    assert!(stdout.ends_with("% =============== Variant batch done ===========\n\n"));
    assert!(
        stdout.find("% Starting Umlaut LTB wrapper").unwrap()
            < stdout
                .find("% SZS status Unsatisfiable for Problems/prob_+1.p")
                .unwrap()
    );
    assert!(
        stdout
            .find("% SZS status Unsatisfiable for Problems/prob_+1.p")
            .unwrap()
            < stdout
                .find("% SZS status Ended for Problems/prob_+1.p")
                .unwrap()
    );

    let destination = fs::read_to_string(dir.path().join("Results").join("prob.out")).unwrap();
    assert!(destination.starts_with("% SZS status Unsatisfiable for Problems/prob_+1.p\n"));
    assert!(destination.contains("% SZS status Unsatisfiable\n"));
}

#[test]
fn public_variant_mode_runs_real_hidden_child_and_replays_its_result() {
    std::thread::scope(|scope| {
        for worker in 0..2 {
            scope.spawn(move || run_real_hidden_variant_child(worker));
        }
    });
}
