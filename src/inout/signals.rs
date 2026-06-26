use crate::basics::error::{Diagnostic, ErrorCode};
use crate::inout::tempfile::temp_file_cleanup;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};

pub const RLIM_INFINITY_COMPAT: u64 = u64::MAX;
pub const SIGINT_COMPAT: i32 = 2;
pub const SIGTERM_COMPAT: i32 = 15;
pub const SIGXCPU_COMPAT: i32 = 24;

static SCHEDULE_TIME_LIMIT: AtomicU64 = AtomicU64::new(0);
static SYSTEM_TIME_LIMIT: AtomicU64 = AtomicU64::new(RLIM_INFINITY_COMPAT);
static SOFT_TIME_LIMIT: AtomicU64 = AtomicU64::new(RLIM_INFINITY_COMPAT);
static HARD_TIME_LIMIT: AtomicU64 = AtomicU64::new(RLIM_INFINITY_COMPAT);
static TIME_IS_UP: AtomicBool = AtomicBool::new(false);
static TIME_LIMIT_IS_SOFT: AtomicBool = AtomicBool::new(false);
static SIG_TERM_CAUGHT: AtomicUsize = AtomicUsize::new(0);
static FATAL_ERROR_IN_PROGRESS: AtomicBool = AtomicBool::new(false);
static SILENT_TIME_OUT: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SignalOutcome {
    HandlerInstalled {
        signal: i32,
    },
    SoftTimeLimitReached {
        next_limit: u64,
    },
    CpuLimitExceeded {
        silent: bool,
        diagnostic: Option<Diagnostic>,
    },
    Terminate {
        signal: i32,
        repeated: bool,
        cleanup_warnings: Vec<Diagnostic>,
    },
    UnexpectedSignal {
        signal: i32,
    },
}

#[must_use]
pub fn schedule_time_limit() -> u64 {
    SCHEDULE_TIME_LIMIT.load(Ordering::SeqCst)
}

#[must_use]
pub fn set_schedule_time_limit(limit: u64) -> u64 {
    SCHEDULE_TIME_LIMIT.swap(limit, Ordering::SeqCst)
}

#[must_use]
pub fn system_time_limit() -> u64 {
    SYSTEM_TIME_LIMIT.load(Ordering::SeqCst)
}

#[must_use]
pub fn set_system_time_limit(limit: u64) -> u64 {
    SYSTEM_TIME_LIMIT.swap(limit, Ordering::SeqCst)
}

#[must_use]
pub fn soft_time_limit() -> u64 {
    SOFT_TIME_LIMIT.load(Ordering::SeqCst)
}

#[must_use]
pub fn set_soft_time_limit(limit: u64) -> u64 {
    SOFT_TIME_LIMIT.swap(limit, Ordering::SeqCst)
}

#[must_use]
pub fn hard_time_limit() -> u64 {
    HARD_TIME_LIMIT.load(Ordering::SeqCst)
}

#[must_use]
pub fn set_hard_time_limit(limit: u64) -> u64 {
    HARD_TIME_LIMIT.swap(limit, Ordering::SeqCst)
}

#[must_use]
pub fn time_is_up() -> bool {
    TIME_IS_UP.load(Ordering::SeqCst)
}

#[must_use]
pub fn set_time_is_up(value: bool) -> bool {
    TIME_IS_UP.swap(value, Ordering::SeqCst)
}

#[must_use]
pub fn time_limit_is_soft() -> bool {
    TIME_LIMIT_IS_SOFT.load(Ordering::SeqCst)
}

#[must_use]
pub fn set_time_limit_is_soft(value: bool) -> bool {
    TIME_LIMIT_IS_SOFT.swap(value, Ordering::SeqCst)
}

#[must_use]
pub fn sig_term_caught() -> usize {
    SIG_TERM_CAUGHT.load(Ordering::SeqCst)
}

#[must_use]
pub fn silent_time_out() -> bool {
    SILENT_TIME_OUT.load(Ordering::SeqCst)
}

#[must_use]
pub fn set_silent_time_out(value: bool) -> bool {
    SILENT_TIME_OUT.swap(value, Ordering::SeqCst)
}

#[must_use]
pub fn e_signal_setup(signal: i32) -> SignalOutcome {
    SYSTEM_TIME_LIMIT.store(RLIM_INFINITY_COMPAT, Ordering::SeqCst);
    SignalOutcome::HandlerInstalled { signal }
}

