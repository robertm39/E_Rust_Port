use crate::basics::defines::DEFAULT_COMCHAR_DIRECT;
use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::os_wrapper::get_usec_clock;
#[cfg(all(target_os = "linux", not(test)))]
use crate::basics::os_wrapper::set_rlimit;
#[cfg(target_os = "linux")]
use crate::basics::os_wrapper::{get_hard_rlimit, RLIMIT_CPU_COMPAT};
use crate::inout::tempfile::temp_file_cleanup;
use std::io::{self, Write};
use std::sync::atomic::{
    AtomicBool, AtomicI32, AtomicI64, AtomicU64, AtomicU8, AtomicUsize, Ordering,
};

pub const RLIM_INFINITY_COMPAT: u64 = u64::MAX;
pub const SIGINT_COMPAT: i32 = 2;
pub const SIGTERM_COMPAT: i32 = 15;
pub const SIGXCPU_COMPAT: i32 = 24;
const USEC_PER_SEC: u64 = 1_000_000;
const TIME_LIMIT_KIND_NONE: u8 = 0;
const TIME_LIMIT_KIND_SOFT: u8 = 1;
const TIME_LIMIT_KIND_HARD: u8 = 2;

static SCHEDULE_TIME_LIMIT: AtomicU64 = AtomicU64::new(0);
static SYSTEM_TIME_LIMIT: AtomicU64 = AtomicU64::new(RLIM_INFINITY_COMPAT);
static SOFT_TIME_LIMIT: AtomicU64 = AtomicU64::new(RLIM_INFINITY_COMPAT);
static HARD_TIME_LIMIT: AtomicU64 = AtomicU64::new(RLIM_INFINITY_COMPAT);
static TIME_IS_UP: AtomicBool = AtomicBool::new(false);
static TIME_LIMIT_IS_SOFT: AtomicBool = AtomicBool::new(false);
static TIME_LIMIT_START_USEC: AtomicI64 = AtomicI64::new(0);
static TIME_LIMIT_DEADLINE_USEC: AtomicI64 = AtomicI64::new(i64::MAX);
static TIME_LIMIT_DEADLINE_KIND: AtomicU8 = AtomicU8::new(TIME_LIMIT_KIND_NONE);
static TIME_LIMIT_EXPIRED_KIND: AtomicU8 = AtomicU8::new(TIME_LIMIT_KIND_NONE);
static SIG_TERM_CAUGHT: AtomicUsize = AtomicUsize::new(0);
static FATAL_ERROR_IN_PROGRESS: AtomicBool = AtomicBool::new(false);
static SILENT_TIME_OUT: AtomicBool = AtomicBool::new(false);
static SIGNAL_GLOBAL_OUT_FD: AtomicI32 = AtomicI32::new(1);

#[cfg(unix)]
#[must_use]
pub fn terminate_process(process_id: u32) -> bool {
    posix_process_signal::terminate(process_id)
}

