use crate::basics::error::{Diagnostic, ErrorCode};
use crate::inout::output::{out_close, out_open};
use std::collections::BTreeSet;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

const TEMP_PREFIX: &str = "epr_";
const TEMP_ATTEMPTS: usize = 1024;

static TEMP_FILE_STORE: OnceLock<Mutex<BTreeSet<PathBuf>>> = OnceLock::new();
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_file_store() -> &'static Mutex<BTreeSet<PathBuf>> {
    TEMP_FILE_STORE.get_or_init(|| Mutex::new(BTreeSet::new()))
}

fn lock_temp_file_store() -> MutexGuard<'static, BTreeSet<PathBuf>> {
    match temp_file_store().lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn file_error(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(ErrorCode::FILE_ERROR, message)
}

fn system_error(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(ErrorCode::SYSTEM_ERROR, message)
}

fn tmpdir() -> PathBuf {
    std::env::var_os("TMPDIR").map_or_else(default_tmpdir, PathBuf::from)
}

#[cfg(windows)]
fn default_tmpdir() -> PathBuf {
    std::env::temp_dir()
}

#[cfg(not(windows))]
fn default_tmpdir() -> PathBuf {
    PathBuf::from("/tmp")
}

fn time_bits() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| {
            duration.as_secs().rotate_left(32) ^ u64::from(duration.subsec_nanos())
        })
}

fn six_char_suffix(mut value: u64) -> String {
    const DIGITS: &[u8; 36] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let mut bytes = [b'0'; 6];
    for slot in bytes.iter_mut().rev() {
        *slot = DIGITS[(value % 36) as usize];
        value /= 36;
    }
    bytes.into_iter().map(char::from).collect()
}

fn candidate_name(directory: &Path) -> PathBuf {
    let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let seed = time_bits()
        ^ u64::from(std::process::id()).rotate_left(17)
        ^ counter.wrapping_mul(0x9e37_79b9_7f4a_7c15);
    directory.join(format!("{TEMP_PREFIX}{}", six_char_suffix(seed)))
}

#[must_use]
pub fn temp_file_cleanup() -> Vec<Diagnostic> {
    let registered = {
        let mut store = lock_temp_file_store();
        let registered = store.iter().cloned().collect::<Vec<_>>();
        store.clear();
        registered
    };

    let mut warnings = Vec::new();
    for file in registered {
        if let Err(error) = fs::remove_file(&file) {
            warnings.push(system_error(format!(
                "Could not remove temporary file {}: {error}",
                file.display()
            )));
        }
    }
    warnings
}

#[must_use]
pub fn temp_file_register(name: &Path) -> bool {
    lock_temp_file_store().insert(name.to_path_buf())
}

pub fn temp_file_name() -> Result<PathBuf, Diagnostic> {
    let directory = tmpdir();
    let name = platform_tempfile::create_temp_file(&directory)?;
    let _ = temp_file_register(&name);
    Ok(name)
}

pub fn temp_file_create(source: &mut impl Read) -> Result<PathBuf, Diagnostic> {
    let name = temp_file_name()?;
    let mut output = out_open(Some(&name))?;
    io::copy(source, &mut output).map_err(|error| {
        file_error(format!(
            "Could not write temporary file {}: {error}",
            name.display()
        ))
    })?;
    out_close(output)?;
    Ok(name)
}

pub fn temp_file_remove(name: &Path) -> Result<bool, Diagnostic> {
    fs::remove_file(name).map_err(|error| {
        system_error(format!(
            "Could not remove temporary file {}: {error}",
            name.display()
        ))
    })?;
    Ok(lock_temp_file_store().remove(name))
}

/// Removes a temporary file, asserting the C `TempFileRemove` registry precondition.
///
/// # Panics
///
/// Panics after a successful unlink if `name` was not registered as a
/// temporary file, matching the C assertion after `StrTreeDeleteEntry`.
pub fn temp_file_remove_asserting(name: &Path) -> Result<(), Diagnostic> {
    assert!(
        temp_file_remove(name)?,
        "TempFileRemove requires a registered temporary file"
    );
    Ok(())
}

#[cfg(test)]
fn registered_temp_file_count() -> usize {
    lock_temp_file_store().len()
}

#[cfg(test)]
fn reset_temp_files_for_tests() {
    let registered = {
        let mut store = lock_temp_file_store();
        let registered = store.iter().cloned().collect::<Vec<_>>();
        store.clear();
        registered
    };
    for path in registered {
        _ = fs::remove_file(path);
    }
}

