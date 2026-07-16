use crate::basics::os_wrapper::get_usec_clock;
use std::fmt;
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};

// Allowed external shared-library boundary: C `strerror` is required for the
// exact `SysError` text emitted for saved C-runtime errno values. The raw
// pointer never leaves this module.
#[allow(unsafe_code)]
mod c_runtime {
    use std::ffi::{c_char, c_int, CStr};

    unsafe extern "C" {
        fn strerror(errnum: c_int) -> *mut c_char;
    }

    pub(super) fn error_message(error_code: i32) -> String {
        // SAFETY: strerror accepts every c_int errno value and returns either
        // null or a pointer to a nul-terminated C-runtime-owned string. The
        // result is checked and copied immediately.
        let message = unsafe { strerror(error_code) };
        if message.is_null() {
            return format!("Unknown error {error_code}");
        }
        // SAFETY: the non-null strerror result is guaranteed to be
        // nul-terminated and remains valid for this immediate owned copy.
        unsafe { CStr::from_ptr(message) }
            .to_string_lossy()
            .into_owned()
    }
}

static PROGRAM_NAME: OnceLock<Mutex<String>> = OnceLock::new();
static TMP_ERRNO: AtomicI32 = AtomicI32::new(0);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ErrorCode(u8);

impl ErrorCode {
    pub const NO_ERROR: Self = Self(0);
    pub const PROOF_FOUND: Self = Self(0);
    pub const SATISFIABLE: Self = Self(1);
    pub const OUT_OF_MEMORY: Self = Self(2);
    pub const SYNTAX_ERROR: Self = Self(3);
    pub const TYPE_ERROR: Self = Self(4);
    pub const USAGE_ERROR: Self = Self(5);
    pub const FILE_ERROR: Self = Self(6);
    pub const SYS_ERROR: Self = Self(7);
    pub const SYSTEM_ERROR: Self = Self(7);
    pub const CPU_LIMIT_ERROR: Self = Self(8);
    pub const RESOURCE_OUT: Self = Self(9);
    pub const INCOMPLETE_PROOFSTATE: Self = Self(10);
    pub const OTHER_ERROR: Self = Self(11);
    pub const INPUT_SEMANTIC_ERROR: Self = Self(12);
    pub const INTERFACE_ERROR: Self = Self(13);
    pub const PARENT_REQUEST: Self = Self(14);

    #[must_use]
    pub const fn exit_status(self) -> u8 {
        self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    code: ErrorCode,
    message: String,
}

impl Diagnostic {
    #[must_use]
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    #[must_use]
    pub const fn code(&self) -> ErrorCode {
        self.code
    }

    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    #[must_use]
    pub fn render_error(&self, program_name: &str) -> String {
        render_error_message(program_name, &self.message)
    }

    #[must_use]
    pub fn render_warning(&self, program_name: &str) -> String {
        render_warning_message(program_name, &self.message)
    }

    #[must_use]
    pub fn render_sys_error(&self, program_name: &str, error: &io::Error) -> String {
        render_sys_error_message(program_name, &self.message, error)
    }

    #[must_use]
    pub fn render_sys_warning(&self, program_name: &str, error: &io::Error) -> String {
        render_sys_warning_message(program_name, &self.message, error)
    }

    #[must_use]
    pub fn render_global_error(&self) -> String {
        self.render_error(&program_name())
    }

