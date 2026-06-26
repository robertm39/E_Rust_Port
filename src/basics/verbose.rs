use std::io::{self, Write};
use std::sync::atomic::{AtomicI32, Ordering};

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

#[cfg(test)]
mod tests {
    use super::{
        set_verbose_level, verbose, verbose10, verbose10_enabled, verbose2, verbose2_enabled,
        verbose_enabled, verbose_level, verbout, verbout10, verbout2, verbout_arg, verbout_arg2,
        verbout_arg_message, verbout_message,
    };
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
        assert_eq!(verbout_message("eprover", "working"), "eprover: working");
        assert_eq!(
            verbout_arg_message("eprover", "read ", "file.p"),
            "eprover: read file.p\n"
        );
    }

    #[test]
    fn output_helpers_write_and_flush_only_when_enabled() {
        let _guard = global_state_lock();
        set_verbose_level(0);
        let mut output = Vec::new();
        assert!(!verbout(&mut output, "eprover", "quiet").unwrap());
        assert!(output.is_empty());

        set_verbose_level(1);
        assert!(verbout(&mut output, "eprover", "one").unwrap());
        assert!(!verbout2(&mut output, "eprover", "two").unwrap());
        assert!(!verbout10(&mut output, "eprover", "ten").unwrap());
        assert!(verbout_arg(&mut output, "eprover", "arg", "1").unwrap());

        set_verbose_level(2);
        assert!(verbout2(&mut output, "eprover", "two").unwrap());
        assert!(verbout_arg2(&mut output, "eprover", "arg", "2").unwrap());

        set_verbose_level(10);
        assert!(verbout10(&mut output, "eprover", "ten").unwrap());
        set_verbose_level(0);

        assert_eq!(
            String::from_utf8(output).unwrap(),
            "eprover: oneeprover: arg1\neprover: twoeprover: arg2\neprover: ten"
        );
    }
}