#[cfg(test)]
pub(crate) fn temp_file_test_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    match LOCK.get_or_init(|| Mutex::new(())).lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        registered_temp_file_count, reset_temp_files_for_tests, temp_file_cleanup,
        temp_file_create, temp_file_name, temp_file_register, temp_file_remove,
        temp_file_remove_asserting, TEMP_PREFIX,
    };
    use crate::basics::error::ErrorCode;
    use std::ffi::OsString;
    use std::io::Cursor;
    use std::path::{Path, PathBuf};

    struct TmpDirGuard {
        previous: Option<OsString>,
    }

    impl Drop for TmpDirGuard {
        fn drop(&mut self) {
            match &self.previous {
                Some(value) => std::env::set_var("TMPDIR", value),
                None => std::env::remove_var("TMPDIR"),
            }
        }
    }

    fn global_test_lock() -> std::sync::MutexGuard<'static, ()> {
        super::temp_file_test_lock()
    }

    fn target_dir() -> PathBuf {
        std::env::current_dir().unwrap().join("target")
    }

    fn set_tmpdir(path: &Path) -> TmpDirGuard {
        let previous = std::env::var_os("TMPDIR");
        std::env::set_var("TMPDIR", path);
        TmpDirGuard { previous }
    }

    fn cleanup_path(path: &Path) {
        _ = std::fs::remove_file(path);
    }

    #[test]
    fn temp_file_name_creates_and_registers_file_under_tmpdir() {
        let _guard = global_test_lock();
        reset_temp_files_for_tests();
        let _tmpdir = set_tmpdir(&target_dir());

        let name = temp_file_name().unwrap();
        assert!(name.exists());
        assert_eq!(name.parent(), Some(target_dir().as_path()));
        let suffix = name
            .file_name()
            .and_then(|file_name| file_name.to_str())
            .and_then(|file_name| file_name.strip_prefix(TEMP_PREFIX))
            .unwrap();
        assert_eq!(suffix.len(), 6);
        assert!(suffix
            .bytes()
            .all(|byte| byte.is_ascii_digit() || byte.is_ascii_lowercase()));
        assert_eq!(registered_temp_file_count(), 1);

        assert!(temp_file_remove(&name).unwrap());
        assert!(!name.exists());
        assert_eq!(registered_temp_file_count(), 0);
    }

    #[test]
    fn temp_file_create_copies_source_contents() {
        let _guard = global_test_lock();
        reset_temp_files_for_tests();
        let _tmpdir = set_tmpdir(&target_dir());

        let mut source = Cursor::new(b"temporary payload".to_vec());
        let name = temp_file_create(&mut source).unwrap();
        assert_eq!(std::fs::read(&name).unwrap(), b"temporary payload");
        assert_eq!(registered_temp_file_count(), 1);

        assert!(temp_file_remove(&name).unwrap());
    }

    #[test]
    fn cleanup_removes_registered_files_and_reports_missing_files() {
        let _guard = global_test_lock();
        reset_temp_files_for_tests();
        let existing = target_dir().join(format!("{TEMP_PREFIX}cleanup-existing.tmp"));
        let missing = target_dir().join(format!("{TEMP_PREFIX}cleanup-missing.tmp"));
        cleanup_path(&existing);
        cleanup_path(&missing);
        std::fs::write(&existing, b"x").unwrap();
        assert!(temp_file_register(&existing));
        assert!(temp_file_register(&missing));

        let warnings = temp_file_cleanup();

        assert!(!existing.exists());
        assert_eq!(registered_temp_file_count(), 0);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].code(), ErrorCode::SYSTEM_ERROR);
        assert!(warnings[0]
            .message()
            .contains("Could not remove temporary file"));
    }

    #[test]
    fn duplicate_registration_is_reported_without_duplicating_store() {
        let _guard = global_test_lock();
        reset_temp_files_for_tests();
        let path = target_dir().join(format!("{TEMP_PREFIX}duplicate.tmp"));
        cleanup_path(&path);
        std::fs::write(&path, b"x").unwrap();

        assert!(temp_file_register(&path));
        assert!(!temp_file_register(&path));
        assert_eq!(registered_temp_file_count(), 1);
        assert!(temp_file_remove(&path).unwrap());
    }

    #[test]
    fn temp_file_remove_asserting_matches_registered_c_path() {
        let _guard = global_test_lock();
        reset_temp_files_for_tests();
        let path = target_dir().join(format!("{TEMP_PREFIX}asserting-remove.tmp"));
        cleanup_path(&path);
        std::fs::write(&path, b"x").unwrap();
        assert!(temp_file_register(&path));

        temp_file_remove_asserting(&path).unwrap();

        assert!(!path.exists());
        assert_eq!(registered_temp_file_count(), 0);
    }

    #[test]
    #[should_panic(expected = "TempFileRemove requires a registered temporary file")]
    fn temp_file_remove_asserting_matches_c_registry_precondition() {
        let _guard = global_test_lock();
        reset_temp_files_for_tests();
        let path = target_dir().join(format!("{TEMP_PREFIX}asserting-unregistered.tmp"));
        cleanup_path(&path);
        std::fs::write(&path, b"x").unwrap();

        let _ = temp_file_remove_asserting(&path);
    }

    #[test]
    fn temp_file_remove_reports_unlink_failures() {
        let _guard = global_test_lock();
        reset_temp_files_for_tests();
        let path = target_dir().join(format!("{TEMP_PREFIX}missing-remove.tmp"));
        cleanup_path(&path);

        let error = temp_file_remove(&path).unwrap_err();
        assert_eq!(error.code(), ErrorCode::SYSTEM_ERROR);
    }
}

mod platform_tempfile {
    use super::{candidate_name, file_error, TEMP_ATTEMPTS};
    use crate::basics::error::Diagnostic;
    use std::fs::{File, OpenOptions};
    use std::io;
    use std::path::{Path, PathBuf};

    fn open_candidate(candidate: &Path) -> io::Result<File> {
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        options.open(candidate)
    }

    pub(super) fn create_temp_file(directory: &Path) -> Result<PathBuf, Diagnostic> {
        for _ in 0..TEMP_ATTEMPTS {
            let candidate = candidate_name(directory);
            match open_candidate(&candidate) {
                Ok(file) => {
                    drop(file);
                    return Ok(candidate);
                }
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
                Err(error) => {
                    return Err(file_error(format!(
                        "Could not create valid temporary file name {} (check $TMPDIR): {error}",
                        candidate.display()
                    )));
                }
            }
        }

        Err(file_error(format!(
            "Could not create valid temporary file name in {} (check $TMPDIR)",
            directory.display()
        )))
    }
}