#[must_use]
pub fn e_signal_handler(signal: i32) -> SignalOutcome {
    match signal {
        SIGXCPU_COMPAT => handle_cpu_limit(),
        SIGTERM_COMPAT | SIGINT_COMPAT => handle_termination(signal),
        _ => SignalOutcome::UnexpectedSignal { signal },
    }
}

#[must_use]
pub fn e_sig_term_sched_handler(signal: i32) -> bool {
    if signal == SIGTERM_COMPAT {
        SIG_TERM_CAUGHT.fetch_add(1, Ordering::SeqCst);
        true
    } else {
        false
    }
}

fn handle_cpu_limit() -> SignalOutcome {
    if TIME_LIMIT_IS_SOFT.swap(false, Ordering::SeqCst) {
        TIME_IS_UP.store(true, Ordering::SeqCst);
        let next_limit = HARD_TIME_LIMIT
            .load(Ordering::SeqCst)
            .min(SYSTEM_TIME_LIMIT.load(Ordering::SeqCst));
        let _ = e_signal_setup(SIGXCPU_COMPAT);
        return SignalOutcome::SoftTimeLimitReached { next_limit };
    }

    if SILENT_TIME_OUT.load(Ordering::SeqCst) {
        SignalOutcome::CpuLimitExceeded {
            silent: true,
            diagnostic: None,
        }
    } else {
        SignalOutcome::CpuLimitExceeded {
            silent: false,
            diagnostic: Some(Diagnostic::new(
                ErrorCode::CPU_LIMIT_ERROR,
                "CPU time limit exceeded, terminating",
            )),
        }
    }
}

fn handle_termination(signal: i32) -> SignalOutcome {
    if FATAL_ERROR_IN_PROGRESS.swap(true, Ordering::SeqCst) {
        return SignalOutcome::Terminate {
            signal,
            repeated: true,
            cleanup_warnings: Vec::new(),
        };
    }

    SignalOutcome::Terminate {
        signal,
        repeated: false,
        cleanup_warnings: temp_file_cleanup(),
    }
}

#[cfg(test)]
fn reset_signal_state_for_tests() {
    SCHEDULE_TIME_LIMIT.store(0, Ordering::SeqCst);
    SYSTEM_TIME_LIMIT.store(RLIM_INFINITY_COMPAT, Ordering::SeqCst);
    SOFT_TIME_LIMIT.store(RLIM_INFINITY_COMPAT, Ordering::SeqCst);
    HARD_TIME_LIMIT.store(RLIM_INFINITY_COMPAT, Ordering::SeqCst);
    TIME_IS_UP.store(false, Ordering::SeqCst);
    TIME_LIMIT_IS_SOFT.store(false, Ordering::SeqCst);
    SIG_TERM_CAUGHT.store(0, Ordering::SeqCst);
    FATAL_ERROR_IN_PROGRESS.store(false, Ordering::SeqCst);
    SILENT_TIME_OUT.store(false, Ordering::SeqCst);
}

#[cfg(test)]
mod tests {
    use super::{
        e_sig_term_sched_handler, e_signal_handler, e_signal_setup, hard_time_limit,
        reset_signal_state_for_tests, schedule_time_limit, set_hard_time_limit,
        set_schedule_time_limit, set_silent_time_out, set_soft_time_limit, set_system_time_limit,
        set_time_is_up, set_time_limit_is_soft, sig_term_caught, silent_time_out, soft_time_limit,
        system_time_limit, time_is_up, time_limit_is_soft, SignalOutcome, RLIM_INFINITY_COMPAT,
        SIGINT_COMPAT, SIGTERM_COMPAT, SIGXCPU_COMPAT,
    };
    use crate::basics::error::ErrorCode;
    use crate::inout::tempfile::{temp_file_register, temp_file_test_lock};
    use crate::test_support::global_state_lock;
    use std::fs::File;

    #[test]
    fn default_globals_match_c_initializers() {
        let _guard = global_state_lock();
        reset_signal_state_for_tests();

        assert_eq!(schedule_time_limit(), 0);
        assert_eq!(system_time_limit(), RLIM_INFINITY_COMPAT);
        assert_eq!(soft_time_limit(), RLIM_INFINITY_COMPAT);
        assert_eq!(hard_time_limit(), RLIM_INFINITY_COMPAT);
        assert!(!time_is_up());
        assert!(!time_limit_is_soft());
        assert_eq!(sig_term_caught(), 0);
        assert!(!silent_time_out());
    }