    #[must_use]
    pub fn render_global_warning(&self) -> String {
        self.render_warning(&program_name())
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for Diagnostic {}

fn program_name_cell() -> &'static Mutex<String> {
    PROGRAM_NAME.get_or_init(|| Mutex::new(String::from("Unknown program")))
}

fn lock_program_name() -> MutexGuard<'static, String> {
    program_name_cell()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

pub fn init_error(program_name: impl Into<String>) {
    *lock_program_name() = program_name.into();
}

#[must_use]
pub fn program_name() -> String {
    lock_program_name().clone()
}

#[must_use]
pub fn tmp_errno() -> i32 {
    TMP_ERRNO.load(Ordering::SeqCst)
}

pub fn set_tmp_errno(errno: i32) -> i32 {
    TMP_ERRNO.swap(errno, Ordering::SeqCst)
}

#[must_use]
pub fn c_runtime_error_message(error_code: i32) -> String {
    c_runtime::error_message(error_code)
}

#[must_use]
pub fn render_error_message(program_name: &str, message: &str) -> String {
    format!("{program_name}: {message}\n")
}

#[must_use]
pub fn render_warning_message(program_name: &str, message: &str) -> String {
    format!("{program_name}: Warning: {message}\n")
}

#[must_use]
pub fn render_sys_message(program_name: &str, message: &str, system_message: &str) -> String {
    format!("{program_name}: {message}\n{program_name}: {system_message}\n")
}

#[must_use]
pub fn render_sys_error_message(program_name: &str, message: &str, error: &io::Error) -> String {
    render_sys_message(program_name, message, &error.to_string())
}

#[must_use]
pub fn render_sys_warning_message(program_name: &str, message: &str, error: &io::Error) -> String {
    render_sys_message(
        program_name,
        &format!("Warning: {message}"),
        &error.to_string(),
    )
}

#[must_use]
pub fn render_tmp_errno_error_message(message: &str) -> String {
    let program_name = program_name();
    let system_message = c_runtime_error_message(tmp_errno());
    render_sys_message(&program_name, message, &system_message)
}

#[must_use]
pub fn render_tmp_errno_warning_message(message: &str) -> String {
    let program_name = program_name();
    let system_message = c_runtime_error_message(tmp_errno());
    render_sys_message(
        &program_name,
        &format!("Warning: {message}"),
        &system_message,
    )
}

pub fn write_error_message(
    output: &mut impl Write,
    program_name: &str,
    message: &str,
) -> io::Result<()> {
    output.write_all(render_error_message(program_name, message).as_bytes())
}

pub fn write_warning_message(
    output: &mut impl Write,
    program_name: &str,
    message: &str,
) -> io::Result<()> {
    output.write_all(render_warning_message(program_name, message).as_bytes())
}

pub fn write_sys_error_message(
    output: &mut impl Write,
    program_name: &str,
    message: &str,
    error: &io::Error,
) -> io::Result<()> {
    output.write_all(render_sys_error_message(program_name, message, error).as_bytes())
}

pub fn write_sys_warning_message(
    output: &mut impl Write,
    program_name: &str,
    message: &str,
    error: &io::Error,
) -> io::Result<()> {
    output.write_all(render_sys_warning_message(program_name, message, error).as_bytes())
}

#[must_use]
pub fn elog_file_name(process_id: u32) -> String {
    format!("elog{process_id}.log")
}

#[must_use]
pub fn render_elog_record(process_id: u32, cpu_time_seconds: f64, message: &str) -> String {
    format!("{process_id}: {cpu_time_seconds:4.9}: {message}")
}

pub fn write_elog_message(
    log_output: &mut impl Write,
    stderr: &mut impl Write,
    process_id: u32,
    cpu_time_seconds: f64,
    message: &str,
) -> io::Result<()> {
    log_output.write_all(render_elog_record(process_id, cpu_time_seconds, message).as_bytes())?;
    stderr.write_all(b"\n")
}

#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn current_elog_cpu_time_seconds() -> f64 {
    get_usec_clock() as f64 / 1_000_000.0
}

pub fn elog(message: &str) -> io::Result<PathBuf> {
    elog_in_dir(Path::new("."), message)
}

pub fn elog_in_dir(directory: impl AsRef<Path>, message: &str) -> io::Result<PathBuf> {
    let stderr = io::stderr();
    let mut stderr = stderr.lock();
    elog_in_dir_with_stderr(directory, message, &mut stderr)
}

pub fn elog_in_dir_with_stderr(
    directory: impl AsRef<Path>,
    message: &str,
    stderr: &mut impl Write,
) -> io::Result<PathBuf> {
    let process_id = std::process::id();
    let path = directory.as_ref().join(elog_file_name(process_id));
    let mut log_output = OpenOptions::new().append(true).create(true).open(&path)?;
    write_elog_message(
        &mut log_output,
        stderr,
        process_id,
        current_elog_cpu_time_seconds(),
        message,
    )?;
    Ok(path)
}

#[must_use]
pub fn test_letter_string(to_check: &str, options: &str) -> bool {
    to_check
        .bytes()
        .all(|candidate| options.bytes().any(|control| control == candidate))
}

pub fn check_option_letter_string(
    to_check: &str,
    options: &str,
    option: &str,
) -> Result<(), Diagnostic> {
    if test_letter_string(to_check, options) {
        Ok(())
    } else {
        Err(Diagnostic::new(
            ErrorCode::USAGE_ERROR,
            format!("Illegal argument to option {option}"),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        c_runtime_error_message, check_option_letter_string, elog_file_name,
        elog_in_dir_with_stderr, init_error, program_name, render_elog_record,
        render_error_message, render_sys_error_message, render_sys_warning_message,
        render_tmp_errno_error_message, render_tmp_errno_warning_message, render_warning_message,
        set_tmp_errno, test_letter_string, tmp_errno, write_elog_message, write_error_message,
        write_sys_error_message, write_sys_warning_message, write_warning_message, Diagnostic,
        ErrorCode,
    };
    use crate::test_support::global_state_lock;
    use std::io;

    #[test]
    fn error_code_names_match_c_exit_statuses() {
        assert_eq!(ErrorCode::NO_ERROR.exit_status(), 0);
        assert_eq!(ErrorCode::PROOF_FOUND.exit_status(), 0);
        assert_eq!(ErrorCode::SATISFIABLE.exit_status(), 1);
        assert_eq!(ErrorCode::SYS_ERROR.exit_status(), 7);
        assert_eq!(ErrorCode::SYSTEM_ERROR, ErrorCode::SYS_ERROR);
        assert_eq!(ErrorCode::PARENT_REQUEST.exit_status(), 14);
    }

    #[test]
    fn global_error_state_matches_progname_and_tmp_errno_defaults() {
        let _guard = global_state_lock();
        init_error("Unknown program");
        set_tmp_errno(0);

        assert_eq!(program_name(), "Unknown program");
        assert_eq!(set_tmp_errno(2), 0);
        assert_eq!(tmp_errno(), 2);

        init_error("eprover");
        let diagnostic = Diagnostic::new(ErrorCode::FILE_ERROR, "cannot open input");
        assert_eq!(
            diagnostic.render_global_error(),
            "eprover: cannot open input\n"
        );
        assert_eq!(
            diagnostic.render_global_warning(),
            "eprover: Warning: cannot open input\n"
        );

        init_error("Unknown program");
        set_tmp_errno(0);
    }

    #[test]
    fn diagnostics_render_c_shaped_error_warning_and_syserror_text() {
        let system_error = io::Error::from_raw_os_error(2);
        let diagnostic = Diagnostic::new(ErrorCode::FILE_ERROR, "cannot open input");

        assert_eq!(
            render_error_message("eprover", diagnostic.message()),
            "eprover: cannot open input\n"
        );
        assert_eq!(
            render_warning_message("eprover", diagnostic.message()),
            "eprover: Warning: cannot open input\n"
        );
        assert_eq!(
            diagnostic.render_sys_error("eprover", &system_error),
            format!("eprover: cannot open input\neprover: {system_error}\n")
        );
        assert_eq!(
            diagnostic.render_sys_warning("eprover", &system_error),
            format!("eprover: Warning: cannot open input\neprover: {system_error}\n")
        );
        assert_eq!(
            render_sys_error_message("eprover", "cannot open input", &system_error),
            format!("eprover: cannot open input\neprover: {system_error}\n")
        );
        assert_eq!(
            render_sys_warning_message("eprover", "cannot open input", &system_error),
            format!("eprover: Warning: cannot open input\neprover: {system_error}\n")
        );
    }

    #[test]
    fn tmp_errno_syserror_rendering_uses_current_global_state() {
        let _guard = global_state_lock();
        init_error("eprover");
        set_tmp_errno(2);

        let system_error = c_runtime_error_message(2);
        assert_eq!(
            render_tmp_errno_error_message("cannot open input"),
            format!("eprover: cannot open input\neprover: {system_error}\n")
        );
        assert_eq!(
            render_tmp_errno_warning_message("cannot open input"),
            format!("eprover: Warning: cannot open input\neprover: {system_error}\n")
        );

        init_error("Unknown program");
        set_tmp_errno(0);
    }

    #[test]
    fn write_helpers_emit_the_rendered_c_shapes() {
        let system_error = io::Error::from_raw_os_error(2);
        let mut output = Vec::new();

        write_error_message(&mut output, "eprover", "fatal").unwrap();
        write_warning_message(&mut output, "eprover", "warn").unwrap();
        write_sys_error_message(&mut output, "eprover", "sys", &system_error).unwrap();
        write_sys_warning_message(&mut output, "eprover", "syswarn", &system_error).unwrap();

        assert_eq!(
            String::from_utf8(output).unwrap(),
            format!(
                "eprover: fatal\neprover: Warning: warn\neprover: sys\neprover: {system_error}\neprover: Warning: syswarn\neprover: {system_error}\n"
            )
        );
    }

    #[test]
    fn elog_helpers_preserve_c_record_and_stderr_newline_split() {
        let mut log_output = Vec::new();
        let mut stderr = Vec::new();

        write_elog_message(&mut log_output, &mut stderr, 1234, 12.5, "trace point").unwrap();

        assert_eq!(elog_file_name(1234), "elog1234.log");
        assert_eq!(
            render_elog_record(1234, -1.0, "failed clock"),
            "1234: -1.000000000: failed clock"
        );
        assert_eq!(
            String::from_utf8(log_output).unwrap(),
            "1234: 12.500000000: trace point"
        );
        assert_eq!(String::from_utf8(stderr).unwrap(), "\n");
    }

    #[test]
    fn elog_in_dir_appends_to_pid_named_file() {
        let directory = std::env::temp_dir().join(format!(
            "e_rust_port_elog_test_{}_{}",
            std::process::id(),
            crate::basics::os_wrapper::get_usec_time()
        ));
        std::fs::create_dir(&directory).unwrap();
        let mut stderr = Vec::new();

        let path = elog_in_dir_with_stderr(&directory, "first", &mut stderr).unwrap();
        let second_path = elog_in_dir_with_stderr(&directory, "second", &mut stderr).unwrap();

        assert_eq!(path, second_path);
        assert_eq!(
            path.file_name().and_then(std::ffi::OsStr::to_str),
            Some(elog_file_name(std::process::id()).as_str())
        );
        let contents = std::fs::read_to_string(&path).unwrap();
        assert!(contents.starts_with(&format!("{}: ", std::process::id())));
        assert!(contents.contains(": first"));
        assert!(contents.contains(": second"));
        assert!(!contents.contains('\n'));
        assert_eq!(String::from_utf8(stderr).unwrap(), "\n\n");

        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(directory).unwrap();
    }

    #[test]
    fn letter_string_accepts_only_known_letters() {
        assert!(test_letter_string("abc", "cadb"));
        assert!(test_letter_string("", ""));
        assert!(!test_letter_string("abcx", "abc"));
    }

    #[test]
    fn check_letter_string_reports_usage_error() {
        let error = check_option_letter_string("az", "abc", "--letters").unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert_eq!(error.message(), "Illegal argument to option --letters");
    }
}
