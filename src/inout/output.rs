use crate::basics::error::{Diagnostic, ErrorCode};
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;
use std::sync::{Mutex, MutexGuard, OnceLock};

pub const STDOUT_FILENO_COMPAT: i32 = 1;
pub const UNKNOWN_FILENO_COMPAT: i32 = -1;

#[derive(Debug)]
pub enum OutputDestination {
    Stdout,
    File(File),
}

impl Write for OutputDestination {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        match self {
            Self::Stdout => io::stdout().write(buffer),
            Self::File(file) => file.write(buffer),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            Self::Stdout => io::stdout().flush(),
            Self::File(file) => file.flush(),
        }
    }
}

#[derive(Debug)]
struct OutputState {
    output_level: i64,
    target: OutputDestination,
    global_out_fd: i32,
}

impl Default for OutputState {
    fn default() -> Self {
        Self {
            output_level: 1,
            target: OutputDestination::Stdout,
            global_out_fd: STDOUT_FILENO_COMPAT,
        }
    }
}

static OUTPUT_STATE: OnceLock<Mutex<OutputState>> = OnceLock::new();

fn output_state() -> &'static Mutex<OutputState> {
    OUTPUT_STATE.get_or_init(|| Mutex::new(OutputState::default()))
}

fn lock_output_state() -> MutexGuard<'static, OutputState> {
    match output_state().lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn file_fd(file: &File) -> i32 {
    #[cfg(unix)]
    {
        use std::os::fd::AsRawFd;
        file.as_raw_fd()
    }
    #[cfg(not(unix))]
    {
        let _ = file;
        UNKNOWN_FILENO_COMPAT
    }
}

fn diagnostic_from_io(prefix: &str, error: &io::Error) -> Diagnostic {
    Diagnostic::new(ErrorCode::FILE_ERROR, format!("{prefix}: {error}"))
}

#[must_use]
pub fn output_level() -> i64 {
    lock_output_state().output_level
}

#[must_use]
pub fn set_output_level(level: i64) -> i64 {
    let mut state = lock_output_state();
    let old = state.output_level;
    state.output_level = level;
    old
}

pub fn init_output() {
    let mut state = lock_output_state();
    state.target = OutputDestination::Stdout;
    state.global_out_fd = STDOUT_FILENO_COMPAT;
}

#[must_use]
pub fn global_out_fd() -> i32 {
    lock_output_state().global_out_fd
}

pub fn out_open(name: Option<&Path>) -> Result<OutputDestination, Diagnostic> {
    if name.is_none() || name == Some(Path::new("-")) {
        return Ok(OutputDestination::Stdout);
    }
    let Some(path) = name else {
        return Ok(OutputDestination::Stdout);
    };
    File::create(path)
        .map(OutputDestination::File)
        .map_err(|error| {
            Diagnostic::new(
                ErrorCode::FILE_ERROR,
                format!("Cannot open file {}: {error}", path.display()),
            )
        })
}

pub fn out_close(mut output: OutputDestination) -> Result<(), Diagnostic> {
    output
        .flush()
        .map_err(|error| diagnostic_from_io("Error while closing file", &error))
}

pub fn open_global_out(name: Option<&Path>) -> Result<(), Diagnostic> {
    let output = out_open(name)?;
    let fd = match &output {
        OutputDestination::Stdout => STDOUT_FILENO_COMPAT,
        OutputDestination::File(file) => file_fd(file),
    };
    let mut state = lock_output_state();
    state.target = output;
    state.global_out_fd = fd;
    Ok(())
}

pub fn close_global_out() -> Result<(), Diagnostic> {
    let mut state = lock_output_state();
    state
        .target
        .flush()
        .map_err(|error| diagnostic_from_io("Error while closing file", &error))?;
    state.target = OutputDestination::Stdout;
    state.global_out_fd = STDOUT_FILENO_COMPAT;
    Ok(())
}

pub fn outprint_to(
    output: &mut impl Write,
    output_level: i64,
    level: i64,
    message: &str,
) -> io::Result<bool> {
    if level <= output_level {
        output.write_all(message.as_bytes())?;
        Ok(true)
    } else {
        Ok(false)
    }
}

pub fn outprint_global(level: i64, message: &str) -> Result<bool, Diagnostic> {
    let mut state = lock_output_state();
    if level > state.output_level {
        return Ok(false);
    }
    state
        .target
        .write_all(message.as_bytes())
        .map_err(|error| diagnostic_from_io("Error writing output", &error))?;
    Ok(true)
}

