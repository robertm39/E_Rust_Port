use std::io::{self, Write};
use std::sync::atomic::{AtomicI32, Ordering};

use crate::basics::error::program_name;

static VERBOSE_LEVEL: AtomicI32 = AtomicI32::new(0);

#[must_use]
pub fn verbose_level() -> i32 {
    VERBOSE_LEVEL.load(Ordering::SeqCst)
}

pub fn set_verbose_level(level: i32) -> i32 {
    VERBOSE_LEVEL.swap(level, Ordering::SeqCst)
}

#[must_use]
pub fn verbose_enabled() -> bool {
    verbose_level() != 0
}

#[must_use]
pub fn verbose2_enabled() -> bool {
    verbose_level() >= 2
}

#[must_use]
pub fn verbose10_enabled() -> bool {
    verbose_level() >= 10
}

pub fn verbose<R>(action: impl FnOnce() -> R) -> Option<R> {
    if verbose_enabled() {
        Some(action())
    } else {
        None
    }
}

pub fn verbose2<R>(action: impl FnOnce() -> R) -> Option<R> {
    if verbose2_enabled() {
        Some(action())
    } else {
        None
    }
}

pub fn verbose10<R>(action: impl FnOnce() -> R) -> Option<R> {
    if verbose10_enabled() {
        Some(action())
    } else {
        None
    }
}

#[must_use]
pub fn verbout_message(program_name: &str, message: &str) -> String {
    format!("{program_name}: {message}")
}

#[must_use]
pub fn verbout_arg_message(program_name: &str, first: &str, second: &str) -> String {
    format!("{program_name}: {first}{second}\n")
}

pub fn verbout(output: &mut impl Write, program_name: &str, message: &str) -> io::Result<bool> {
    if !verbose_enabled() {
        return Ok(false);
    }
    output.write_all(verbout_message(program_name, message).as_bytes())?;
    output.flush()?;
    Ok(true)
}

pub fn verbout2(output: &mut impl Write, program_name: &str, message: &str) -> io::Result<bool> {
    if !verbose2_enabled() {
        return Ok(false);
    }
    output.write_all(verbout_message(program_name, message).as_bytes())?;
    output.flush()?;
    Ok(true)
}

pub fn verbout10(output: &mut impl Write, program_name: &str, message: &str) -> io::Result<bool> {
    if !verbose10_enabled() {
        return Ok(false);
    }
    output.write_all(verbout_message(program_name, message).as_bytes())?;
    output.flush()?;
    Ok(true)
}

pub fn verbout_arg(
    output: &mut impl Write,
    program_name: &str,
    first: &str,
    second: &str,
) -> io::Result<bool> {
    if !verbose_enabled() {
        return Ok(false);
    }
    output.write_all(verbout_arg_message(program_name, first, second).as_bytes())?;
    output.flush()?;
    Ok(true)
}

pub fn verbout_arg2(
    output: &mut impl Write,
    program_name: &str,
    first: &str,
    second: &str,
) -> io::Result<bool> {
    if !verbose2_enabled() {
        return Ok(false);
    }
    output.write_all(verbout_arg_message(program_name, first, second).as_bytes())?;
    output.flush()?;
    Ok(true)
}

pub fn verbout_global_to(output: &mut impl Write, message: &str) -> io::Result<bool> {
    verbout(output, &program_name(), message)
}

pub fn verbout2_global_to(output: &mut impl Write, message: &str) -> io::Result<bool> {
    verbout2(output, &program_name(), message)
}

pub fn verbout10_global_to(output: &mut impl Write, message: &str) -> io::Result<bool> {
    verbout10(output, &program_name(), message)
}

pub fn verbout_arg_global_to(
    output: &mut impl Write,
    first: &str,
    second: &str,
) -> io::Result<bool> {
    verbout_arg(output, &program_name(), first, second)
}

pub fn verbout_arg2_global_to(
    output: &mut impl Write,
    first: &str,
    second: &str,
) -> io::Result<bool> {
    verbout_arg2(output, &program_name(), first, second)
}

pub fn verbout_global(message: &str) -> io::Result<bool> {
    let stderr = io::stderr();
    let mut output = stderr.lock();
    verbout_global_to(&mut output, message)
}

pub fn verbout2_global(message: &str) -> io::Result<bool> {
    let stderr = io::stderr();
    let mut output = stderr.lock();
    verbout2_global_to(&mut output, message)
}

pub fn verbout10_global(message: &str) -> io::Result<bool> {
    let stderr = io::stderr();
    let mut output = stderr.lock();
    verbout10_global_to(&mut output, message)
}

