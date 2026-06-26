use crate::basics::defines::DEFAULT_COMCHAR_RAW;
use crate::basics::error::{Diagnostic, ErrorCode};
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
use std::sync::OnceLock;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

pub const DEFAULT_PAGE_SIZE: isize = 4096;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i32)]
pub enum RLimResult {
    Failed = 0,
    Reduced = 1,
    Success = 2,
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

#[must_use]
pub const fn set_soft_rlimit(_resource: i32, _limit: u64) -> RLimResult {
    RLimResult::Failed
}

#[must_use]
pub const fn get_soft_rlimit(_resource: i32) -> u64 {
    0
}

#[must_use]
pub const fn set_memory_limit(mem_limit: u64) -> RLimResult {
    if mem_limit == 0 {
        RLimResult::Success
    } else {
        RLimResult::Failed
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
    ResourceUsage {
        user_time_seconds: fallback_user_time_seconds(),
        system_time_seconds: 0.0,
        max_resident_pages: 0,
    }
}

#[must_use]
pub fn get_core_number() -> usize {
    std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get)
}

#[must_use]
pub const fn get_system_page_size() -> isize {
    DEFAULT_PAGE_SIZE
}

#[cfg(target_os = "linux")]
#[must_use]
pub fn get_system_phys_memory() -> i64 {
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

#[cfg(not(target_os = "linux"))]
#[must_use]
pub const fn get_system_phys_memory() -> i64 {
    -1
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

#[cfg(test)]
mod tests {
    use super::{
        current_resource_usage, format_resource_usage, get_core_number, get_msec_time,
        get_sec_time, get_sec_time_mod, get_soft_rlimit, get_system_page_size,
        get_system_phys_memory, get_usec_clock, get_usec_time, secure_fclose, secure_fopen,
        set_memory_limit, set_soft_rlimit, stride_memory, RLimResult, ResourceUsage,
        DEFAULT_PAGE_SIZE,
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
    fn unsupported_resource_limits_are_explicit() {
        assert_eq!(set_soft_rlimit(0, 1), RLimResult::Failed);
        assert_eq!(get_soft_rlimit(0), 0);
        assert_eq!(set_memory_limit(0), RLimResult::Success);
        assert_eq!(set_memory_limit(1), RLimResult::Failed);
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
        assert_eq!(get_system_page_size(), DEFAULT_PAGE_SIZE);
        assert!(get_system_phys_memory() >= -1);
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