    #[test]
    fn setters_update_global_signal_state() {
        let _guard = global_state_lock();
        reset_signal_state_for_tests();

        assert_eq!(set_schedule_time_limit(10), 0);
        assert_eq!(set_soft_time_limit(20), RLIM_INFINITY_COMPAT);
        assert_eq!(set_hard_time_limit(30), RLIM_INFINITY_COMPAT);
        assert_eq!(set_system_time_limit(40), RLIM_INFINITY_COMPAT);
        assert!(!set_time_is_up(true));
        assert!(!set_time_limit_is_soft(true));
        assert!(!set_silent_time_out(true));

        assert_eq!(schedule_time_limit(), 10);
        assert_eq!(soft_time_limit(), 20);
        assert_eq!(hard_time_limit(), 30);
        assert_eq!(system_time_limit(), 40);
        assert!(time_is_up());
        assert!(time_limit_is_soft());
        assert!(silent_time_out());
    }

    #[test]
    fn setup_records_system_limit_shape_without_installing_raw_handler() {
        let _guard = global_state_lock();
        reset_signal_state_for_tests();
        let _ = set_system_time_limit(123);

        assert_eq!(
            e_signal_setup(SIGXCPU_COMPAT),
            SignalOutcome::HandlerInstalled {
                signal: SIGXCPU_COMPAT
            }
        );
        assert_eq!(system_time_limit(), RLIM_INFINITY_COMPAT);
    }

    #[test]
    fn soft_cpu_signal_marks_time_up_and_computes_next_limit() {
        let _guard = global_state_lock();
        reset_signal_state_for_tests();
        let _ = set_time_limit_is_soft(true);
        let _ = set_hard_time_limit(75);
        let _ = set_system_time_limit(50);

        assert_eq!(
            e_signal_handler(SIGXCPU_COMPAT),
            SignalOutcome::SoftTimeLimitReached { next_limit: 50 }
        );
        assert!(time_is_up());
        assert!(!time_limit_is_soft());
        assert_eq!(system_time_limit(), RLIM_INFINITY_COMPAT);
    }

    #[test]
    fn hard_cpu_signal_reports_silent_or_diagnostic_timeout() {
        let _guard = global_state_lock();
        reset_signal_state_for_tests();
        let _ = set_silent_time_out(true);

        assert_eq!(
            e_signal_handler(SIGXCPU_COMPAT),
            SignalOutcome::CpuLimitExceeded {
                silent: true,
                diagnostic: None
            }
        );

        reset_signal_state_for_tests();
        let SignalOutcome::CpuLimitExceeded {
            silent,
            diagnostic: Some(diagnostic),
        } = e_signal_handler(SIGXCPU_COMPAT)
        else {
            panic!("expected non-silent CPU limit outcome");
        };
        assert!(!silent);
        assert_eq!(diagnostic.code(), ErrorCode::CPU_LIMIT_ERROR);
        assert_eq!(diagnostic.message(), "CPU time limit exceeded, terminating");
    }

    #[test]
    fn termination_signal_cleans_temp_files_once() {
        let _signal_guard = global_state_lock();
        let _temp_guard = temp_file_test_lock();
        reset_signal_state_for_tests();
        let path = std::env::current_dir()
            .unwrap()
            .join("target")
            .join(format!("signal-temp-{}.txt", std::process::id()));
        let _file = File::create(&path).unwrap();
        assert!(temp_file_register(&path));

        assert_eq!(
            e_signal_handler(SIGTERM_COMPAT),
            SignalOutcome::Terminate {
                signal: SIGTERM_COMPAT,
                repeated: false,
                cleanup_warnings: Vec::new()
            }
        );
        assert!(!path.exists());

        assert_eq!(
            e_signal_handler(SIGINT_COMPAT),
            SignalOutcome::Terminate {
                signal: SIGINT_COMPAT,
                repeated: true,
                cleanup_warnings: Vec::new()
            }
        );
    }

    #[test]
    fn scheduler_sigterm_handler_counts_only_sigterm() {
        let _guard = global_state_lock();
        reset_signal_state_for_tests();

        assert!(!e_sig_term_sched_handler(SIGINT_COMPAT));
        assert_eq!(sig_term_caught(), 0);
        assert!(e_sig_term_sched_handler(SIGTERM_COMPAT));
        assert!(e_sig_term_sched_handler(SIGTERM_COMPAT));
        assert_eq!(sig_term_caught(), 2);
    }

    #[test]
    fn unexpected_signals_are_reported_as_continuable() {
        let _guard = global_state_lock();
        reset_signal_state_for_tests();

        assert_eq!(
            e_signal_handler(999),
            SignalOutcome::UnexpectedSignal { signal: 999 }
        );
    }
}