pub fn verbout_arg_global(first: &str, second: &str) -> io::Result<bool> {
    let stderr = io::stderr();
    let mut output = stderr.lock();
    verbout_arg_global_to(&mut output, first, second)
}

pub fn verbout_arg2_global(first: &str, second: &str) -> io::Result<bool> {
    let stderr = io::stderr();
    let mut output = stderr.lock();
    verbout_arg2_global_to(&mut output, first, second)
}

#[cfg(test)]
mod tests {
    use super::{
        set_verbose_level, verbose, verbose10, verbose10_enabled, verbose2, verbose2_enabled,
        verbose_enabled, verbose_level, verbout, verbout10, verbout10_global_to, verbout2,
        verbout2_global_to, verbout_arg, verbout_arg2, verbout_arg2_global_to,
        verbout_arg_global_to, verbout_arg_message, verbout_global_to, verbout_message,
    };
    use crate::basics::error::init_error;
    use crate::test_support::global_state_lock;

    #[test]
    fn global_level_uses_c_threshold_rules() {
        let _guard = global_state_lock();
        set_verbose_level(0);
        assert_eq!(verbose_level(), 0);
        assert!(!verbose_enabled());
        assert!(!verbose2_enabled());
        assert!(!verbose10_enabled());

        set_verbose_level(-1);
        assert!(verbose_enabled());
        assert!(!verbose2_enabled());
        assert!(!verbose10_enabled());

        set_verbose_level(2);
        assert!(verbose_enabled());
        assert!(verbose2_enabled());
        assert!(!verbose10_enabled());

        set_verbose_level(10);
        assert!(verbose10_enabled());
        set_verbose_level(0);
    }

    #[test]
    fn closure_helpers_execute_only_when_matching_c_macro_would_execute() {
        let _guard = global_state_lock();
        set_verbose_level(1);
        assert_eq!(verbose(|| 3), Some(3));
        assert_eq!(verbose2(|| 4), None);
        assert_eq!(verbose10(|| 5), None);

        set_verbose_level(10);
        assert_eq!(verbose2(|| 4), Some(4));
        assert_eq!(verbose10(|| 5), Some(5));
        set_verbose_level(0);
    }

    #[test]
    fn message_formatting_matches_c_macros() {
        assert_eq!(verbout_message("umlaut", "working"), "umlaut: working");
        assert_eq!(
            verbout_arg_message("umlaut", "read ", "file.p"),
            "umlaut: read file.p\n"
        );
    }

    #[test]
    fn output_helpers_write_and_flush_only_when_enabled() {
        let _guard = global_state_lock();
        set_verbose_level(0);
        let mut output = Vec::new();
        assert!(!verbout(&mut output, "umlaut", "quiet").unwrap());
        assert!(output.is_empty());

        set_verbose_level(1);
        assert!(verbout(&mut output, "umlaut", "one").unwrap());
        assert!(!verbout2(&mut output, "umlaut", "two").unwrap());
        assert!(!verbout10(&mut output, "umlaut", "ten").unwrap());
        assert!(verbout_arg(&mut output, "umlaut", "arg", "1").unwrap());

        set_verbose_level(2);
        assert!(verbout2(&mut output, "umlaut", "two").unwrap());
        assert!(verbout_arg2(&mut output, "umlaut", "arg", "2").unwrap());

        set_verbose_level(10);
        assert!(verbout10(&mut output, "umlaut", "ten").unwrap());
        set_verbose_level(0);

        assert_eq!(
            String::from_utf8(output).unwrap(),
            "umlaut: oneumlaut: arg1\numlaut: twoumlaut: arg2\numlaut: ten"
        );
    }

    #[test]
    fn global_program_name_helpers_match_c_progname_closure() {
        let _guard = global_state_lock();
        init_error("umlaut-global");
        set_verbose_level(0);
        let mut output = Vec::new();

        assert!(!verbout_global_to(&mut output, "quiet").unwrap());
        set_verbose_level(1);
        assert!(verbout_global_to(&mut output, "one").unwrap());
        assert!(!verbout2_global_to(&mut output, "two").unwrap());
        assert!(verbout_arg_global_to(&mut output, "arg", "1").unwrap());

        set_verbose_level(2);
        assert!(verbout2_global_to(&mut output, "two").unwrap());
        assert!(verbout_arg2_global_to(&mut output, "arg", "2").unwrap());

        set_verbose_level(10);
        assert!(verbout10_global_to(&mut output, "ten").unwrap());

        assert_eq!(
            String::from_utf8(output).unwrap(),
            "umlaut-global: oneumlaut-global: arg1\numlaut-global: twoumlaut-global: arg2\numlaut-global: ten"
        );

        init_error("Unknown program");
        set_verbose_level(0);
    }
}
