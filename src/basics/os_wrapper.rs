use crate::basics::defines::DEFAULT_COMCHAR_RAW;
use crate::basics::error::{Diagnostic, ErrorCode};
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
#[cfg(windows)]
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

pub const DEFAULT_PAGE_SIZE: isize = 4096;

#[cfg(windows)]
static WAITED_CHILD_KERNEL_TIME_100NS: AtomicU64 = AtomicU64::new(0);
#[cfg(windows)]
static WAITED_CHILD_USER_TIME_100NS: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i32)]
pub enum RLimResult {
    Failed = 0,
    Reduced = 1,
    Success = 2,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RLimitOutcome {
    result: RLimResult,
    os_error: Option<i32>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResourceUsage {
    pub user_time_seconds: f64,
    pub system_time_seconds: f64,
    pub max_resident_pages: u64,
}

impl RLimResult {
    #[must_use]
    pub const fn c_value(self) -> i32 {
        self as i32
    }
}

impl RLimitOutcome {
    #[must_use]
    pub const fn new(result: RLimResult, os_error: Option<i32>) -> Self {
        Self { result, os_error }
    }

    #[must_use]
    pub const fn result(self) -> RLimResult {
        self.result
    }

    #[must_use]
    pub const fn os_error(self) -> Option<i32> {
        self.os_error
    }
}

#[cfg(target_os = "linux")]
pub const RLIMIT_CPU_COMPAT: i32 = linux_rlimit::RLIMIT_CPU;
#[cfg(target_os = "linux")]
pub const RLIMIT_CORE_COMPAT: i32 = linux_rlimit::RLIMIT_CORE;
#[cfg(target_os = "linux")]
pub const RLIMIT_DATA_COMPAT: i32 = linux_rlimit::RLIMIT_DATA;

#[cfg(target_os = "linux")]
#[must_use]
pub fn set_soft_rlimit(resource: i32, limit: u64) -> RLimResult {
    set_soft_rlimit_with_error(resource, limit).result()
}

#[cfg(not(target_os = "linux"))]
#[must_use]
pub const fn set_soft_rlimit(_resource: i32, _limit: u64) -> RLimResult {
    RLimResult::Failed
}

#[cfg(target_os = "linux")]
#[must_use]
pub fn set_soft_rlimit_with_error(resource: i32, limit: u64) -> RLimitOutcome {
    linux_rlimit::set_soft_rlimit_with_error(resource, limit)
}

#[cfg(not(target_os = "linux"))]
#[must_use]
pub const fn set_soft_rlimit_with_error(_resource: i32, _limit: u64) -> RLimitOutcome {
    RLimitOutcome::new(RLimResult::Failed, None)
}

#[cfg(target_os = "linux")]
#[must_use]
pub fn set_rlimit(resource: i32, current: u64, maximum: u64) -> RLimResult {
    linux_rlimit::set_rlimit(resource, current, maximum)
}

#[cfg(not(target_os = "linux"))]
#[must_use]
pub const fn set_rlimit(_resource: i32, _current: u64, _maximum: u64) -> RLimResult {
    RLimResult::Failed
}

#[cfg(target_os = "linux")]
#[must_use]
pub fn get_soft_rlimit(resource: i32) -> u64 {
    linux_rlimit::get_soft_rlimit(resource)
}

#[cfg(not(target_os = "linux"))]
#[must_use]
pub const fn get_soft_rlimit(_resource: i32) -> u64 {
    0
}

#[cfg(target_os = "linux")]
#[must_use]
pub fn get_hard_rlimit(resource: i32) -> u64 {
    linux_rlimit::get_hard_rlimit(resource)
}

#[cfg(not(target_os = "linux"))]
#[must_use]
pub const fn get_hard_rlimit(_resource: i32) -> u64 {
    0
}

#[cfg(target_os = "linux")]
#[must_use]
pub fn set_memory_limit(mem_limit: u64) -> RLimResult {
    if mem_limit == 0 {
        return RLimResult::Success;
    }

    let data_result = set_soft_rlimit(RLIMIT_DATA_COMPAT, mem_limit);
    // Preserve the C implementation's RLIMIT_AS branch bug: when RLIMIT_AS is
    // available, it labels the warning as RLIMIT_AS but still passes RLIMIT_DATA.
    let as_result = set_soft_rlimit(RLIMIT_DATA_COMPAT, mem_limit);
    combine_rlimit_results(data_result, as_result)
}

#[cfg(windows)]
#[must_use]
pub fn set_hard_cpu_limit(limit_seconds: u64) -> RLimResult {
    windows_kernel32::set_process_cpu_limit(limit_seconds)
}

#[cfg(windows)]
#[must_use]
pub fn set_memory_limit(mem_limit: u64) -> RLimResult {
    if mem_limit == 0 {
        RLimResult::Success
    } else {
        windows_kernel32::set_process_memory_limit(mem_limit)
    }
}

#[cfg(not(any(target_os = "linux", windows)))]
#[must_use]
pub const fn set_memory_limit(mem_limit: u64) -> RLimResult {
    if mem_limit == 0 {
        RLimResult::Success
    } else {
        RLimResult::Failed
    }
}

#[cfg(any(test, target_os = "linux"))]
#[must_use]
const fn combine_rlimit_results(first: RLimResult, second: RLimResult) -> RLimResult {
    match (first, second) {
        (RLimResult::Failed, _) | (_, RLimResult::Failed) => RLimResult::Failed,
        (RLimResult::Reduced, _) | (_, RLimResult::Reduced) => RLimResult::Reduced,
        (RLimResult::Success, RLimResult::Success) => RLimResult::Success,
    }
}

#[must_use]
pub fn get_usec_time() -> i64 {
    let Ok(duration) = SystemTime::now().duration_since(UNIX_EPOCH) else {
        return 0;
    };
    i64::try_from(duration.as_micros()).unwrap_or(i64::MAX)
}

#[must_use]
pub fn get_usec_clock() -> i64 {
    static START: OnceLock<Instant> = OnceLock::new();

    #[cfg(windows)]
    {
        if let Some((kernel_100ns, user_100ns)) = windows_process_times_100ns() {
            let total_100ns = kernel_100ns.saturating_add(user_100ns);
            return i64::try_from(total_100ns / 10).unwrap_or(i64::MAX);
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Some(clock_usec) = linux_resource::process_clock_usec() {
            return clock_usec;
        }
    }

    let start = START.get_or_init(Instant::now);
    i64::try_from(start.elapsed().as_micros()).unwrap_or(i64::MAX)
}

#[must_use]
pub fn get_msec_time() -> i64 {
    get_usec_time() / 1_000
}

#[must_use]
pub fn get_sec_time() -> i64 {
    get_usec_time() / 1_000_000
}

#[must_use]
pub fn get_sec_time_mod() -> i64 {
    get_sec_time() % 1_000
}

#[must_use]
pub fn format_resource_usage(usage: ResourceUsage) -> String {
    format!(
        "\n{DEFAULT_COMCHAR_RAW} -------------------------------------------------\n\
         {DEFAULT_COMCHAR_RAW} User time                : {:.3} s\n\
         {DEFAULT_COMCHAR_RAW} System time              : {:.3} s\n\
         {DEFAULT_COMCHAR_RAW} Total time               : {:.3} s\n\
         {DEFAULT_COMCHAR_RAW} Maximum resident set size: {} pages\n",
        usage.user_time_seconds,
        usage.system_time_seconds,
        usage.user_time_seconds + usage.system_time_seconds,
        usage.max_resident_pages
    )
}

#[must_use]
pub fn current_resource_usage() -> ResourceUsage {
    #[cfg(windows)]
    {
        if let Some((kernel_100ns, user_100ns)) = windows_process_times_100ns() {
            let child_kernel = WAITED_CHILD_KERNEL_TIME_100NS.load(Ordering::SeqCst);
            let child_user = WAITED_CHILD_USER_TIME_100NS.load(Ordering::SeqCst);
            return ResourceUsage {
                user_time_seconds: filetime_100ns_to_seconds(user_100ns.saturating_add(child_user)),
                system_time_seconds: filetime_100ns_to_seconds(
                    kernel_100ns.saturating_add(child_kernel),
                ),
                max_resident_pages: windows_peak_working_set_pages().unwrap_or(0),
            };
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Some(usage) = linux_getrusage_resource_usage() {
            return usage;
        }
        if let Some(usage) = linux_proc_resource_usage() {
            return usage;
        }
    }

    ResourceUsage {
        user_time_seconds: fallback_user_time_seconds(),
        system_time_seconds: 0.0,
        max_resident_pages: 0,
    }
}

/// Returns CPU seconds charged to this process, excluding waited-for children.
///
/// This is the Rust counterpart of C `GetTotalCPUTime()`. Resource summaries
/// continue to use [`current_resource_usage`], which includes child CPU time on
/// Linux like C `PrintRusage()`.
#[must_use]
pub fn current_process_cpu_time_seconds() -> f64 {
    #[cfg(windows)]
    {
        if let Some((kernel_100ns, user_100ns)) = windows_process_times_100ns() {
            return filetime_100ns_to_seconds(user_100ns) + filetime_100ns_to_seconds(kernel_100ns);
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Some(usage) = linux_resource::getrusage(linux_resource::RUSAGE_SELF) {
            return timeval_seconds(usage.user_time) + timeval_seconds(usage.system_time);
        }
    }

    fallback_user_time_seconds()
}

#[cfg(windows)]
pub fn record_waited_child_resource_usage(child: &std::process::Child) {
    record_waited_child_resource_usage_with_report(child, None);
}

#[cfg(windows)]
pub fn record_waited_child_resource_usage_with_report(
    child: &std::process::Child,
    reported: Option<ResourceUsage>,
) {
    use std::os::windows::io::AsRawHandle;

    let Some((mut kernel_100ns, mut user_100ns)) =
        windows_kernel32::process_times_100ns_for_handle(child.as_raw_handle().cast())
    else {
        return;
    };
    if let Some(reported) = reported {
        kernel_100ns = kernel_100ns.max(seconds_to_100ns_saturating(reported.system_time_seconds));
        user_100ns = user_100ns.max(seconds_to_100ns_saturating(reported.user_time_seconds));
    }
    WAITED_CHILD_KERNEL_TIME_100NS.fetch_add(kernel_100ns, Ordering::SeqCst);
    WAITED_CHILD_USER_TIME_100NS.fetch_add(user_100ns, Ordering::SeqCst);
}

#[cfg(not(windows))]
pub fn record_waited_child_resource_usage(_child: &std::process::Child) {}

#[cfg(not(windows))]
pub fn record_waited_child_resource_usage_with_report(
    _child: &std::process::Child,
    _reported: Option<ResourceUsage>,
) {
}

#[cfg(windows)]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn seconds_to_100ns_saturating(seconds: f64) -> u64 {
    if !seconds.is_finite() || seconds <= 0.0 {
        return 0;
    }
    let ticks = seconds * 10_000_000.0;
    if ticks >= u64::MAX as f64 {
        u64::MAX
    } else {
        ticks.round() as u64
    }
}

#[must_use]
pub fn get_core_number() -> usize {
    #[cfg(target_os = "linux")]
    {
        if let Some(core_number) = linux_resource::online_processor_count() {
            return core_number;
        }
    }

    std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get)
}

#[must_use]
pub fn get_system_page_size() -> isize {
    #[cfg(windows)]
    {
        if let Some(page_size) = windows_system_page_size() {
            return page_size;
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Some(page_size) = linux_resource::system_page_size() {
            return page_size;
        }
    }

    DEFAULT_PAGE_SIZE
}

#[cfg(windows)]
#[must_use]
pub fn get_system_phys_memory() -> i64 {
    windows_physical_memory_mb().unwrap_or(-1)
}

#[cfg(target_os = "linux")]
#[must_use]
pub fn get_system_phys_memory() -> i64 {
    if let Some(memory_mb) = linux_resource::physical_memory_mb() {
        return memory_mb;
    }

    let Ok(contents) = std::fs::read_to_string("/proc/meminfo") else {
        return -1;
    };
    for line in contents.lines() {
        let Some(rest) = line.strip_prefix("MemTotal:") else {
            continue;
        };
        let Some(kib_text) = rest.split_whitespace().next() else {
            return -1;
        };
        return kib_text.parse::<i64>().map_or(-1, |kib| kib / 1_024);
    }
    -1
}

#[cfg(not(any(target_os = "linux", windows)))]
#[must_use]
pub const fn get_system_phys_memory() -> i64 {
    -1
}

#[must_use]
pub fn resource_limit_error_message(error_code: i32) -> String {
    #[cfg(target_os = "linux")]
    {
        linux_rlimit::strerror_message(error_code)
    }

    #[cfg(not(target_os = "linux"))]
    {
        std::io::Error::from_raw_os_error(error_code).to_string()
    }
}

pub fn stride_memory(memory: &mut [u8]) {
    let page_size = usize::try_from(get_system_page_size())
        .ok()
        .filter(|size| *size > 0)
        .unwrap_or(usize::try_from(DEFAULT_PAGE_SIZE).unwrap_or(4096));

    for index in (0..memory.len()).step_by(page_size) {
        memory[index] = b'S';
    }
}

fn open_options_for_mode(mode: &str) -> Option<OpenOptions> {
    let mut options = OpenOptions::new();
    let plus = mode.as_bytes().contains(&b'+');
    match mode.as_bytes().first().copied()? {
        b'r' => {
            options.read(true);
            if plus {
                options.write(true);
            }
        }
        b'w' => {
            options.write(true).create(true).truncate(true);
            if plus {
                options.read(true);
            }
        }
        b'a' => {
            options.append(true).create(true);
            if plus {
                options.read(true);
            }
        }
        _ => return None,
    }
    Some(options)
}

pub fn secure_fopen(path: impl AsRef<Path>, mode: &str) -> Result<File, Diagnostic> {
    let Some(options) = open_options_for_mode(mode) else {
        return Err(Diagnostic::new(
            ErrorCode::USAGE_ERROR,
            format!("Unsupported fopen mode {mode}"),
        ));
    };
    let path = path.as_ref();
    options.open(path).map_err(|error| {
        Diagnostic::new(
            ErrorCode::FILE_ERROR,
            format!("Cannot open file {}: {error}", path.display()),
        )
    })
}

pub fn secure_fclose(mut file: File) -> Result<(), Diagnostic> {
    file.flush().map_err(|error| {
        Diagnostic::new(
            ErrorCode::FILE_ERROR,
            format!("Problem closing file: {error}"),
        )
    })
}

pub fn secure_fclose_io(file: File) -> io::Result<()> {
    let mut file = file;
    file.flush()
}

#[allow(clippy::cast_precision_loss)]
fn fallback_user_time_seconds() -> f64 {
    get_usec_clock() as f64 / 1_000_000.0
}

#[cfg(target_os = "linux")]
fn linux_getrusage_resource_usage() -> Option<ResourceUsage> {
    let self_usage = linux_resource::getrusage(linux_resource::RUSAGE_SELF)?;
    let child_usage = linux_resource::getrusage(linux_resource::RUSAGE_CHILDREN)?;
    Some(resource_usage_from_linux_rusage(&self_usage, &child_usage))
}

#[cfg(any(test, target_os = "linux"))]
#[allow(clippy::cast_precision_loss)]
fn resource_usage_from_linux_rusage(
    self_usage: &linux_resource::RUsage,
    child_usage: &linux_resource::RUsage,
) -> ResourceUsage {
    ResourceUsage {
        user_time_seconds: timeval_seconds(self_usage.user_time)
            + timeval_seconds(child_usage.user_time),
        system_time_seconds: timeval_seconds(self_usage.system_time)
            + timeval_seconds(child_usage.system_time),
        // C PrintRusage adds child CPU time but still prints the parent
        // ru_maxrss field.
        max_resident_pages: u64::try_from(self_usage.max_resident_set_size).unwrap_or(0),
    }
}

#[cfg(any(test, target_os = "linux"))]
#[allow(clippy::cast_lossless, clippy::cast_precision_loss)]
fn timeval_seconds(time: linux_resource::TimeVal) -> f64 {
    time.seconds as f64 + time.microseconds as f64 / 1_000_000.0
}

#[cfg(target_os = "linux")]
fn linux_proc_resource_usage() -> Option<ResourceUsage> {
    let stat = std::fs::read_to_string("/proc/self/stat").ok()?;
    let (user_ticks, system_ticks) = parse_linux_stat_cpu_ticks(&stat)?;
    let status = std::fs::read_to_string("/proc/self/status").ok();
    Some(ResourceUsage {
        user_time_seconds: linux_ticks_to_seconds(user_ticks),
        system_time_seconds: linux_ticks_to_seconds(system_ticks),
        // Linux getrusage returns ru_maxrss in KiB, despite E's historical
        // "pages" label. Preserve the raw Linux-style value for compatibility.
        max_resident_pages: status
            .as_deref()
            .and_then(parse_linux_status_vm_hwm_kib)
            .unwrap_or(0),
    })
}

#[cfg(any(test, target_os = "linux"))]
fn parse_linux_stat_cpu_ticks(stat: &str) -> Option<(u64, u64)> {
    let (_, rest) = stat.rsplit_once(") ")?;
    let mut fields = rest.split_whitespace().skip(11);
    let user_ticks = parse_non_negative_u64(fields.next()?)?;
    let system_ticks = parse_non_negative_u64(fields.next()?)?;
    let child_user_ticks = parse_non_negative_u64(fields.next()?)?;
    let child_system_ticks = parse_non_negative_u64(fields.next()?)?;
    Some((
        user_ticks.saturating_add(child_user_ticks),
        system_ticks.saturating_add(child_system_ticks),
    ))
}

#[cfg(any(test, target_os = "linux"))]
fn parse_non_negative_u64(value: &str) -> Option<u64> {
    let parsed = value.parse::<i128>().ok()?;
    u64::try_from(parsed).ok()
}

#[cfg(any(test, target_os = "linux"))]
fn parse_linux_status_vm_hwm_kib(status: &str) -> Option<u64> {
    status.lines().find_map(|line| {
        let rest = line.strip_prefix("VmHWM:")?;
        rest.split_whitespace().next()?.parse::<u64>().ok()
    })
}

#[cfg(target_os = "linux")]
#[allow(clippy::cast_precision_loss)]
fn linux_ticks_to_seconds(ticks: u64) -> f64 {
    // Linux exposes /proc CPU fields in USER_HZ units. sysconf(_SC_CLK_TCK)
    // would require libc FFI; the port keeps this safe and documents that
    // exact nonstandard tick rates should be revisited if reference tests need
    // them.
    ticks as f64 / 100.0
}

#[cfg(windows)]
#[allow(clippy::cast_precision_loss)]
fn filetime_100ns_to_seconds(value: u64) -> f64 {
    value as f64 / 10_000_000.0
}

#[cfg(windows)]
fn windows_process_times_100ns() -> Option<(u64, u64)> {
    windows_kernel32::process_times_100ns()
}

#[cfg(windows)]
fn windows_system_page_size() -> Option<isize> {
    windows_kernel32::system_page_size()
}

#[cfg(windows)]
fn windows_physical_memory_mb() -> Option<i64> {
    windows_kernel32::physical_memory_mb()
}

#[cfg(windows)]
fn windows_peak_working_set_pages() -> Option<u64> {
    windows_kernel32::peak_working_set_pages(windows_system_page_size()?)
}

// Allowed external DLL boundary: Kernel32/PSAPI resource calls are isolated here
// and converted to Rust result values before leaving the module.
#[cfg(windows)]
#[allow(unsafe_code)]
mod windows_kernel32 {
    use super::RLimResult;
    use std::ffi::c_void;
    use std::mem::{size_of, MaybeUninit};
    use std::num::NonZeroUsize;
    use std::ptr;
    use std::sync::{Mutex, OnceLock};

    type Bool = i32;
    type Dword = u32;
    type Handle = *mut c_void;

    const JOB_OBJECT_EXTENDED_LIMIT_INFORMATION_CLASS: i32 = 9;
    pub(super) const JOB_OBJECT_LIMIT_PROCESS_TIME: Dword = 0x0000_0002;
    pub(super) const JOB_OBJECT_LIMIT_PROCESS_MEMORY: Dword = 0x0000_0100;
    const HUNDRED_NS_PER_SEC: u64 = 10_000_000;

    #[repr(C)]
    struct FileTime {
        low_date_time: Dword,
        high_date_time: Dword,
    }

    #[repr(C)]
    struct SystemInfo {
        processor_architecture: u16,
        reserved: u16,
        page_size: Dword,
        minimum_application_address: *mut c_void,
        maximum_application_address: *mut c_void,
        active_processor_mask: usize,
        number_of_processors: Dword,
        processor_type: Dword,
        allocation_granularity: Dword,
        processor_level: u16,
        processor_revision: u16,
    }

    #[repr(C)]
    struct MemoryStatusEx {
        length: Dword,
        memory_load: Dword,
        total_physical: u64,
        available_physical: u64,
        total_page_file: u64,
        available_page_file: u64,
        total_virtual: u64,
        available_virtual: u64,
        available_extended_virtual: u64,
    }

    #[repr(C)]
    struct ProcessMemoryCounters {
        size: Dword,
        page_fault_count: Dword,
        peak_working_set_size: usize,
        working_set_size: usize,
        quota_peak_paged_pool_usage: usize,
        quota_paged_pool_usage: usize,
        quota_peak_non_paged_pool_usage: usize,
        quota_non_paged_pool_usage: usize,
        pagefile_usage: usize,
        peak_pagefile_usage: usize,
    }

    #[repr(C)]
    pub(super) struct JobObjectBasicLimitInformation {
        pub(super) per_process_user_time_limit: i64,
        per_job_user_time_limit: i64,
        pub(super) limit_flags: Dword,
        minimum_working_set_size: usize,
        maximum_working_set_size: usize,
        active_process_limit: Dword,
        affinity: usize,
        priority_class: Dword,
        scheduling_class: Dword,
    }

    #[expect(
        clippy::struct_field_names,
        reason = "Win32 IO_COUNTERS layout uses count-suffixed fields"
    )]
    #[repr(C)]
    struct IoCounters {
        read_operation_count: u64,
        write_operation_count: u64,
        other_operation_count: u64,
        read_transfer_count: u64,
        write_transfer_count: u64,
        other_transfer_count: u64,
    }

    #[repr(C)]
    pub(super) struct JobObjectExtendedLimitInformation {
        pub(super) basic_limit_information: JobObjectBasicLimitInformation,
        io_info: IoCounters,
        pub(super) process_memory_limit: usize,
        job_memory_limit: usize,
        peak_process_memory_used: usize,
        peak_job_memory_used: usize,
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn AssignProcessToJobObject(job: Handle, process: Handle) -> Bool;
        fn CloseHandle(object: Handle) -> Bool;
        fn CreateJobObjectW(job_attributes: *mut c_void, name: *const u16) -> Handle;
        fn GetCurrentProcess() -> Handle;
        fn GetProcessTimes(
            process: Handle,
            creation_time: *mut FileTime,
            exit_time: *mut FileTime,
            kernel_time: *mut FileTime,
            user_time: *mut FileTime,
        ) -> Bool;
        fn GetSystemInfo(system_info: *mut SystemInfo);
        fn GlobalMemoryStatusEx(memory_status: *mut MemoryStatusEx) -> Bool;
        fn K32GetProcessMemoryInfo(
            process: Handle,
            counters: *mut ProcessMemoryCounters,
            size: Dword,
        ) -> Bool;
        fn SetInformationJobObject(
            job: Handle,
            info_class: i32,
            job_object_info: *mut c_void,
            job_object_info_length: Dword,
        ) -> Bool;
    }

    #[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
    pub(super) struct JobLimitState {
        pub(super) process_time_100ns: Option<i64>,
        pub(super) process_memory_limit: Option<usize>,
    }

    pub(super) fn set_process_memory_limit(limit: u64) -> RLimResult {
        let Ok(limit) = usize::try_from(limit) else {
            return RLimResult::Failed;
        };
        update_process_limits(|limits| limits.process_memory_limit = Some(limit))
    }

    pub(super) fn set_process_cpu_limit(limit_seconds: u64) -> RLimResult {
        let Some(limit) = seconds_to_100ns(limit_seconds) else {
            return RLimResult::Failed;
        };
        update_process_limits(|limits| limits.process_time_100ns = Some(limit))
    }

    fn update_process_limits(update: impl FnOnce(&mut JobLimitState)) -> RLimResult {
        static PROCESS_LIMITS: Mutex<JobLimitState> = Mutex::new(JobLimitState {
            process_time_100ns: None,
            process_memory_limit: None,
        });

        let mut limits = match PROCESS_LIMITS.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        update(&mut limits);
        apply_process_limits(*limits)
    }

    fn apply_process_limits(limits: JobLimitState) -> RLimResult {
        let Some(job) = process_limit_job() else {
            return RLimResult::Failed;
        };
        let Ok(info_size) = Dword::try_from(size_of::<JobObjectExtendedLimitInformation>()) else {
            return RLimResult::Failed;
        };
        let mut info = job_object_extended_limit_information(limits);

        // SAFETY: job is a live job-object handle retained for the process
        // lifetime, info points to an initialized JOBOBJECT_EXTENDED_LIMIT_INFORMATION
        // buffer, and info_size matches that buffer's layout.
        let ok = unsafe {
            SetInformationJobObject(
                job,
                JOB_OBJECT_EXTENDED_LIMIT_INFORMATION_CLASS,
                (&raw mut info).cast::<c_void>(),
                info_size,
            )
        };
        if ok == 0 {
            RLimResult::Failed
        } else {
            RLimResult::Success
        }
    }

    pub(super) fn seconds_to_100ns(seconds: u64) -> Option<i64> {
        let ticks = seconds.checked_mul(HUNDRED_NS_PER_SEC)?;
        i64::try_from(ticks).ok()
    }

    pub(super) fn process_times_100ns() -> Option<(u64, u64)> {
        // SAFETY: GetCurrentProcess returns a valid pseudo-handle for the
        // calling process.
        process_times_100ns_for_handle(unsafe { GetCurrentProcess() })
    }

    pub(super) fn process_times_100ns_for_handle(process: *mut c_void) -> Option<(u64, u64)> {
        let mut creation_time = MaybeUninit::<FileTime>::uninit();
        let mut exit_time = MaybeUninit::<FileTime>::uninit();
        let mut kernel_time = MaybeUninit::<FileTime>::uninit();
        let mut user_time = MaybeUninit::<FileTime>::uninit();

        // SAFETY: process is a live process handle retained by std::process or
        // the current-process pseudo-handle, and all FILETIME pointers refer
        // to writable, aligned storage that the OS initializes on success.
        let ok = unsafe {
            GetProcessTimes(
                process,
                creation_time.as_mut_ptr(),
                exit_time.as_mut_ptr(),
                kernel_time.as_mut_ptr(),
                user_time.as_mut_ptr(),
            )
        };
        if ok == 0 {
            return None;
        }

        // SAFETY: GetProcessTimes returned success, so kernel_time and
        // user_time have been initialized by the OS. The creation and exit
        // values are intentionally ignored.
        let kernel_time = unsafe { kernel_time.assume_init() };
        let user_time = unsafe { user_time.assume_init() };
        Some((file_time_to_u64(&kernel_time), file_time_to_u64(&user_time)))
    }

    pub(super) fn system_page_size() -> Option<isize> {
        let mut system_info = MaybeUninit::<SystemInfo>::uninit();
        // SAFETY: system_info points to writable, properly aligned storage for
        // the OS to initialize. GetSystemInfo has no failure return.
        unsafe {
            GetSystemInfo(system_info.as_mut_ptr());
        }
        // SAFETY: GetSystemInfo always initializes the SYSTEM_INFO buffer.
        let system_info = unsafe { system_info.assume_init() };
        isize::try_from(system_info.page_size)
            .ok()
            .filter(|page_size| *page_size > 0)
    }

    pub(super) fn physical_memory_mb() -> Option<i64> {
        let size = Dword::try_from(size_of::<MemoryStatusEx>()).ok()?;
        let mut status = MemoryStatusEx {
            length: size,
            memory_load: 0,
            total_physical: 0,
            available_physical: 0,
            total_page_file: 0,
            available_page_file: 0,
            total_virtual: 0,
            available_virtual: 0,
            available_extended_virtual: 0,
        };

        // SAFETY: status is a valid MEMORYSTATUSEX buffer with dwLength set as
        // required by GlobalMemoryStatusEx.
        let ok = unsafe { GlobalMemoryStatusEx(&raw mut status) };
        if ok == 0 {
            return None;
        }

        i64::try_from(status.total_physical / 1_048_576).ok()
    }

    pub(super) fn peak_working_set_pages(page_size: isize) -> Option<u64> {
        let page_size = u64::try_from(page_size).ok().filter(|size| *size > 0)?;
        let size = Dword::try_from(size_of::<ProcessMemoryCounters>()).ok()?;
        let mut counters = ProcessMemoryCounters {
            size,
            page_fault_count: 0,
            peak_working_set_size: 0,
            working_set_size: 0,
            quota_peak_paged_pool_usage: 0,
            quota_paged_pool_usage: 0,
            quota_peak_non_paged_pool_usage: 0,
            quota_non_paged_pool_usage: 0,
            pagefile_usage: 0,
            peak_pagefile_usage: 0,
        };

        // SAFETY: GetCurrentProcess returns a valid pseudo-handle, and
        // counters points to a writable PROCESS_MEMORY_COUNTERS buffer whose
        // size field and cb argument match the C API contract.
        let ok = unsafe { K32GetProcessMemoryInfo(GetCurrentProcess(), &raw mut counters, size) };
        if ok == 0 {
            return None;
        }

        let peak = u64::try_from(counters.peak_working_set_size).ok()?;
        Some(peak.div_ceil(page_size))
    }

    fn process_limit_job() -> Option<Handle> {
        static PROCESS_LIMIT_JOB: OnceLock<Option<NonZeroUsize>> = OnceLock::new();

        PROCESS_LIMIT_JOB
            .get_or_init(create_assigned_job)
            .map(|job| job.get() as Handle)
    }

    fn create_assigned_job() -> Option<NonZeroUsize> {
        // SAFETY: null security attributes and null name request an unnamed
        // job object from Kernel32; the returned handle is checked for null.
        let job = unsafe { CreateJobObjectW(ptr::null_mut(), ptr::null()) };
        if job.is_null() {
            return None;
        }

        // SAFETY: GetCurrentProcess returns a valid pseudo-handle, and job is
        // the non-null job handle returned by CreateJobObjectW.
        let assigned = unsafe { AssignProcessToJobObject(job, GetCurrentProcess()) };
        if assigned == 0 {
            // SAFETY: job is an owned handle returned by CreateJobObjectW and
            // has not been stored for process-lifetime use.
            let _ = unsafe { CloseHandle(job) };
            return None;
        }

        NonZeroUsize::new(job as usize)
    }

    pub(super) const fn job_object_extended_limit_information(
        limits: JobLimitState,
    ) -> JobObjectExtendedLimitInformation {
        let (time_flag, time_limit) = match limits.process_time_100ns {
            Some(limit) => (JOB_OBJECT_LIMIT_PROCESS_TIME, limit),
            None => (0, 0),
        };
        let (memory_flag, memory_limit) = match limits.process_memory_limit {
            Some(limit) => (JOB_OBJECT_LIMIT_PROCESS_MEMORY, limit),
            None => (0, 0),
        };
        JobObjectExtendedLimitInformation {
            basic_limit_information: JobObjectBasicLimitInformation {
                per_process_user_time_limit: time_limit,
                per_job_user_time_limit: 0,
                limit_flags: time_flag | memory_flag,
                minimum_working_set_size: 0,
                maximum_working_set_size: 0,
                active_process_limit: 0,
                affinity: 0,
                priority_class: 0,
                scheduling_class: 0,
            },
            io_info: IoCounters {
                read_operation_count: 0,
                write_operation_count: 0,
                other_operation_count: 0,
                read_transfer_count: 0,
                write_transfer_count: 0,
                other_transfer_count: 0,
            },
            process_memory_limit: memory_limit,
            job_memory_limit: 0,
            peak_process_memory_used: 0,
            peak_job_memory_used: 0,
        }
    }

    fn file_time_to_u64(time: &FileTime) -> u64 {
        (u64::from(time.high_date_time) << 32) | u64::from(time.low_date_time)
    }
}

// Allowed external shared-library boundary: libc rlimit calls and strerror stay
// inside this module and return typed Rust outcomes/messages.
#[cfg(target_os = "linux")]
#[allow(unsafe_code)]
mod linux_rlimit {
    use super::{RLimResult, RLimitOutcome};
    use std::ffi::{c_char, CStr};
    use std::io;
    use std::mem::MaybeUninit;

    pub(super) const RLIMIT_CPU: i32 = 0;
    pub(super) const RLIMIT_DATA: i32 = 2;
    pub(super) const RLIMIT_CORE: i32 = 4;

    #[repr(C)]
    struct RLimit {
        current: u64,
        maximum: u64,
    }

    unsafe extern "C" {
        fn getrlimit(resource: i32, limit: *mut RLimit) -> i32;
        fn setrlimit(resource: i32, limit: *const RLimit) -> i32;
        fn strerror(errnum: i32) -> *mut c_char;
    }

    pub(super) fn set_soft_rlimit_with_error(resource: i32, mut limit: u64) -> RLimitOutcome {
        let mut rlimit = match get_rlimit(resource) {
            Ok(rlimit) => rlimit,
            Err(error) => return RLimitOutcome::new(RLimResult::Failed, error),
        };

        let mut result = RLimResult::Success;
        if rlimit.maximum < limit {
            result = RLimResult::Reduced;
            limit = rlimit.maximum;
        }
        rlimit.current = limit;

        // SAFETY: rlimit is a valid pointer to an initialized rlimit struct
        // whose layout matches Linux's two-rlim_t struct rlimit ABI.
        if unsafe { setrlimit(resource, &raw const rlimit) } == -1 {
            return RLimitOutcome::new(RLimResult::Failed, last_os_error_code());
        }
        RLimitOutcome::new(result, None)
    }

    pub(super) fn set_rlimit(resource: i32, current: u64, maximum: u64) -> RLimResult {
        let rlimit = RLimit { current, maximum };
        set_rlimit_raw(resource, &rlimit).result()
    }

    pub(super) fn get_soft_rlimit(resource: i32) -> u64 {
        get_rlimit(resource).map_or(0, |limit| limit.current)
    }

    pub(super) fn get_hard_rlimit(resource: i32) -> u64 {
        get_rlimit(resource).map_or(0, |limit| limit.maximum)
    }

    pub(super) fn strerror_message(error_code: i32) -> String {
        // SAFETY: strerror returns a pointer to a nul-terminated C string for
        // the supplied errno value. The string is copied immediately.
        let message = unsafe { strerror(error_code) };
        if message.is_null() {
            return format!("Unknown error {error_code}");
        }
        // SAFETY: strerror returned a non-null pointer to a nul-terminated C
        // string that remains valid long enough for this immediate copy.
        unsafe { CStr::from_ptr(message) }
            .to_string_lossy()
            .into_owned()
    }

    fn get_rlimit(resource: i32) -> Result<RLimit, Option<i32>> {
        let mut rlimit = MaybeUninit::<RLimit>::uninit();
        // SAFETY: rlimit points to writable, properly aligned storage for the
        // C library to initialize on success.
        if unsafe { getrlimit(resource, rlimit.as_mut_ptr()) } == -1 {
            return Err(last_os_error_code());
        }
        // SAFETY: getrlimit returned success, so the rlimit buffer is
        // initialized by the C library.
        Ok(unsafe { rlimit.assume_init() })
    }

    fn set_rlimit_raw(resource: i32, rlimit: &RLimit) -> RLimitOutcome {
        // SAFETY: rlimit is a valid pointer to an initialized rlimit struct
        // whose layout matches Linux's two-rlim_t struct rlimit ABI.
        if unsafe { setrlimit(resource, rlimit) } == -1 {
            RLimitOutcome::new(RLimResult::Failed, last_os_error_code())
        } else {
            RLimitOutcome::new(RLimResult::Success, None)
        }
    }

    fn last_os_error_code() -> Option<i32> {
        io::Error::last_os_error().raw_os_error()
    }
}

// Allowed external shared-library boundary: libc process-resource queries stay
// isolated here and return plain Rust measurements.
#[cfg(any(test, target_os = "linux"))]
#[allow(unsafe_code)]
mod linux_resource {
    use std::ffi::c_long;
    #[cfg(target_os = "linux")]
    use std::mem::MaybeUninit;

    #[cfg(target_os = "linux")]
    pub(super) const RUSAGE_SELF: i32 = 0;
    #[cfg(target_os = "linux")]
    pub(super) const RUSAGE_CHILDREN: i32 = -1;
    #[cfg(target_os = "linux")]
    const CLOCKS_PER_SEC_COMPAT: c_long = 1_000_000;
    #[cfg(target_os = "linux")]
    const SC_PAGESIZE_COMPAT: i32 = 30;
    #[cfg(target_os = "linux")]
    const SC_NPROCESSORS_ONLN_COMPAT: i32 = 84;
    #[cfg(target_os = "linux")]
    const SC_PHYS_PAGES_COMPAT: i32 = 85;

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    #[repr(C)]
    pub(super) struct TimeVal {
        pub(super) seconds: c_long,
        pub(super) microseconds: c_long,
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    #[repr(C)]
    pub(super) struct RUsage {
        pub(super) user_time: TimeVal,
        pub(super) system_time: TimeVal,
        pub(super) max_resident_set_size: c_long,
        shared_memory_size: c_long,
        unshared_data_size: c_long,
        unshared_stack_size: c_long,
        minor_page_faults: c_long,
        major_page_faults: c_long,
        swaps: c_long,
        block_input_operations: c_long,
        block_output_operations: c_long,
        ipc_messages_sent: c_long,
        ipc_messages_received: c_long,
        signals_received: c_long,
        voluntary_context_switches: c_long,
        involuntary_context_switches: c_long,
    }

    #[cfg(target_os = "linux")]
    unsafe extern "C" {
        fn clock() -> c_long;
        #[link_name = "getrusage"]
        fn libc_getrusage(who: i32, usage: *mut RUsage) -> i32;
        fn sysconf(name: i32) -> c_long;
    }

    #[cfg(target_os = "linux")]
    pub(super) fn process_clock_usec() -> Option<i64> {
        // SAFETY: clock has no pointer arguments and returns process CPU ticks
        // for the current process, matching C GetUSecClock's use.
        let ticks = unsafe { clock() };
        clock_ticks_to_usec(ticks, CLOCKS_PER_SEC_COMPAT)
    }

    #[cfg(target_os = "linux")]
    pub(super) fn online_processor_count() -> Option<usize> {
        positive_c_long_to_usize(sysconf_positive(SC_NPROCESSORS_ONLN_COMPAT)?)
    }

    #[cfg(target_os = "linux")]
    pub(super) fn system_page_size() -> Option<isize> {
        positive_c_long_to_isize(sysconf_positive(SC_PAGESIZE_COMPAT)?)
    }

    #[cfg(target_os = "linux")]
    pub(super) fn physical_memory_mb() -> Option<i64> {
        let page_size = sysconf_positive(SC_PAGESIZE_COMPAT)?;
        let page_count = sysconf_positive(SC_PHYS_PAGES_COMPAT)?;
        physical_memory_mb_from_sysconf(page_size, page_count)
    }

    #[cfg(any(test, target_os = "linux"))]
    pub(super) fn clock_ticks_to_usec(ticks: c_long, ticks_per_sec: c_long) -> Option<i64> {
        if ticks < 0 || ticks_per_sec <= 0 {
            return None;
        }
        let usec = i128::from(ticks) * 1_000_000_i128 / i128::from(ticks_per_sec);
        i64::try_from(usec).ok()
    }

    #[cfg(any(test, target_os = "linux"))]
    pub(super) fn physical_memory_mb_from_sysconf(
        page_size: c_long,
        page_count: c_long,
    ) -> Option<i64> {
        if page_size <= 0 || page_count <= 0 {
            return None;
        }
        let bytes = i128::from(page_size) * i128::from(page_count);
        i64::try_from(bytes / 1_048_576_i128).ok()
    }

    #[cfg(any(test, target_os = "linux"))]
    pub(super) fn positive_c_long_to_usize(value: c_long) -> Option<usize> {
        if value <= 0 {
            return None;
        }
        usize::try_from(value).ok()
    }

    #[cfg(any(test, target_os = "linux"))]
    pub(super) fn positive_c_long_to_isize(value: c_long) -> Option<isize> {
        if value <= 0 {
            return None;
        }
        isize::try_from(value).ok()
    }

    #[cfg(target_os = "linux")]
    fn sysconf_positive(name: i32) -> Option<c_long> {
        // SAFETY: sysconf has no pointer arguments. The selector constants are
        // the Linux/glibc values used by the C source's target environment.
        let value = unsafe { sysconf(name) };
        (value > 0).then_some(value)
    }

    #[cfg(target_os = "linux")]
    pub(super) fn getrusage(who: i32) -> Option<RUsage> {
        let mut usage = MaybeUninit::<RUsage>::uninit();
        // SAFETY: usage points to writable, properly aligned storage for the C
        // library to initialize on success. The selector is one of the POSIX
        // RUSAGE_* constants used by the C source.
        if unsafe { libc_getrusage(who, usage.as_mut_ptr()) } == -1 {
            return None;
        }
        // SAFETY: getrusage returned success, so usage is initialized.
        Some(unsafe { usage.assume_init() })
    }

    #[cfg(test)]
    impl RUsage {
        pub(super) const fn new(
            user_time: TimeVal,
            system_time: TimeVal,
            max_resident_set_size: c_long,
        ) -> Self {
            Self {
                user_time,
                system_time,
                max_resident_set_size,
                shared_memory_size: 0,
                unshared_data_size: 0,
                unshared_stack_size: 0,
                minor_page_faults: 0,
                major_page_faults: 0,
                swaps: 0,
                block_input_operations: 0,
                block_output_operations: 0,
                ipc_messages_sent: 0,
                ipc_messages_received: 0,
                signals_received: 0,
                voluntary_context_switches: 0,
                involuntary_context_switches: 0,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        combine_rlimit_results, current_resource_usage, format_resource_usage, get_core_number,
        get_msec_time, get_sec_time, get_sec_time_mod, get_system_page_size,
        get_system_phys_memory, get_usec_clock, get_usec_time, parse_linux_stat_cpu_ticks,
        parse_linux_status_vm_hwm_kib, resource_limit_error_message,
        resource_usage_from_linux_rusage, secure_fclose, secure_fopen, set_memory_limit,
        stride_memory, RLimResult, RLimitOutcome, ResourceUsage,
    };
    use crate::basics::error::ErrorCode;
    use std::io::Write;

    #[test]
    fn rlimit_result_discriminants_match_c_enum() {
        assert_eq!(RLimResult::Failed.c_value(), 0);
        assert_eq!(RLimResult::Reduced.c_value(), 1);
        assert_eq!(RLimResult::Success.c_value(), 2);
    }

    #[test]
    fn rlimit_outcome_preserves_result_and_errno() {
        let outcome = RLimitOutcome::new(RLimResult::Failed, Some(22));

        assert_eq!(outcome.result(), RLimResult::Failed);
        assert_eq!(outcome.os_error(), Some(22));
        assert!(!resource_limit_error_message(22).is_empty());
    }

    #[cfg(not(any(target_os = "linux", windows)))]
    #[test]
    fn unsupported_resource_limits_are_explicit() {
        use super::{get_hard_rlimit, get_soft_rlimit, set_rlimit, set_soft_rlimit};

        assert_eq!(set_soft_rlimit(0, 1), RLimResult::Failed);
        assert_eq!(set_rlimit(0, 1, 1), RLimResult::Failed);
        assert_eq!(get_soft_rlimit(0), 0);
        assert_eq!(get_hard_rlimit(0), 0);
        assert_eq!(set_memory_limit(0), RLimResult::Success);
        assert_eq!(set_memory_limit(1), RLimResult::Failed);
    }

    #[cfg(windows)]
    #[test]
    fn windows_zero_memory_limit_is_noop() {
        assert_eq!(set_memory_limit(0), RLimResult::Success);
    }

    #[cfg(windows)]
    #[test]
    fn windows_job_limit_info_sets_combined_process_limits() {
        let info = super::windows_kernel32::job_object_extended_limit_information(
            super::windows_kernel32::JobLimitState {
                process_time_100ns: Some(30_000_000),
                process_memory_limit: Some(4096),
            },
        );

        assert_eq!(
            info.basic_limit_information.limit_flags
                & super::windows_kernel32::JOB_OBJECT_LIMIT_PROCESS_MEMORY,
            super::windows_kernel32::JOB_OBJECT_LIMIT_PROCESS_MEMORY
        );
        assert_eq!(
            info.basic_limit_information.limit_flags
                & super::windows_kernel32::JOB_OBJECT_LIMIT_PROCESS_TIME,
            super::windows_kernel32::JOB_OBJECT_LIMIT_PROCESS_TIME
        );
        assert_eq!(
            info.basic_limit_information.per_process_user_time_limit,
            30_000_000
        );
        assert_eq!(info.process_memory_limit, 4096);
    }

    #[cfg(windows)]
    #[test]
    fn windows_cpu_limit_seconds_convert_to_job_time_units() {
        assert_eq!(
            super::windows_kernel32::seconds_to_100ns(3),
            Some(30_000_000)
        );
        assert_eq!(super::windows_kernel32::seconds_to_100ns(u64::MAX), None);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_rlimit_boundary_reports_invalid_resource_like_c() {
        use super::{
            get_hard_rlimit, get_soft_rlimit, set_rlimit, set_soft_rlimit,
            set_soft_rlimit_with_error,
        };

        assert_eq!(set_soft_rlimit(-1, 1), RLimResult::Failed);
        let detailed = set_soft_rlimit_with_error(-1, 1);
        assert_eq!(detailed.result(), RLimResult::Failed);
        assert!(detailed.os_error().is_some());
        assert_eq!(set_rlimit(-1, 1, 1), RLimResult::Failed);
        assert_eq!(get_soft_rlimit(-1), 0);
        assert_eq!(get_hard_rlimit(-1), 0);
        assert_eq!(set_memory_limit(0), RLimResult::Success);
    }

    #[test]
    fn rlimit_results_combine_like_memory_limit_attempts() {
        assert_eq!(
            combine_rlimit_results(RLimResult::Success, RLimResult::Success),
            RLimResult::Success
        );
        assert_eq!(
            combine_rlimit_results(RLimResult::Reduced, RLimResult::Success),
            RLimResult::Reduced
        );
        assert_eq!(
            combine_rlimit_results(RLimResult::Success, RLimResult::Failed),
            RLimResult::Failed
        );
    }

    #[test]
    fn time_helpers_return_non_negative_c_shaped_units() {
        let first_wall = get_usec_time();
        let second_wall = get_usec_time();
        assert!(first_wall >= 0);
        assert!(second_wall >= first_wall);
        let millis = get_msec_time();
        assert!(millis >= first_wall / 1_000);
        assert!(millis <= get_usec_time() / 1_000);
        assert!(get_sec_time() >= 0);
        assert!(get_sec_time_mod() >= 0);
        assert!(get_sec_time_mod() < 1_000);

        let first_clock = get_usec_clock();
        let second_clock = get_usec_clock();
        assert!(second_clock >= first_clock);
    }

    #[test]
    fn system_queries_have_safe_fallbacks() {
        assert!(get_core_number() >= 1);
        assert!(get_system_page_size() > 0);
        assert!(get_system_phys_memory() >= -1);
    }

    #[cfg(windows)]
    #[test]
    fn windows_system_queries_use_kernel32_values() {
        assert!(get_system_page_size() > 0);
        assert!(get_system_phys_memory() > 0);
        assert!(current_resource_usage().max_resident_pages > 0);
    }

    #[test]
    fn resource_usage_prints_c_shaped_footer() {
        let usage = ResourceUsage {
            user_time_seconds: 1.25,
            system_time_seconds: 0.5,
            max_resident_pages: 42,
        };

        assert_eq!(
            format_resource_usage(usage),
            "\n% -------------------------------------------------\n\
             % User time                : 1.250 s\n\
             % System time              : 0.500 s\n\
             % Total time               : 1.750 s\n\
             % Maximum resident set size: 42 pages\n"
        );

        let current = current_resource_usage();
        assert!(current.user_time_seconds >= 0.0);
        assert!(current.system_time_seconds >= 0.0);
    }

    #[test]
    fn linux_stat_parser_sums_self_and_child_cpu_ticks() {
        let stat = "1234 (eprover worker) R 1 2 3 4 5 6 7 8 9 10 13 17 19 23 24";

        assert_eq!(parse_linux_stat_cpu_ticks(stat), Some((32, 40)));
    }

    #[test]
    fn linux_status_parser_reads_peak_resident_kib() {
        let status = "Name:\teprover\nVmRSS:\t   2048 kB\nVmHWM:\t   4096 kB\n";

        assert_eq!(parse_linux_status_vm_hwm_kib(status), Some(4096));
    }

    #[test]
    fn linux_getrusage_conversion_matches_c_print_rusage_shape() {
        let self_usage = super::linux_resource::RUsage::new(
            super::linux_resource::TimeVal {
                seconds: 1,
                microseconds: 250_000,
            },
            super::linux_resource::TimeVal {
                seconds: 0,
                microseconds: 500_000,
            },
            42,
        );
        let child_usage = super::linux_resource::RUsage::new(
            super::linux_resource::TimeVal {
                seconds: 2,
                microseconds: 750_000,
            },
            super::linux_resource::TimeVal {
                seconds: 3,
                microseconds: 125_000,
            },
            99,
        );

        assert_eq!(
            resource_usage_from_linux_rusage(&self_usage, &child_usage),
            ResourceUsage {
                user_time_seconds: 4.0,
                system_time_seconds: 3.625,
                max_resident_pages: 42,
            }
        );
    }

    #[test]
    fn linux_clock_ticks_convert_like_c_get_usec_clock() {
        assert_eq!(
            super::linux_resource::clock_ticks_to_usec(2_500_000, 1_000_000),
            Some(2_500_000)
        );
        assert_eq!(
            super::linux_resource::clock_ticks_to_usec(3, 2),
            Some(1_500_000)
        );
        assert_eq!(
            super::linux_resource::clock_ticks_to_usec(-1, 1_000_000),
            None
        );
        assert_eq!(super::linux_resource::clock_ticks_to_usec(1, 0), None);
    }

    #[test]
    fn linux_sysconf_conversions_reject_invalid_values() {
        assert_eq!(super::linux_resource::positive_c_long_to_usize(1), Some(1));
        assert_eq!(super::linux_resource::positive_c_long_to_usize(0), None);
        assert_eq!(super::linux_resource::positive_c_long_to_usize(-1), None);
        assert_eq!(
            super::linux_resource::positive_c_long_to_isize(4096),
            Some(4096)
        );
        assert_eq!(super::linux_resource::positive_c_long_to_isize(0), None);
        assert_eq!(super::linux_resource::positive_c_long_to_isize(-4096), None);
    }

    #[test]
    fn linux_sysconf_physical_memory_conversion_matches_c_shape() {
        assert_eq!(
            super::linux_resource::physical_memory_mb_from_sysconf(4096, 262_144),
            Some(1024)
        );
        assert_eq!(
            super::linux_resource::physical_memory_mb_from_sysconf(2048, 1536),
            Some(3)
        );
        assert_eq!(
            super::linux_resource::physical_memory_mb_from_sysconf(0, 262_144),
            None
        );
        assert_eq!(
            super::linux_resource::physical_memory_mb_from_sysconf(4096, -1),
            None
        );
    }

    #[test]
    fn stride_memory_writes_one_byte_per_page() {
        let page_size = usize::try_from(get_system_page_size()).unwrap();
        let mut memory = vec![0_u8; page_size * 2 + 3];

        stride_memory(&mut memory);

        assert_eq!(memory[0], b'S');
        assert_eq!(memory[page_size], b'S');
        assert_eq!(memory[page_size * 2], b'S');
        assert_eq!(memory[1], 0);
    }

    #[test]
    fn secure_fopen_maps_c_modes_to_open_options() {
        let path = std::env::current_dir()
            .unwrap()
            .join("target")
            .join(format!(
                "os-wrapper-test-{}-{}.txt",
                std::process::id(),
                get_usec_time()
            ));

        let mut file = secure_fopen(&path, "w").unwrap();
        file.write_all(b"abc").unwrap();
        secure_fclose(file).unwrap();

        let mut file = secure_fopen(&path, "a").unwrap();
        file.write_all(b"def").unwrap();
        secure_fclose(file).unwrap();

        assert_eq!(std::fs::read(&path).unwrap(), b"abcdef");
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn secure_fopen_reports_invalid_modes_and_missing_files() {
        let invalid = secure_fopen("unused", "z").unwrap_err();
        assert_eq!(invalid.code(), ErrorCode::USAGE_ERROR);

        let missing = secure_fopen("target/definitely-missing-e-port-file", "r").unwrap_err();
        assert_eq!(missing.code(), ErrorCode::FILE_ERROR);
    }
}