#[cfg(not(unix))]
#[must_use]
pub const fn terminate_process(_process_id: u32) -> bool {
    false
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct CpuSoftTimeoutRLimitSequence {
    reset_current: u64,
    reset_maximum: u64,
    rearm_current: u64,
    rearm_maximum: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SignalOutcome {
    HandlerInstalled {
        signal: i32,
    },
    HandlerInstallFailed {
        signal: i32,
        diagnostic: Diagnostic,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TimeLimitKind {
    Soft,
    Hard,
}

impl TimeLimitKind {
    const fn marker(self) -> u8 {
        match self {
            Self::Soft => TIME_LIMIT_KIND_SOFT,
            Self::Hard => TIME_LIMIT_KIND_HARD,
        }
    }

    const fn from_marker(marker: u8) -> Option<Self> {
        match marker {
            TIME_LIMIT_KIND_SOFT => Some(Self::Soft),
            TIME_LIMIT_KIND_HARD => Some(Self::Hard),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SchedulerSignalOutcome {
    SigTermCaught {
        count: usize,
        default_reset_attempted: bool,
    },
    Ignored {
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

pub fn configure_time_limits(hard_limit: u64, soft_limit: u64, schedule_limit: u64) {
    HARD_TIME_LIMIT.store(hard_limit, Ordering::SeqCst);
    SOFT_TIME_LIMIT.store(soft_limit, Ordering::SeqCst);
    SCHEDULE_TIME_LIMIT.store(schedule_limit, Ordering::SeqCst);
    TIME_IS_UP.store(false, Ordering::SeqCst);
    TIME_LIMIT_EXPIRED_KIND.store(TIME_LIMIT_KIND_NONE, Ordering::SeqCst);

    let (active_limit, deadline_kind) = if soft_limit != RLIM_INFINITY_COMPAT {
        TIME_LIMIT_IS_SOFT.store(true, Ordering::SeqCst);
        (Some(soft_limit), TimeLimitKind::Soft.marker())
    } else if hard_limit != RLIM_INFINITY_COMPAT {
        TIME_LIMIT_IS_SOFT.store(false, Ordering::SeqCst);
        (Some(hard_limit), TimeLimitKind::Hard.marker())
    } else {
        TIME_LIMIT_IS_SOFT.store(false, Ordering::SeqCst);
        (None, TIME_LIMIT_KIND_NONE)
    };

    let start = get_usec_clock();
    TIME_LIMIT_START_USEC.store(start, Ordering::SeqCst);
    TIME_LIMIT_DEADLINE_KIND.store(deadline_kind, Ordering::SeqCst);
    TIME_LIMIT_DEADLINE_USEC.store(
        active_limit.map_or(i64::MAX, |limit| time_limit_deadline_usec(start, limit)),
        Ordering::SeqCst,
    );
}

#[must_use]
pub fn time_is_up() -> bool {
    if TIME_IS_UP.load(Ordering::SeqCst) {
        return true;
    }

    let deadline = TIME_LIMIT_DEADLINE_USEC.load(Ordering::SeqCst);
    if deadline == i64::MAX {
        return false;
    }
    if get_usec_clock() < deadline {
        return false;
    }

    TIME_IS_UP.store(true, Ordering::SeqCst);
    TIME_LIMIT_IS_SOFT.store(false, Ordering::SeqCst);
    TIME_LIMIT_EXPIRED_KIND.store(
        TIME_LIMIT_DEADLINE_KIND.load(Ordering::SeqCst),
        Ordering::SeqCst,
    );
    true
}

#[must_use]
pub fn set_time_is_up(value: bool) -> bool {
    TIME_LIMIT_EXPIRED_KIND.store(TIME_LIMIT_KIND_NONE, Ordering::SeqCst);
    TIME_IS_UP.swap(value, Ordering::SeqCst)
}

#[must_use]
pub fn time_limit_expired_kind() -> Option<TimeLimitKind> {
    TimeLimitKind::from_marker(TIME_LIMIT_EXPIRED_KIND.load(Ordering::SeqCst))
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
pub fn signal_global_out_fd() -> i32 {
    SIGNAL_GLOBAL_OUT_FD.load(Ordering::SeqCst)
}

#[must_use]
pub fn set_signal_global_out_fd(fd: i32) -> i32 {
    SIGNAL_GLOBAL_OUT_FD.swap(fd, Ordering::SeqCst)
}

#[must_use]
pub fn e_signal_setup(signal: i32) -> SignalOutcome {
    #[cfg(target_os = "linux")]
    {
        SYSTEM_TIME_LIMIT.store(get_hard_rlimit(RLIMIT_CPU_COMPAT), Ordering::SeqCst);
    }
    #[cfg(not(target_os = "linux"))]
    {
        SYSTEM_TIME_LIMIT.store(RLIM_INFINITY_COMPAT, Ordering::SeqCst);
    }

    #[cfg(all(target_os = "linux", not(test)))]
    {
        if !linux_signal::install_e_signal_handler(signal) {
            return SignalOutcome::HandlerInstallFailed {
                signal,
                diagnostic: Diagnostic::new(
                    ErrorCode::SYSTEM_ERROR,
                    "Unable to set up signal handler",
                ),
            };
        }
    }

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
pub fn e_sig_term_sched_handler(signal: i32) -> SchedulerSignalOutcome {
    if signal == SIGTERM_COMPAT {
        let count = SIG_TERM_CAUGHT.fetch_add(1, Ordering::SeqCst) + 1;
        restore_scheduler_sigterm_default();
        SchedulerSignalOutcome::SigTermCaught {
            count,
            default_reset_attempted: true,
        }
    } else {
        SchedulerSignalOutcome::Ignored { signal }
    }
}

pub fn finalize_cpu_limit_outcome(
    output: &mut impl Write,
    outcome: &SignalOutcome,
) -> io::Result<Option<u8>> {
    match outcome {
        SignalOutcome::CpuLimitExceeded { silent: true, .. } => {
            Ok(Some(ErrorCode::CPU_LIMIT_ERROR.exit_status()))
        }
        SignalOutcome::CpuLimitExceeded { silent: false, .. } => {
            writeln!(
                output,
                "\n{DEFAULT_COMCHAR_DIRECT} Failure: Resource limit exceeded (time)"
            )?;
            writeln!(output, "{DEFAULT_COMCHAR_DIRECT} SZS status ResourceOut")?;
            Ok(Some(ErrorCode::CPU_LIMIT_ERROR.exit_status()))
        }
        _ => Ok(None),
    }
}

pub fn finalize_signal_outcome(
    output: &mut impl Write,
    stderr: &mut impl Write,
    outcome: &SignalOutcome,
) -> io::Result<Option<u8>> {
    if matches!(outcome, SignalOutcome::UnexpectedSignal { .. }) {
        stderr.write_all(b"Warning: ")?;
        stderr.write_all(b"Unexpected signal caught, continuing")?;
        return Ok(None);
    }
    finalize_cpu_limit_outcome(output, outcome)
}

fn handle_cpu_limit() -> SignalOutcome {
    if TIME_LIMIT_IS_SOFT.swap(false, Ordering::SeqCst) {
        TIME_IS_UP.store(true, Ordering::SeqCst);
        TIME_LIMIT_EXPIRED_KIND.store(TimeLimitKind::Soft.marker(), Ordering::SeqCst);
        let rlimit_sequence = cpu_soft_timeout_rlimit_sequence(
            HARD_TIME_LIMIT.load(Ordering::SeqCst),
            SYSTEM_TIME_LIMIT.load(Ordering::SeqCst),
        );
        #[cfg(all(target_os = "linux", not(test)))]
        {
            let _ = set_rlimit(
                RLIMIT_CPU_COMPAT,
                rlimit_sequence.reset_current,
                rlimit_sequence.reset_maximum,
            );
            let _ = set_rlimit(
                RLIMIT_CPU_COMPAT,
                rlimit_sequence.rearm_current,
                rlimit_sequence.rearm_maximum,
            );
        }
        let _ = e_signal_setup(SIGXCPU_COMPAT);
        return SignalOutcome::SoftTimeLimitReached {
            next_limit: rlimit_sequence.rearm_current,
        };
    }

    TIME_LIMIT_EXPIRED_KIND.store(TimeLimitKind::Hard.marker(), Ordering::SeqCst);
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

fn time_limit_deadline_usec(start_usec: i64, limit_seconds: u64) -> i64 {
    let limit_usec = limit_seconds.saturating_mul(USEC_PER_SEC);
    let limit_usec = i64::try_from(limit_usec).unwrap_or(i64::MAX);
    start_usec.saturating_add(limit_usec)
}

fn restore_scheduler_sigterm_default() {
    #[cfg(all(target_os = "linux", not(test)))]
    {
        let _ = linux_signal::restore_default_handler(SIGTERM_COMPAT);
    }
}

#[must_use]
const fn cpu_soft_timeout_rlimit_sequence(
    hard_limit: u64,
    system_limit: u64,
) -> CpuSoftTimeoutRLimitSequence {
    CpuSoftTimeoutRLimitSequence {
        reset_current: system_limit,
        reset_maximum: system_limit,
        rearm_current: if hard_limit < system_limit {
            hard_limit
        } else {
            system_limit
        },
        rearm_maximum: system_limit,
    }
}

// Allowed external shared-library boundary: normal subprocess cleanup uses
// libc's process-directed signal API to match `EPCtrlCleanup`.
#[cfg(unix)]
#[allow(unsafe_code)]
mod posix_process_signal {
    use super::SIGTERM_COMPAT;

    unsafe extern "C" {
        fn kill(process_id: i32, signal_number: i32) -> i32;
    }

    pub(super) fn terminate(process_id: u32) -> bool {
        let Ok(process_id) = i32::try_from(process_id) else {
            return false;
        };
        // SAFETY: kill receives a positive process identifier obtained from
        // std::process::Child and the POSIX SIGTERM value. It does not borrow
        // Rust memory and mirrors C `EPCtrlCleanup`'s best-effort call.
        unsafe { kill(process_id, SIGTERM_COMPAT) == 0 }
    }
}

// Allowed external shared-library boundary: POSIX signal registration and
// signal-time descriptor writes use libc's process-global ABI.
#[cfg(all(target_os = "linux", not(test)))]
#[allow(unsafe_code)]
mod linux_signal {
    use super::{e_signal_handler, signal_global_out_fd, ErrorCode, SignalOutcome};
    use std::ffi::c_void;

    type SignalHandler = extern "C" fn(i32);

    const SIG_ERR_COMPAT: usize = usize::MAX;
    const STDERR_FILENO_COMPAT: i32 = 2;
    const HARD_CPU_TIMEOUT_OUTPUT: &[u8] =
        b"\n%% Failure: Resource limit exceeded (time)\n%% SZS status ResourceOut\n";
    const HARD_CPU_TIMEOUT_ERROR: &[u8] = b"eprover: CPU time limit exceeded, terminating\n";
    const UNEXPECTED_SIGNAL_WARNING: &[u8] = b"Warning: Unexpected signal caught, continuing";

    unsafe extern "C" {
        fn exit(status: i32) -> !;
        fn raise(signal_number: i32) -> i32;
        fn write(fd: i32, buffer: *const c_void, count: usize) -> isize;
        #[link_name = "signal"]
        fn signal_compat(signum: i32, handler: Option<SignalHandler>) -> usize;
    }

    pub(super) fn install_e_signal_handler(signal_number: i32) -> bool {
        // SAFETY: signal installs a process-global handler. The handler is an
        // extern "C" function with the required integer signal argument and no
        // captured Rust state.
        (unsafe { signal_compat(signal_number, Some(signal_trampoline)) }) != SIG_ERR_COMPAT
    }

    pub(super) fn restore_default_handler(signal_number: i32) -> bool {
        // SAFETY: SIG_DFL is the C signal API's null handler sentinel. This is
        // the same process-global reset used by C `ESigTermSchedHandler`.
        (unsafe { signal_compat(signal_number, None) }) != SIG_ERR_COMPAT
    }

    pub(super) fn restore_default_and_reraise(signal_number: i32) {
        let _ = restore_default_handler(signal_number);
        // SAFETY: raise is libc's process-global signal API. The signal number
        // comes from the active C signal trampoline and mirrors C
        // `ESignalHandler` after resetting the handler to SIG_DFL.
        let _ = unsafe { raise(signal_number) };
    }

    fn write_fd_all(fd: i32, mut buffer: &[u8]) {
        while !buffer.is_empty() {
            // SAFETY: write is called with a raw file descriptor and a pointer
            // into the live byte slice for exactly its current length, matching
            // C `WriteStr` use from `ESignalHandler`.
            let result = unsafe { write(fd, buffer.as_ptr().cast::<c_void>(), buffer.len()) };
            let Ok(written) = usize::try_from(result) else {
                break;
            };
            if written == 0 {
                break;
            }
            buffer = &buffer[written.min(buffer.len())..];
        }
    }

    fn finalize_hard_cpu_signal_and_exit(outcome: &SignalOutcome) -> ! {
        if matches!(
            outcome,
            SignalOutcome::CpuLimitExceeded { silent: false, .. }
        ) {
            write_fd_all(signal_global_out_fd(), HARD_CPU_TIMEOUT_OUTPUT);
            write_fd_all(STDERR_FILENO_COMPAT, HARD_CPU_TIMEOUT_ERROR);
        }
        // SAFETY: exit is libc's process-termination API. C `ESignalHandler`
        // calls exit directly for silent hard CPU timeouts and reaches it via
        // Error(...) for non-silent hard CPU timeouts.
        unsafe { exit(i32::from(ErrorCode::CPU_LIMIT_ERROR.exit_status())) }
    }

    extern "C" fn signal_trampoline(signal_number: i32) {
        let outcome = e_signal_handler(signal_number);
        match &outcome {
            SignalOutcome::Terminate { .. } => restore_default_and_reraise(signal_number),
            SignalOutcome::CpuLimitExceeded { .. } => finalize_hard_cpu_signal_and_exit(&outcome),
            SignalOutcome::UnexpectedSignal { .. } => {
                write_fd_all(STDERR_FILENO_COMPAT, UNEXPECTED_SIGNAL_WARNING);
            }
            SignalOutcome::HandlerInstalled { .. }
            | SignalOutcome::HandlerInstallFailed { .. }
            | SignalOutcome::SoftTimeLimitReached { .. } => {}
        }
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
    TIME_LIMIT_START_USEC.store(0, Ordering::SeqCst);
    TIME_LIMIT_DEADLINE_USEC.store(i64::MAX, Ordering::SeqCst);
    TIME_LIMIT_DEADLINE_KIND.store(TIME_LIMIT_KIND_NONE, Ordering::SeqCst);
    TIME_LIMIT_EXPIRED_KIND.store(TIME_LIMIT_KIND_NONE, Ordering::SeqCst);
    SIG_TERM_CAUGHT.store(0, Ordering::SeqCst);
    FATAL_ERROR_IN_PROGRESS.store(false, Ordering::SeqCst);
    SILENT_TIME_OUT.store(false, Ordering::SeqCst);
    SIGNAL_GLOBAL_OUT_FD.store(1, Ordering::SeqCst);
}

#[cfg(test)]
mod tests {
    use super::{
        configure_time_limits, e_sig_term_sched_handler, e_signal_handler, e_signal_setup,
        finalize_cpu_limit_outcome, finalize_signal_outcome, hard_time_limit,
        reset_signal_state_for_tests, schedule_time_limit, set_hard_time_limit,
        set_schedule_time_limit, set_signal_global_out_fd, set_silent_time_out,
        set_soft_time_limit, set_system_time_limit, set_time_is_up, set_time_limit_is_soft,
        sig_term_caught, signal_global_out_fd, silent_time_out, soft_time_limit, system_time_limit,
        time_is_up, time_limit_expired_kind, time_limit_is_soft, SchedulerSignalOutcome,
        SignalOutcome, TimeLimitKind, RLIM_INFINITY_COMPAT, SIGINT_COMPAT, SIGTERM_COMPAT,
        SIGXCPU_COMPAT,
    };
    use crate::basics::error::{Diagnostic, ErrorCode};
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
        assert_eq!(signal_global_out_fd(), 1);
    }

    #[test]
    fn process_termination_rejects_pid_outside_posix_range() {
        assert!(!super::terminate_process(u32::MAX));
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
        assert_eq!(set_signal_global_out_fd(9), 1);

        assert_eq!(schedule_time_limit(), 10);
        assert_eq!(soft_time_limit(), 20);
        assert_eq!(hard_time_limit(), 30);
        assert_eq!(system_time_limit(), 40);
        assert!(time_is_up());
        assert!(time_limit_is_soft());
        assert!(silent_time_out());
        assert_eq!(signal_global_out_fd(), 9);
    }

    #[test]
    fn setup_records_system_limit_shape_without_installing_raw_handler_in_tests() {
        let _guard = global_state_lock();
        reset_signal_state_for_tests();
        let _ = set_system_time_limit(123);

        assert_eq!(
            e_signal_setup(SIGXCPU_COMPAT),
            SignalOutcome::HandlerInstalled {
                signal: SIGXCPU_COMPAT
            }
        );
        #[cfg(target_os = "linux")]
        {
            assert_ne!(system_time_limit(), 123);
            assert!(system_time_limit() > 0);
        }
        #[cfg(not(target_os = "linux"))]
        assert_eq!(system_time_limit(), RLIM_INFINITY_COMPAT);
    }

    #[test]
    fn configured_soft_cpu_limit_latches_time_up_cooperatively() {
        let _guard = global_state_lock();
        reset_signal_state_for_tests();

        configure_time_limits(10, 0, 0);

        assert_eq!(hard_time_limit(), 10);
        assert_eq!(soft_time_limit(), 0);
        assert!(time_limit_is_soft());
        assert!(time_is_up());
        assert!(!time_limit_is_soft());
        assert_eq!(time_limit_expired_kind(), Some(TimeLimitKind::Soft));
    }

    #[test]
    fn configured_hard_cpu_limit_latches_time_up_without_soft_marker() {
        let _guard = global_state_lock();
        reset_signal_state_for_tests();

        configure_time_limits(0, RLIM_INFINITY_COMPAT, 0);

        assert_eq!(hard_time_limit(), 0);
        assert_eq!(soft_time_limit(), RLIM_INFINITY_COMPAT);
        assert!(!time_limit_is_soft());
        assert!(time_is_up());
        assert_eq!(time_limit_expired_kind(), Some(TimeLimitKind::Hard));
    }

    #[test]
    fn configuring_unlimited_cpu_limits_resets_latch_and_deadline() {
        let _guard = global_state_lock();
        reset_signal_state_for_tests();
        let _ = set_time_is_up(true);
        let _ = set_time_limit_is_soft(true);

        configure_time_limits(RLIM_INFINITY_COMPAT, RLIM_INFINITY_COMPAT, 0);

        assert!(!time_is_up());
        assert!(!time_limit_is_soft());
        assert_eq!(time_limit_expired_kind(), None);
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
        assert_eq!(time_limit_expired_kind(), Some(TimeLimitKind::Soft));
        #[cfg(target_os = "linux")]
        {
            assert_ne!(system_time_limit(), 50);
            assert!(system_time_limit() > 0);
        }
        #[cfg(not(target_os = "linux"))]
        assert_eq!(system_time_limit(), RLIM_INFINITY_COMPAT);
    }

    #[test]
    fn soft_cpu_signal_rlimit_sequence_matches_c_reset_and_rearm_shape() {
        assert_eq!(
            super::cpu_soft_timeout_rlimit_sequence(75, 50),
            super::CpuSoftTimeoutRLimitSequence {
                reset_current: 50,
                reset_maximum: 50,
                rearm_current: 50,
                rearm_maximum: 50,
            }
        );
        assert_eq!(
            super::cpu_soft_timeout_rlimit_sequence(25, 50),
            super::CpuSoftTimeoutRLimitSequence {
                reset_current: 50,
                reset_maximum: 50,
                rearm_current: 25,
                rearm_maximum: 50,
            }
        );
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
        assert_eq!(time_limit_expired_kind(), Some(TimeLimitKind::Hard));

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
        assert_eq!(time_limit_expired_kind(), Some(TimeLimitKind::Hard));
    }

    #[test]
    fn cpu_limit_outcome_finalizer_writes_c_hard_timeout_shape() {
        let mut output = Vec::new();
        let status = finalize_cpu_limit_outcome(
            &mut output,
            &SignalOutcome::CpuLimitExceeded {
                silent: false,
                diagnostic: Some(Diagnostic::new(
                    ErrorCode::CPU_LIMIT_ERROR,
                    "CPU time limit exceeded, terminating",
                )),
            },
        )
        .unwrap();

        assert_eq!(status, Some(ErrorCode::CPU_LIMIT_ERROR.exit_status()));
        assert_eq!(
            String::from_utf8(output).unwrap(),
            "\n%% Failure: Resource limit exceeded (time)\n%% SZS status ResourceOut\n"
        );
    }

    #[test]
    fn cpu_limit_outcome_finalizer_keeps_silent_timeout_quiet() {
        let mut output = Vec::new();
        let status = finalize_cpu_limit_outcome(
            &mut output,
            &SignalOutcome::CpuLimitExceeded {
                silent: true,
                diagnostic: None,
            },
        )
        .unwrap();

        assert_eq!(status, Some(ErrorCode::CPU_LIMIT_ERROR.exit_status()));
        assert!(output.is_empty());
    }

    #[test]
    fn cpu_limit_outcome_finalizer_ignores_other_signal_outcomes() {
        let mut output = Vec::new();

        assert_eq!(
            finalize_cpu_limit_outcome(
                &mut output,
                &SignalOutcome::UnexpectedSignal { signal: 999 }
            )
            .unwrap(),
            None
        );
        assert!(output.is_empty());
    }

    #[test]
    fn signal_outcome_finalizer_writes_c_unexpected_signal_warning() {
        let mut output = Vec::new();
        let mut stderr = Vec::new();

        assert_eq!(
            finalize_signal_outcome(
                &mut output,
                &mut stderr,
                &SignalOutcome::UnexpectedSignal { signal: 999 },
            )
            .unwrap(),
            None
        );
        assert!(output.is_empty());
        assert_eq!(
            String::from_utf8(stderr).unwrap(),
            "Warning: Unexpected signal caught, continuing"
        );
    }

    #[test]
    fn signal_outcome_finalizer_delegates_cpu_timeout_shape() {
        let mut output = Vec::new();
        let mut stderr = Vec::new();
        let status = finalize_signal_outcome(
            &mut output,
            &mut stderr,
            &SignalOutcome::CpuLimitExceeded {
                silent: false,
                diagnostic: Some(Diagnostic::new(
                    ErrorCode::CPU_LIMIT_ERROR,
                    "CPU time limit exceeded, terminating",
                )),
            },
        )
        .unwrap();

        assert_eq!(status, Some(ErrorCode::CPU_LIMIT_ERROR.exit_status()));
        assert_eq!(
            String::from_utf8(output).unwrap(),
            "\n%% Failure: Resource limit exceeded (time)\n%% SZS status ResourceOut\n"
        );
        assert!(stderr.is_empty());
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

        assert_eq!(
            e_sig_term_sched_handler(SIGINT_COMPAT),
            SchedulerSignalOutcome::Ignored {
                signal: SIGINT_COMPAT
            }
        );
        assert_eq!(sig_term_caught(), 0);
        assert_eq!(
            e_sig_term_sched_handler(SIGTERM_COMPAT),
            SchedulerSignalOutcome::SigTermCaught {
                count: 1,
                default_reset_attempted: true,
            }
        );
        assert_eq!(
            e_sig_term_sched_handler(SIGTERM_COMPAT),
            SchedulerSignalOutcome::SigTermCaught {
                count: 2,
                default_reset_attempted: true,
            }
        );
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