#[must_use]
pub fn dashed_statuses(stat1: Option<&str>, stat2: Option<&str>, fallback: &str) -> String {
    match (stat1, stat2) {
        (Some(left), Some(right)) => format!("{left}-{right}"),
        (Some(left), None) => left.to_owned(),
        (None, Some(right)) => right.to_owned(),
        (None, None) => fallback.to_owned(),
    }
}

pub fn print_dashed_statuses(
    output: &mut impl Write,
    stat1: Option<&str>,
    stat2: Option<&str>,
    fallback: &str,
) -> io::Result<()> {
    output.write_all(dashed_statuses(stat1, stat2, fallback).as_bytes())
}

#[cfg(test)]
fn reset_output_for_tests() {
    let mut state = lock_output_state();
    *state = OutputState::default();
}

#[cfg(test)]
mod tests {
    use super::{
        close_global_out, dashed_statuses, global_out_fd, init_output, open_global_out, out_close,
        out_open, outprint_global, outprint_to, output_level, print_dashed_statuses,
        reset_output_for_tests, set_output_level, OutputDestination, STDOUT_FILENO_COMPAT,
    };
    use crate::basics::error::ErrorCode;
    use std::io::Write;
    use std::sync::{Mutex, OnceLock};

    fn global_test_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
    }

    fn temp_path(name: &str) -> std::path::PathBuf {
        std::env::current_dir()
            .unwrap()
            .join("target")
            .join(format!("output-{name}-{}.txt", std::process::id()))
    }

    #[test]
    fn output_level_defaults_to_one_and_gates_outprint() {
        let _guard = global_test_lock();
        reset_output_for_tests();

        assert_eq!(output_level(), 1);
        assert_eq!(set_output_level(2), 1);

        let mut output = Vec::new();
        assert!(outprint_to(&mut output, output_level(), 2, "visible").unwrap());
        assert!(!outprint_to(&mut output, output_level(), 3, "hidden").unwrap());
        assert_eq!(output, b"visible");
    }

    #[test]
    fn init_output_sets_stdout_compatibility_fd() {
        let _guard = global_test_lock();
        reset_output_for_tests();
        open_global_out(Some(&temp_path("init"))).unwrap();

        init_output();

        assert_eq!(global_out_fd(), STDOUT_FILENO_COMPAT);
    }

    #[test]
    fn out_open_writes_files_and_dash_means_stdout() {
        let _guard = global_test_lock();
        reset_output_for_tests();
        let path = temp_path("file");

        let mut output = out_open(Some(&path)).unwrap();
        output.write_all(b"abc").unwrap();
        out_close(output).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"abc");
        std::fs::remove_file(&path).unwrap();

        let dash = out_open(Some(std::path::Path::new("-"))).unwrap();
        assert!(matches!(dash, OutputDestination::Stdout));
    }

    #[test]
    fn out_open_reports_file_errors() {
        let _guard = global_test_lock();
        reset_output_for_tests();
        let error = out_open(Some(std::path::Path::new("target"))).unwrap_err();
        assert_eq!(error.code(), ErrorCode::FILE_ERROR);
    }

    #[test]
    fn global_output_writes_to_selected_file_and_resets_on_close() {
        let _guard = global_test_lock();
        reset_output_for_tests();
        let path = temp_path("global");

        open_global_out(Some(&path)).unwrap();
        assert!(outprint_global(1, "alpha").unwrap());
        assert!(!outprint_global(2, "hidden").unwrap());
        close_global_out().unwrap();

        assert_eq!(std::fs::read(&path).unwrap(), b"alpha");
        std::fs::remove_file(&path).unwrap();
        assert_eq!(global_out_fd(), STDOUT_FILENO_COMPAT);
    }

    #[test]
    fn dashed_status_formatting_matches_c_cases() {
        assert_eq!(dashed_statuses(Some("A"), Some("B"), "F"), "A-B");
        assert_eq!(dashed_statuses(Some("A"), None, "F"), "A");
        assert_eq!(dashed_statuses(None, Some("B"), "F"), "B");
        assert_eq!(dashed_statuses(None, None, "F"), "F");

        let mut output = Vec::new();
        print_dashed_statuses(&mut output, Some("Theorem"), Some("FOF"), "Unknown").unwrap();
        assert_eq!(output, b"Theorem-FOF");
    }
}
