use crate::basics::error::{Diagnostic, ErrorCode};
use crate::inout::commandline::{
    get_float_arg, get_int_arg, print_options, CommandLineState, OptArgType, OptCell,
};
use std::io::{Read, Write};

pub const PROGRAM_NAME: &str = "ex_commandline";
const VERSION: &str = "1.0 Tue Jan 20 00:35:40 MET 1998";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OptionCode {
    Help,
    IntExample,
    FloatExample,
}

const OPTIONS: &[OptCell<OptionCode>] = &[
    OptCell::new(
        OptionCode::Help,
        Some('h'),
        Some("help"),
        OptArgType::NoArg,
        None,
        "Print a short description of program usage and options.",
    ),
    OptCell::new(
        OptionCode::IntExample,
        Some('i'),
        Some("int_example"),
        OptArgType::ReqArg,
        Some("1"),
        "Print the value given with the option..",
    ),
    OptCell::new(
        OptionCode::FloatExample,
        Some('f'),
        Some("float_example"),
        OptArgType::OptArg,
        Some("3.1415"),
        "Print the given argument or a default value.",
    ),
];

pub fn run<I, S>(
    argv: I,
    _stdin: &mut impl Read,
    stdout: &mut impl Write,
    _stderr: &mut impl Write,
) -> Result<u8, Diagnostic>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    match process_options(argv, stdout)? {
        RunCommand::Exit(status) => Ok(status),
        RunCommand::Execute(files) => {
            for file in files {
                writeln_diag(stdout, &format!("File to process: {file}"))?;
            }
            Ok(0)
        }
    }
}

enum RunCommand {
    Execute(Vec<String>),
    Exit(u8),
}

fn process_options<I, S>(argv: I, stdout: &mut impl Write) -> Result<RunCommand, Diagnostic>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut state = CommandLineState::new(argv);

    while let Some(parsed) = state.next_opt(OPTIONS)? {
        match parsed.option().option_code {
            OptionCode::Help => {
                write_all(stdout, print_help().as_bytes())?;
                return Ok(RunCommand::Exit(0));
            }
            OptionCode::IntExample => {
                let value = get_int_arg(parsed.option(), parsed.arg().unwrap_or(""))?;
                writeln_diag(stdout, &format!("Integer option has value {value}"))?;
            }
            OptionCode::FloatExample => {
                let value = get_float_arg(parsed.option(), parsed.arg().unwrap_or(""))?;
                writeln_diag(stdout, &format!("Float option has value {value:.6}"))?;
            }
        }
    }

    let mut files = state.remaining_args().to_vec();
    if files.is_empty() {
        files.push("-".to_owned());
    }
    Ok(RunCommand::Execute(files))
}

#[must_use]
pub fn print_help() -> String {
    let mut result = format!(
        "\n\
\n\
ex_commandline.c {VERSION}\n\
\n\
Usage: ex_commandline [options] [files]\n\
\n\
Shows the usage of options, print non-option commandline arguments.\n\
\n"
    );
    result.push_str(&print_options(OPTIONS, Some("Options\n\n")));
    result
}

fn write_all(output: &mut impl Write, bytes: &[u8]) -> Result<(), Diagnostic> {
    output.write_all(bytes).map_err(|error| {
        Diagnostic::new(
            ErrorCode::FILE_ERROR,
            format!("Cannot write output: {error}"),
        )
    })
}

fn writeln_diag(output: &mut impl Write, line: &str) -> Result<(), Diagnostic> {
    write_all(output, line.as_bytes())?;
    write_all(output, b"\n")
}

#[cfg(test)]
mod tests {
    use super::{print_help, run, PROGRAM_NAME};
    use crate::basics::error::ErrorCode;
    use std::io::Cursor;

    const EXPECTED_HELP: &str = concat!(
        "\n",
        "\n",
        "ex_commandline.c 1.0 Tue Jan 20 00:35:40 MET 1998\n",
        "\n",
        "Usage: ex_commandline [options] [files]\n",
        "\n",
        "Shows the usage of options, print non-option commandline arguments.\n",
        "\n",
        "Options\n",
        "\n",
        "   -h\n",
        "  --help\n",
        "    Print a short description of program usage and options.\n",
        "\n",
        "   -i <arg>\n",
        "  --int_example=<arg>\n",
        "    Print the value given with the option..\n",
        "\n",
        "   -f\n",
        "  --float_example[=<arg>]\n",
        "    Print the given argument or a default value. The short form or the long\n",
        "    form without the optional argument is equivalent to\n",
        "    --float_example=3.1415.\n",
        "\n",
    );

    #[test]
    fn help_exits_before_file_defaults() {
        let mut stdin = Cursor::new(Vec::new());
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [PROGRAM_NAME, "--help"],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .expect("help succeeds");

        assert_eq!(status, 0);
        let output = String::from_utf8(stdout).expect("help is utf8");
        assert_eq!(output, EXPECTED_HELP);
        assert!(stderr.is_empty());
    }

    #[test]
    fn no_files_defaults_to_stdin_marker() {
        let mut stdin = Cursor::new(Vec::new());
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status =
            run([PROGRAM_NAME], &mut stdin, &mut stdout, &mut stderr).expect("run succeeds");

        assert_eq!(status, 0);
        assert_eq!(
            String::from_utf8(stdout).expect("output is utf8"),
            "File to process: -\n"
        );
        assert!(stderr.is_empty());
    }

    #[test]
    fn options_print_values_before_remaining_files() {
        let mut stdin = Cursor::new(Vec::new());
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                PROGRAM_NAME,
                "--int_example=42",
                "--float_example",
                "one.p",
                "two.p",
            ],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .expect("run succeeds");

        assert_eq!(status, 0);
        assert_eq!(
            String::from_utf8(stdout).expect("output is utf8"),
            "Integer option has value 42\n\
Float option has value 3.141500\n\
File to process: one.p\n\
File to process: two.p\n"
        );
        assert!(stderr.is_empty());
    }

    #[test]
    fn required_int_argument_is_validated() {
        let mut stdin = Cursor::new(Vec::new());
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let error = run(
            [PROGRAM_NAME, "--int_example=bad"],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .expect_err("bad int is rejected");

        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert!(error.message().contains("expects integer"));
        assert!(stdout.is_empty());
        assert!(stderr.is_empty());
    }

    #[test]
    fn explicit_float_argument_uses_c_six_decimal_shape() {
        let mut stdin = Cursor::new(Vec::new());
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [PROGRAM_NAME, "--float_example=2.5"],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .expect("run succeeds");

        assert_eq!(status, 0);
        assert_eq!(
            String::from_utf8(stdout).expect("output is utf8"),
            "Float option has value 2.500000\nFile to process: -\n"
        );
        assert!(stderr.is_empty());
    }

    #[test]
    fn help_text_preserves_c_banner() {
        let rendered = print_help();

        assert_eq!(rendered, EXPECTED_HELP);
    }
}
