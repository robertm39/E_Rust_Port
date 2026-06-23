use crate::inout::output::init_output;
use std::sync::{Mutex, MutexGuard, OnceLock};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct IoState {
    program_name: Option<String>,
    tptp_dir: Option<String>,
}

static IO_STATE: OnceLock<Mutex<IoState>> = OnceLock::new();

fn io_state() -> &'static Mutex<IoState> {
    IO_STATE.get_or_init(|| Mutex::new(IoState::default()))
}

fn lock_io_state() -> MutexGuard<'static, IoState> {
    match io_state().lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn tptp_env_dir() -> Option<String> {
    std::env::var_os("TPTP").map(|value| {
        let mut dir = value.to_string_lossy().into_owned();
        if !dir.is_empty() && !dir.ends_with('/') {
            dir.push('/');
        }
        dir
    })
}

pub fn init_io(program_name: &str) {
    init_output();
    let mut state = lock_io_state();
    state.program_name = Some(program_name.to_owned());
    if let Some(tptp_dir) = tptp_env_dir() {
        state.tptp_dir = Some(tptp_dir);
    }
}

pub fn exit_io() {
    lock_io_state().tptp_dir = None;
}

#[must_use]
pub fn program_name() -> Option<String> {
    lock_io_state().program_name.clone()
}

#[must_use]
pub fn tptp_dir() -> Option<String> {
    lock_io_state().tptp_dir.clone()
}

#[cfg(test)]
fn reset_io_for_tests() {
    *lock_io_state() = IoState::default();
}

#[cfg(test)]
mod tests {
    use super::{exit_io, init_io, program_name, reset_io_for_tests, tptp_dir};
    use crate::inout::output::{global_out_fd, init_output, open_global_out, STDOUT_FILENO_COMPAT};
    use std::ffi::OsString;
    use std::path::{Path, PathBuf};
    use std::sync::{Mutex, OnceLock};

    struct EnvGuard {
        previous: Option<OsString>,
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match &self.previous {
                Some(value) => std::env::set_var("TPTP", value),
                None => std::env::remove_var("TPTP"),
            }
        }
    }

    fn global_test_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
    }

    fn target_path(name: &str) -> PathBuf {
        std::env::current_dir()
            .unwrap()
            .join("target")
            .join(format!("initio-{name}-{}.tmp", std::process::id()))
    }

    fn set_tptp(value: Option<&str>) -> EnvGuard {
        let previous = std::env::var_os("TPTP");
        match value {
            Some(value) => std::env::set_var("TPTP", value),
            None => std::env::remove_var("TPTP"),
        }
        EnvGuard { previous }
    }

    fn remove_if_present(path: &Path) {
        _ = std::fs::remove_file(path);
    }

    #[test]
    fn init_io_sets_program_name_tptp_dir_and_output_target() {
        let _guard = global_test_lock();
        reset_io_for_tests();
        let _env = set_tptp(Some("Problems"));
        let output_path = target_path("out");
        remove_if_present(&output_path);
        open_global_out(Some(&output_path)).unwrap();

        init_io("eprover");

        assert_eq!(program_name().as_deref(), Some("eprover"));
        assert_eq!(tptp_dir().as_deref(), Some("Problems/"));
        assert_eq!(global_out_fd(), STDOUT_FILENO_COMPAT);
        remove_if_present(&output_path);
    }

    #[test]
    fn init_io_preserves_c_tptp_reinitialization_shape() {
        let _guard = global_test_lock();
        reset_io_for_tests();
        let env = set_tptp(Some("First/"));
        init_io("first");
        assert_eq!(tptp_dir().as_deref(), Some("First/"));
        drop(env);

        let _env = set_tptp(None);
        init_io("second");

        assert_eq!(program_name().as_deref(), Some("second"));
        assert_eq!(tptp_dir().as_deref(), Some("First/"));

        exit_io();
        assert_eq!(tptp_dir(), None);
        assert_eq!(program_name().as_deref(), Some("second"));
    }

    #[test]
    fn empty_tptp_is_stored_without_appending_a_slash() {
        let _guard = global_test_lock();
        reset_io_for_tests();
        let _env = set_tptp(Some(""));

        init_io("empty");

        assert_eq!(tptp_dir().as_deref(), Some(""));
        init_output();
    }
}
