//! Deduction-server interactive command surface from `cco_einteractive_mode`.

use std::{
    ffi::OsStr,
    fmt::Write as _,
    fs,
    io::{self, BufRead, Read, Write},
    path::Path,
};

use crate::basics::dstrings::DynamicString;
use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::simple_stuff::ProblemType;
use crate::clauses::{clausesets::ClauseSet, formulasets::FormulaSet};
use crate::control::batch_spec::{
    BatchCompletedRunner, BatchProblemData, BatchProcessProblemConfig, BatchProcessProblemOutputs,
    BatchProcessProblemReport, BatchRunnerBackend, BatchRunnerProblemConfig, BatchRunnerRequest,
    BatchSpawnedRunner, BatchSpec,
};
use crate::control::sine::StructFofSpec;
use crate::inout::network::{tcp_string_recv_from_or_error, tcp_string_send_to_or_error};
use crate::inout::scanner::{IoFormat, Scanner, TokenType};
use crate::inout::simplestuff::{read_text_block, tcp_read_text_block_from};
use crate::terms::signature::Signature;
use crate::terms::termbanks::TermBank;

pub const STAGE_COMMAND: &str = "STAGE";
pub const UNSTAGE_COMMAND: &str = "UNSTAGE";
pub const REMOVE_COMMAND: &str = "REMOVE";
pub const DOWNLOAD_COMMAND: &str = "DOWNLOAD";
pub const ADD_COMMAND: &str = "ADD";
pub const LOAD_COMMAND: &str = "LOAD";
pub const RUN_COMMAND: &str = "RUN";
pub const LIST_COMMAND: &str = "LIST";
pub const HELP_COMMAND: &str = "HELP";
pub const QUIT_COMMAND: &str = "QUIT";
pub const END_OF_BLOCK_TOKEN: &str = "GO\n";

pub const OK_SUCCESS_MESSAGE: &str = "200 ok : success\n";
pub const OK_STAGED_MESSAGE: &str = "201 ok : staged\n";
pub const OK_UNSTAGED_MESSAGE: &str = "202 ok : unstaged\n";
pub const OK_REMOVED_MESSAGE: &str = "203 ok : removed\n";
pub const OK_DOWNLOADED_MESSAGE: &str = "204 ok : downloaded\n";
pub const OK_ADDED_MESSAGE: &str = "205 ok : added\n";
pub const OK_LOADED_MESSAGE: &str = "206 ok : loaded\n";

pub const ERR_ERROR_MESSAGE: &str = "499 Err : Something went wrong\n";
pub const ERR_AXIOM_SET_NAME_TAKEN_MESSAGE: &str = "401 Err : axiom set name is taken\n";
pub const ERR_SYNTAX_ERROR_MESSAGE: &str = "402 Err : syntax error\n";
pub const ERR_AXIOM_SET_IS_STAGED_MESSAGE: &str =
    "403 Err : axiom set is staged, please unstage it first\n";
pub const ERR_UNKNOWN_AXIOM_SET_MESSAGE: &str = "404 Err : unknown axiom set\n";
pub const ERR_AXIOM_SET_IS_ALREADY_STAGED_MESSAGE: &str = "405 Err : axiom set is already staged\n";
pub const ERR_AXIOM_SET_IS_ALREADY_UNSTAGED_MESSAGE: &str =
    "406 Err : axiom set is already unstaged\n";
pub const ERR_UNKNOWN_COMMAND_MESSAGE: &str = "407 Err : unknown command\n";
pub const ERR_NO_AXIOM_LIBRARY_ON_SERVER_MESSAGE: &str = "408 Err : no axioms library on server\n";
pub const ERR_CANNOT_READ_SERVER_LIBRARY_MESSAGE: &str = "409 Err : cannot read server library\n";

pub const HELP_MESSAGE: &str = "\
% Note : Block commands that are of the form of \"COMMAND <NAME> ... GO\"\n\
% should have the \"COMMAND <NAME>\" and GO each on a separate line of\n\
% their own. The block should be in between these two.\n\
%\n\
%- ADD <NAME> ... GO : Uploads a new axiom set with the name <NAME>.\n\
%- LOAD <NAME>       : Loads a server-side axiom set with the name <NAME>. \n\
%- STAGE <NAME>      : Stages the axiom set <NAME>.\n\
%- UNSTAGE <NAME>    : Unstages the axiom set <NAME>.\n\
%- REMOVE <NAME>     : Removes the axiom set <NAME> from the memory.\n\
%- DOWNLOAD <NAME>   : Prints the axiom set <NAME>.\n\
%- RUN <NAME> ... GO : Runs a job with the name <NAME>.\n\
%- LIST              : Prints the status of the axiom sets.\n\
%- HELP              : Prints the help message.\n\
%- QUIT              : Closes the connection with the server.\n";

#[derive(Clone, Debug, PartialEq)]
pub struct AxiomSet {
    cset: ClauseSet,
    fset: FormulaSet,
    problem_type: ProblemType,
    staged: bool,
    raw_data: String,
}

impl AxiomSet {
    /// C `AxiomSetAlloc`.
    ///
    /// The `staged` parameter is intentionally ignored because the C allocator
    /// always initializes `handle->staged = 0`.
    #[must_use]
    pub fn new(
        cset: ClauseSet,
        fset: FormulaSet,
        raw_data: impl Into<String>,
        staged: bool,
    ) -> Self {
        Self::new_with_problem_type(cset, fset, raw_data, staged, ProblemType::FirstOrder)
    }

    #[must_use]
    pub fn new_with_problem_type(
        cset: ClauseSet,
        fset: FormulaSet,
        raw_data: impl Into<String>,
        staged: bool,
        problem_type: ProblemType,
    ) -> Self {
        let _ = staged;
        Self {
            cset,
            fset,
            problem_type,
            staged: false,
            raw_data: raw_data.into(),
        }
    }

    #[must_use]
    pub fn name(&self) -> &str {
        self.cset.identifier()
    }

    #[must_use]
    pub fn clause_set(&self) -> &ClauseSet {
        &self.cset
    }

    #[must_use]
    pub fn formula_set(&self) -> &FormulaSet {
        &self.fset
    }

    #[must_use]
    pub const fn problem_type(&self) -> ProblemType {
        self.problem_type
    }

    #[must_use]
    pub const fn is_staged(&self) -> bool {
        self.staged
    }

    pub const fn set_staged(&mut self, staged: bool) {
        self.staged = staged;
    }

    #[must_use]
    pub fn raw_data(&self) -> &str {
        &self.raw_data
    }
}

impl From<(String, String, BatchProblemData)> for AxiomSet {
    fn from((name, raw_data, mut problem): (String, String, BatchProblemData)) -> Self {
        problem.clauses.set_identifier(name.clone());
        problem.formulas.set_identifier(name);
        Self::new_with_problem_type(
            problem.clauses,
            problem.formulas,
            raw_data,
            false,
            problem.problem_type,
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InteractiveCommandOutput {
    pub output: String,
    pub frame_offsets: Vec<usize>,
    pub status: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InteractiveDispatchResult {
    pub output: String,
    pub frame_offsets: Vec<usize>,
    pub done: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InteractiveRunReport {
    pub command: InteractiveCommandOutput,
    pub process: BatchProcessProblemReport,
    pub global_output: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InteractiveBlockRead {
    pub input: String,
    pub complete: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InteractiveServerReport {
    pub commands: usize,
    pub done: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct InteractiveSpec {
    axiom_sets: Vec<AxiomSet>,
    server_lib: String,
}

impl InteractiveSpec {
    /// C `InteractiveSpecAlloc` surface for the state currently represented in
    /// Rust. Batch/control pointers and the output transport are wired by later
    /// command-dispatch slices.
    #[must_use]
    pub fn new(server_lib: impl Into<String>) -> Self {
        Self {
            axiom_sets: Vec::new(),
            server_lib: server_lib.into(),
        }
    }

    #[must_use]
    pub fn server_lib(&self) -> &str {
        &self.server_lib
    }

    #[must_use]
    pub fn axiom_set_count(&self) -> usize {
        self.axiom_sets.len()
    }

    pub fn axiom_sets(&self) -> impl Iterator<Item = &AxiomSet> {
        self.axiom_sets.iter()
    }

    pub fn axiom_set_mut(&mut self, name: &str) -> Option<&mut AxiomSet> {
        self.axiom_sets
            .iter_mut()
            .find(|axiom_set| axiom_set.name() == name)
    }

    /// C `add_command` duplicate-name tail, after parsing has produced clause
    /// and formula sets.
    pub fn add_axiom_set(&mut self, axiom_set: AxiomSet) -> &'static str {
        if self
            .axiom_sets
            .iter()
            .any(|handle| handle.name() == axiom_set.name())
        {
            ERR_AXIOM_SET_NAME_TAKEN_MESSAGE
        } else {
            self.axiom_sets.push(axiom_set);
            OK_ADDED_MESSAGE
        }
    }

    /// C `add_command` after the input block has already been parsed.
    pub fn add_parsed_axiom_set(
        &mut self,
        axioms_name: impl Into<String>,
        raw_data: impl Into<String>,
        problem: BatchProblemData,
    ) -> &'static str {
        self.add_axiom_set(AxiomSet::from((
            axioms_name.into(),
            raw_data.into(),
            problem,
        )))
    }

    /// C `add_command`.
    ///
    /// # Errors
    ///
    /// Returns parser diagnostics from constructing the clause/formula sets.
    pub fn add_command(
        &mut self,
        axioms_name: &str,
        input_axioms: &str,
        spec: &BatchSpec,
        bank: &mut TermBank,
        ctrl: &StructFofSpec,
    ) -> Result<&'static str, Diagnostic> {
        let problem = parse_interactive_axioms(axioms_name, input_axioms, spec, bank, ctrl)?;
        Ok(self.add_parsed_axiom_set(axioms_name, input_axioms, problem))
    }

    /// C `load_command`.
    ///
    /// # Errors
    ///
    /// Returns file-read diagnostics or parser diagnostics for the selected
    /// server-library file.
    pub fn load_command(
        &mut self,
        filename: &str,
        spec: &BatchSpec,
        bank: &mut TermBank,
        ctrl: &StructFofSpec,
    ) -> Result<&'static str, Diagnostic> {
        self.load_command_with(filename, |path, raw_data| {
            let source_name = path.to_string_lossy();
            parse_interactive_axioms(&source_name, raw_data, spec, bank, ctrl)
        })
    }

    /// C `load_command`, with parsing supplied by the caller.
    ///
    /// The parser boundary corresponds to C's `FileLoad` plus `add_command`
    /// parse step. This keeps the directory/file status behavior local while
    /// allowing the batch parser owner to provide the actual clause/formula
    /// construction.
    ///
    /// # Errors
    ///
    /// Returns file-read diagnostics or parser diagnostics for the selected
    /// server-library file.
    pub fn load_command_with<F>(
        &mut self,
        filename: &str,
        parse_axioms: F,
    ) -> Result<&'static str, Diagnostic>
    where
        F: FnOnce(&Path, &str) -> Result<BatchProblemData, Diagnostic>,
    {
        if self.server_lib.is_empty() {
            return Ok(ERR_NO_AXIOM_LIBRARY_ON_SERVER_MESSAGE);
        }

        let Some(files) = get_directory_listings(&self.server_lib) else {
            return Ok(ERR_CANNOT_READ_SERVER_LIBRARY_MESSAGE);
        };
        if !files.iter().rev().any(|handle| handle == filename) {
            return Ok(ERR_UNKNOWN_AXIOM_SET_MESSAGE);
        }

        let path = Path::new(&self.server_lib).join(filename);
        let raw_data = fs::read_to_string(&path).map_err(|error| {
            Diagnostic::new(
                ErrorCode::FILE_ERROR,
                format!("Cannot read file {}: {error}", path.display()),
            )
        })?;
        let problem = parse_axioms(&path, &raw_data)?;
        let status = self.add_parsed_axiom_set(filename, raw_data, problem);
        if status == OK_ADDED_MESSAGE {
            Ok(OK_LOADED_MESSAGE)
        } else {
            Ok(status)
        }
    }

    /// C `stage_command`.
    pub fn stage_command(
        &mut self,
        ctrl: &mut StructFofSpec,
        signature: &Signature,
        axiom_set: &str,
    ) -> &'static str {
        let Some(index) = self
            .axiom_sets
            .iter()
            .position(|handle| handle.name() == axiom_set)
        else {
            return ERR_UNKNOWN_AXIOM_SET_MESSAGE;
        };

        if self.axiom_sets[index].is_staged() {
            return ERR_AXIOM_SET_IS_ALREADY_STAGED_MESSAGE;
        }

        ctrl.add_problem_with_type(
            signature,
            self.axiom_sets[index].clause_set().clone(),
            self.axiom_sets[index].formula_set().clone(),
            false,
            self.axiom_sets[index].problem_type(),
        );
        self.axiom_sets[index].set_staged(true);
        ctrl.mark_current_problem_stack_shared();
        OK_STAGED_MESSAGE
    }

    /// C `unstage_command`.
    pub fn unstage_command(
        &mut self,
        ctrl: &mut StructFofSpec,
        signature: &Signature,
        axiom_set: &str,
    ) -> &'static str {
        let Some(index) = self
            .axiom_sets
            .iter()
            .position(|handle| handle.name() == axiom_set)
        else {
            return ERR_UNKNOWN_AXIOM_SET_MESSAGE;
        };

        if !self.axiom_sets[index].is_staged() {
            return ERR_AXIOM_SET_IS_ALREADY_UNSTAGED_MESSAGE;
        }

        self.axiom_sets[index].set_staged(false);
        if ctrl.remove_problem_by_identifier(signature, axiom_set) {
            OK_UNSTAGED_MESSAGE
        } else {
            ERR_UNKNOWN_AXIOM_SET_MESSAGE
        }
    }

    /// C `list_command`.
    #[must_use]
    pub fn list_command(&self) -> InteractiveCommandOutput {
        let mut output = String::new();

        let staged: Vec<_> = self
            .axiom_sets
            .iter()
            .filter(|handle| handle.is_staged())
            .collect();
        let unstaged: Vec<_> = self
            .axiom_sets
            .iter()
            .filter(|handle| !handle.is_staged())
            .collect();

        if !staged.is_empty() {
            output.push_str("Staged :\n");
            for handle in staged {
                let _ = writeln!(output, "  {}", handle.name());
            }
        }

        if !unstaged.is_empty() {
            output.push_str("Unstaged :\n");
            for handle in unstaged {
                let _ = writeln!(output, "  {}", handle.name());
            }
        }

        if self.axiom_sets.is_empty() {
            output.push_str("No Axiom Sets currently in memory.\n");
        }

        output.push_str("On Disk :\n");
        if self.server_lib.is_empty() {
            output.push_str("\tNo axioms directory was specified on server startup.\n");
        } else if let Some(files) = get_directory_listings(&self.server_lib) {
            for file in files.iter().rev() {
                let _ = writeln!(output, "\t{file}");
            }
        } else {
            output.push_str("\tCould not open current directory.\n");
        }

        let frame_offsets = vec![output.len()];
        InteractiveCommandOutput {
            output,
            frame_offsets,
            status: OK_SUCCESS_MESSAGE,
        }
    }

    /// C `download_command`.
    #[must_use]
    pub fn download_command(&self, axiom_set: &str) -> InteractiveCommandOutput {
        self.axiom_sets
            .iter()
            .find(|handle| handle.name() == axiom_set)
            .map_or(
                InteractiveCommandOutput {
                    output: String::new(),
                    frame_offsets: Vec::new(),
                    status: ERR_UNKNOWN_AXIOM_SET_MESSAGE,
                },
                |handle| {
                    let output = handle.raw_data().to_owned();
                    let frame_offsets = vec![output.len()];
                    InteractiveCommandOutput {
                        output,
                        frame_offsets,
                        status: OK_DOWNLOADED_MESSAGE,
                    }
                },
            )
    }

    /// C `remove_command`, including its stack-pop side effects on staged-set
    /// errors.
    pub fn remove_command(&mut self, axiom_set: &str) -> &'static str {
        let mut spare_stack = Vec::new();
        let mut found = false;

        while let Some(handle) = self.axiom_sets.pop() {
            if handle.name() == axiom_set {
                if handle.is_staged() {
                    return ERR_AXIOM_SET_IS_STAGED_MESSAGE;
                }
                found = true;
                break;
            }
            spare_stack.push(handle);
        }

        while let Some(handle) = spare_stack.pop() {
            self.axiom_sets.push(handle);
        }

        if found {
            OK_REMOVED_MESSAGE
        } else {
            ERR_UNKNOWN_AXIOM_SET_MESSAGE
        }
    }

    /// C `quit_command`: unstage every currently staged axiom set before the
    /// connection closes.
    pub fn quit_command(&mut self, ctrl: &mut StructFofSpec, signature: &Signature) {
        let staged_names: Vec<_> = self
            .axiom_sets
            .iter()
            .filter(|handle| handle.is_staged())
            .map(|handle| handle.name().to_owned())
            .collect();

        for name in staged_names.into_iter().rev() {
            let _ = self.unstage_command(ctrl, signature, &name);
        }
    }

    /// C `StartDeductionServer` single-message command dispatch, with the
    /// transport-specific block reader and `RUN` implementation supplied by
    /// the caller.
    ///
    /// # Errors
    ///
    /// Returns scanner diagnostics, parser diagnostics from `ADD`/`LOAD`, or
    /// caller-supplied diagnostics from block reading and `RUN`.
    pub fn dispatch_command_with<B, R>(
        &mut self,
        command_input: &str,
        spec: &BatchSpec,
        bank: &mut TermBank,
        ctrl: &mut StructFofSpec,
        mut read_block: B,
        mut run_command: R,
    ) -> Result<InteractiveDispatchResult, Diagnostic>
    where
        B: FnMut(&str) -> Result<String, Diagnostic>,
        R: FnMut(
            &BatchSpec,
            &mut TermBank,
            &mut StructFofSpec,
            &str,
            &str,
        ) -> Result<InteractiveCommandOutput, Diagnostic>,
    {
        let mut scanner = Scanner::from_user_string(command_input, true)?;
        scanner.set_format(IoFormat::Tstp);

        if scanner.test_id(STAGE_COMMAND) {
            scanner.accept_id(STAGE_COMMAND)?;
            let axiom_set = accept_command_axiom_set_name(&mut scanner)?;
            Ok(dispatch_status(
                self.stage_command(ctrl, bank.signature(), &axiom_set),
                false,
            ))
        } else if scanner.test_id(UNSTAGE_COMMAND) {
            scanner.accept_id(UNSTAGE_COMMAND)?;
            let axiom_set = accept_command_axiom_set_name(&mut scanner)?;
            Ok(dispatch_status(
                self.unstage_command(ctrl, bank.signature(), &axiom_set),
                false,
            ))
        } else if scanner.test_id(REMOVE_COMMAND) {
            scanner.accept_id(REMOVE_COMMAND)?;
            let axiom_set = accept_command_axiom_set_name(&mut scanner)?;
            Ok(dispatch_status(self.remove_command(&axiom_set), false))
        } else if scanner.test_id(DOWNLOAD_COMMAND) {
            scanner.accept_id(DOWNLOAD_COMMAND)?;
            let axiom_set = accept_command_axiom_set_name(&mut scanner)?;
            Ok(dispatch_command_output(
                self.download_command(&axiom_set),
                false,
            ))
        } else if scanner.test_id(LOAD_COMMAND) {
            scanner.accept_id(LOAD_COMMAND)?;
            let axiom_set = accept_command_axiom_set_name(&mut scanner)?;
            Ok(dispatch_status(
                self.load_command(&axiom_set, spec, bank, &*ctrl)?,
                false,
            ))
        } else if scanner.test_id(ADD_COMMAND) {
            scanner.accept_id(ADD_COMMAND)?;
            let axiom_set = accept_command_axiom_set_name(&mut scanner)?;
            let input_axioms = read_block(END_OF_BLOCK_TOKEN)?;
            Ok(dispatch_status(
                self.add_command(&axiom_set, &input_axioms, spec, bank, &*ctrl)?,
                false,
            ))
        } else if scanner.test_id(RUN_COMMAND) {
            scanner.accept_id(RUN_COMMAND)?;
            let job_name = scanner.current_token().literal();
            scanner.accept_tok(TokenType::IDENTIFIER)?;
            let input_axioms = read_block(END_OF_BLOCK_TOKEN)?;
            Ok(dispatch_command_output(
                run_command(spec, bank, ctrl, &job_name, &input_axioms)?,
                false,
            ))
        } else if scanner.test_id(LIST_COMMAND) {
            scanner.accept_id(LIST_COMMAND)?;
            Ok(dispatch_command_output(self.list_command(), false))
        } else if scanner.test_id(HELP_COMMAND) {
            scanner.accept_id(HELP_COMMAND)?;
            let output = format!("{HELP_MESSAGE}{OK_SUCCESS_MESSAGE}");
            Ok(InteractiveDispatchResult {
                frame_offsets: vec![HELP_MESSAGE.len(), output.len()],
                output,
                done: false,
            })
        } else if scanner.test_id(QUIT_COMMAND) {
            scanner.accept_id(QUIT_COMMAND)?;
            self.quit_command(ctrl, bank.signature());
            Ok(InteractiveDispatchResult {
                output: String::new(),
                frame_offsets: Vec::new(),
                done: true,
            })
        } else {
            Ok(dispatch_status(ERR_UNKNOWN_COMMAND_MESSAGE, false))
        }
    }

    /// Single-message dispatch using C `ReadTextBlock`-style line input for
    /// block commands.
    ///
    /// # Errors
    ///
    /// Returns scanner, parser, text-block I/O, or caller-supplied `RUN`
    /// diagnostics.
    pub fn dispatch_text_command_with<B, R>(
        &mut self,
        command_input: &str,
        block_reader: &mut B,
        spec: &BatchSpec,
        bank: &mut TermBank,
        ctrl: &mut StructFofSpec,
        run_command: R,
    ) -> Result<InteractiveDispatchResult, Diagnostic>
    where
        B: BufRead,
        R: FnMut(
            &BatchSpec,
            &mut TermBank,
            &mut StructFofSpec,
            &str,
            &str,
        ) -> Result<InteractiveCommandOutput, Diagnostic>,
    {
        self.dispatch_command_with(
            command_input,
            spec,
            bank,
            ctrl,
            |terminator| Ok(read_interactive_text_block(block_reader, terminator)?.input),
            run_command,
        )
    }

    /// Single-message dispatch using C `TCPReadTextBlock`-style TCP message
    /// input for block commands.
    ///
    /// # Errors
    ///
    /// Returns scanner, parser, TCP block-read, or caller-supplied `RUN`
    /// diagnostics.
    pub fn dispatch_tcp_command_with<B, R>(
        &mut self,
        command_input: &str,
        block_reader: &mut B,
        spec: &BatchSpec,
        bank: &mut TermBank,
        ctrl: &mut StructFofSpec,
        run_command: R,
    ) -> Result<InteractiveDispatchResult, Diagnostic>
    where
        B: Read,
        R: FnMut(
            &BatchSpec,
            &mut TermBank,
            &mut StructFofSpec,
            &str,
            &str,
        ) -> Result<InteractiveCommandOutput, Diagnostic>,
    {
        self.dispatch_command_with(
            command_input,
            spec,
            bank,
            ctrl,
            |terminator| Ok(read_interactive_tcp_block(block_reader, terminator)?.input),
            run_command,
        )
    }
}

/// C `StartDeductionServer` socket-message loop, with the `RUN` backend
/// supplied by the caller.
///
/// The C implementation has a file/stdout parameter, but exits immediately
/// for that path. This adapter covers the real socket branch: receive one TCP
/// string command at a time, use TCP text-block reads for `ADD`/`RUN`, and send
/// each command response using the same TCP-string frame boundaries as C.
///
/// # Errors
///
/// Returns TCP receive/send diagnostics or command/parser diagnostics.
pub fn start_deduction_server_tcp_with<S, R>(
    stream: &mut S,
    server_lib: impl Into<String>,
    spec: &BatchSpec,
    bank: &mut TermBank,
    ctrl: &mut StructFofSpec,
    mut run_command: R,
) -> Result<InteractiveServerReport, Diagnostic>
where
    S: Read + Write,
    R: FnMut(
        &BatchSpec,
        &mut TermBank,
        &mut StructFofSpec,
        &str,
        &str,
    ) -> Result<InteractiveCommandOutput, Diagnostic>,
{
    let mut interactive = InteractiveSpec::new(server_lib);
    let mut report = InteractiveServerReport {
        commands: 0,
        done: false,
    };

    while !report.done {
        let command = tcp_string_recv_from_or_error(stream)?;
        let result = interactive.dispatch_tcp_command_with(
            &command,
            stream,
            spec,
            bank,
            ctrl,
            &mut run_command,
        )?;
        report.commands += 1;
        report.done = result.done;
        let mut start = 0;
        for end in result.frame_offsets {
            let frame = result.output.get(start..end).ok_or_else(|| {
                Diagnostic::new(
                    ErrorCode::OTHER_ERROR,
                    "Invalid interactive TCP frame boundary",
                )
            })?;
            tcp_string_send_to_or_error(stream, frame)?;
            start = end;
        }
        if start != result.output.len() {
            return Err(Diagnostic::new(
                ErrorCode::OTHER_ERROR,
                "Interactive TCP frame boundaries did not cover the complete response",
            ));
        }
    }

    Ok(report)
}

/// C `ReadTextBlock` adapter for interactive `ADD`/`RUN` payloads.
///
/// # Errors
///
/// Returns an I/O diagnostic when reading fails or an output conversion
/// diagnostic if the captured text is not UTF-8.
pub fn read_interactive_text_block(
    reader: &mut impl BufRead,
    terminator: &str,
) -> Result<InteractiveBlockRead, Diagnostic> {
    let mut result = DynamicString::new();
    let complete = read_text_block(&mut result, reader, terminator.as_bytes())
        .map_err(|error| interactive_block_io_error(&error))?;
    Ok(InteractiveBlockRead {
        input: dynamic_string_to_string(&result)?,
        complete,
    })
}

/// C `TCPReadTextBlock` adapter for interactive `ADD`/`RUN` payloads.
///
/// # Errors
///
/// Returns TCP receive diagnostics or an output conversion diagnostic if the
/// captured text is not UTF-8.
pub fn read_interactive_tcp_block(
    reader: &mut impl Read,
    terminator: &str,
) -> Result<InteractiveBlockRead, Diagnostic> {
    let mut result = DynamicString::new();
    let complete = tcp_read_text_block_from(&mut result, reader, terminator.as_bytes())?;
    Ok(InteractiveBlockRead {
        input: dynamic_string_to_string(&result)?,
        complete,
    })
}

/// C `run_command`, staged over injectable runner spawning and polling.
///
/// The C implementation forks before parsing the job and writes progress to
/// the connection from the child while the parent later returns
/// `OK_SUCCESS_MESSAGE`. This Rust helper performs the same logical work
/// synchronously and returns captured connection/global output so transport
/// owners can decide where to write it.
///
/// # Errors
///
/// Returns parser diagnostics or batch runner diagnostics.
#[expect(
    clippy::too_many_arguments,
    reason = "The interactive RUN port must thread parser, control, clock, and runner hooks"
)]
pub fn run_command_with<C, S, P>(
    job_name: &str,
    input_axioms: &str,
    spec: &BatchSpec,
    bank: &mut TermBank,
    ctrl: &mut StructFofSpec,
    clock_seconds: C,
    spawn_runner: S,
    poll_runners: P,
) -> Result<InteractiveRunReport, Diagnostic>
where
    C: FnMut() -> i64,
    S: FnMut(BatchRunnerRequest) -> Result<BatchSpawnedRunner, Diagnostic>,
    P: FnMut(&mut Vec<BatchSpawnedRunner>) -> Result<Option<BatchCompletedRunner>, Diagnostic>,
{
    let mut global_output = Vec::new();
    global_output.extend_from_slice(job_name.as_bytes());
    let mut outstream_output = InteractiveFrameWriter::default();
    outstream_output.push_frame(format!("\n% Processing started for {job_name}\n").as_bytes());

    let problem = parse_interactive_axioms(job_name, input_axioms, spec, bank, ctrl)?;
    let mut ignored_external_output = Vec::new();
    let process = spec.process_problem_with(
        bank.signature(),
        ctrl,
        problem,
        BatchProcessProblemConfig {
            wct_limit: run_command_wct_limit(spec),
            jobname: job_name,
            interactive: true,
        },
        BatchProcessProblemOutputs {
            global_output: &mut global_output,
            external_output: Some(&mut ignored_external_output),
            socket_output: Some(&mut outstream_output),
        },
        clock_seconds,
        spawn_runner,
        poll_runners,
    )?;
    outstream_output.push_frame(format!("\n% Processing finished for {job_name}\n\n").as_bytes());
    let (outstream_output, frame_offsets) = outstream_output.into_string_parts()?;

    Ok(InteractiveRunReport {
        command: InteractiveCommandOutput {
            output: outstream_output,
            frame_offsets,
            status: OK_SUCCESS_MESSAGE,
        },
        process,
        global_output: bytes_to_string(global_output)?,
    })
}

/// C `run_command` over the concrete temp-file runner backend.
///
/// This keeps executable integrations on the same parsed-command path as the
/// injectable `run_command_with` helper while still rendering the selected
/// problem to the temporary file expected by `EPCtrlCreateGeneric`.
///
/// # Errors
///
/// Returns parser diagnostics or batch runner diagnostics.
pub fn run_command_with_runner_backend<C, B>(
    job_name: &str,
    input_axioms: &str,
    spec: &BatchSpec,
    bank: &mut TermBank,
    ctrl: &mut StructFofSpec,
    clock_seconds: C,
    backend: &mut B,
) -> Result<InteractiveRunReport, Diagnostic>
where
    C: FnMut() -> i64,
    B: BatchRunnerBackend,
{
    let mut global_output = Vec::new();
    global_output.extend_from_slice(job_name.as_bytes());
    let mut outstream_output = InteractiveFrameWriter::default();
    outstream_output.push_frame(format!("\n% Processing started for {job_name}\n").as_bytes());

    let problem = parse_interactive_axioms(job_name, input_axioms, spec, bank, ctrl)?;
    let mut ignored_external_output = Vec::new();
    let process = spec.process_problem_with_runner_backend(
        bank,
        ctrl,
        problem,
        BatchProcessProblemConfig {
            wct_limit: run_command_wct_limit(spec),
            jobname: job_name,
            interactive: true,
        },
        BatchRunnerProblemConfig::default(),
        BatchProcessProblemOutputs {
            global_output: &mut global_output,
            external_output: Some(&mut ignored_external_output),
            socket_output: Some(&mut outstream_output),
        },
        clock_seconds,
        backend,
    )?;
    outstream_output.push_frame(format!("\n% Processing finished for {job_name}\n\n").as_bytes());
    let (outstream_output, frame_offsets) = outstream_output.into_string_parts()?;

    Ok(InteractiveRunReport {
        command: InteractiveCommandOutput {
            output: outstream_output,
            frame_offsets,
            status: OK_SUCCESS_MESSAGE,
        },
        process,
        global_output: bytes_to_string(global_output)?,
    })
}

/// C `AXIOM_SET_NAME_TOKENS`.
#[must_use]
pub fn axiom_set_name_tokens() -> TokenType {
    TokenType::STRING
        | TokenType::NAME
        | TokenType::POS_INT
        | TokenType::FULLSTOP
        | TokenType::PLUS
        | TokenType::HYPHEN
}

/// C `AcceptAxiomSetName`: append every current axiom-name token and stop at
/// the first token outside `AXIOM_SET_NAME_TOKENS`.
///
/// Unlike filename parsing helpers, the C loop uses ordinary token tests, so
/// whitespace between accepted tokens is allowed by the scanner and omitted
/// from the destination.
///
/// # Errors
///
/// Returns scanner diagnostics when advancing to the next token fails.
pub fn accept_axiom_set_name(scanner: &mut Scanner, dest: &mut String) -> Result<(), Diagnostic> {
    while scanner.test_tok(axiom_set_name_tokens()) {
        dest.push_str(&scanner.current_token().literal());
        scanner.next_token()?;
    }
    Ok(())
}

fn accept_command_axiom_set_name(scanner: &mut Scanner) -> Result<String, Diagnostic> {
    let mut axiom_set = String::new();
    accept_axiom_set_name(scanner, &mut axiom_set)?;
    Ok(axiom_set)
}

fn dispatch_status(status: &'static str, done: bool) -> InteractiveDispatchResult {
    InteractiveDispatchResult {
        output: status.to_owned(),
        frame_offsets: vec![status.len()],
        done,
    }
}

fn dispatch_command_output(
    result: InteractiveCommandOutput,
    done: bool,
) -> InteractiveDispatchResult {
    let mut output = result.output;
    let mut frame_offsets = result.frame_offsets;
    output.push_str(result.status);
    frame_offsets.push(output.len());
    InteractiveDispatchResult {
        output,
        frame_offsets,
        done,
    }
}

#[derive(Debug, Default)]
struct InteractiveFrameWriter {
    bytes: Vec<u8>,
    frame_offsets: Vec<usize>,
}

impl InteractiveFrameWriter {
    fn push_frame(&mut self, frame: &[u8]) {
        self.bytes.extend_from_slice(frame);
        self.frame_offsets.push(self.bytes.len());
    }

    fn into_string_parts(self) -> Result<(String, Vec<usize>), Diagnostic> {
        Ok((bytes_to_string(self.bytes)?, self.frame_offsets))
    }
}

impl Write for InteractiveFrameWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.bytes.extend_from_slice(buffer);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.frame_offsets.push(self.bytes.len());
        Ok(())
    }
}

const fn run_command_wct_limit(spec: &BatchSpec) -> i64 {
    if spec.per_prob_limit != 0 {
        spec.per_prob_limit
    } else {
        30
    }
}

fn bytes_to_string(bytes: Vec<u8>) -> Result<String, Diagnostic> {
    String::from_utf8(bytes).map_err(|error| {
        Diagnostic::new(
            ErrorCode::OTHER_ERROR,
            format!("Interactive command output was not UTF-8: {error}"),
        )
    })
}

fn dynamic_string_to_string(value: &DynamicString) -> Result<String, Diagnostic> {
    bytes_to_string(value.copy())
}

fn interactive_block_io_error(error: &io::Error) -> Diagnostic {
    Diagnostic::new(
        ErrorCode::FILE_ERROR,
        format!("Could not read interactive text block: {error}"),
    )
}

/// C `get_directory_listings`: return a stack-shaped list of regular file
/// names in the directory.
///
/// The C helper returns `NULL` when `opendir()` fails, pushes names in raw
/// directory iteration order, and lets callers pop the stack. This Rust helper
/// therefore returns `None` on open failure and does not sort the resulting
/// vector.
#[must_use]
pub fn get_directory_listings(dirname: impl AsRef<Path>) -> Option<Vec<String>> {
    let entries = fs::read_dir(dirname).ok()?;
    let mut files = Vec::new();

    for entry in entries {
        let Ok(entry) = entry else {
            continue;
        };
        let file_name = entry.file_name();
        if file_name == OsStr::new(".") || file_name == OsStr::new("..") {
            continue;
        }

        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_file() {
            files.push(file_name.to_string_lossy().into_owned());
        }
    }

    Some(files)
}

fn parse_interactive_axioms(
    source_name: &str,
    input_axioms: &str,
    spec: &BatchSpec,
    bank: &mut TermBank,
    ctrl: &StructFofSpec,
) -> Result<BatchProblemData, Diagnostic> {
    let mut scanner =
        Scanner::from_file_content(source_name, input_axioms.as_bytes().to_vec(), true)?;
    scanner.set_format(IoFormat::Tstp);
    spec.load_problem_from_scanner(bank, ctrl, &mut scanner)
}

#[cfg(test)]
mod tests {
    use super::{
        accept_axiom_set_name, axiom_set_name_tokens, get_directory_listings,
        read_interactive_tcp_block, read_interactive_text_block, run_command_with,
        start_deduction_server_tcp_with, AxiomSet, InteractiveCommandOutput, InteractiveSpec,
        ADD_COMMAND, DOWNLOAD_COMMAND, END_OF_BLOCK_TOKEN, ERR_AXIOM_SET_IS_ALREADY_STAGED_MESSAGE,
        ERR_AXIOM_SET_IS_ALREADY_UNSTAGED_MESSAGE, ERR_AXIOM_SET_IS_STAGED_MESSAGE,
        ERR_AXIOM_SET_NAME_TAKEN_MESSAGE, ERR_CANNOT_READ_SERVER_LIBRARY_MESSAGE,
        ERR_NO_AXIOM_LIBRARY_ON_SERVER_MESSAGE, ERR_UNKNOWN_AXIOM_SET_MESSAGE,
        ERR_UNKNOWN_COMMAND_MESSAGE, HELP_COMMAND, HELP_MESSAGE, LIST_COMMAND, LOAD_COMMAND,
        OK_ADDED_MESSAGE, OK_DOWNLOADED_MESSAGE, OK_LOADED_MESSAGE, OK_REMOVED_MESSAGE,
        OK_STAGED_MESSAGE, OK_SUCCESS_MESSAGE, OK_UNSTAGED_MESSAGE, QUIT_COMMAND, REMOVE_COMMAND,
        RUN_COMMAND, STAGE_COMMAND, UNSTAGE_COMMAND,
    };
    use crate::basics::error::{Diagnostic, ErrorCode};
    use crate::basics::simple_stuff::{ProblemType, ProverResult};
    use crate::clauses::{clausesets::ClauseSet, formulasets::FormulaSet};
    use crate::control::batch_spec::{
        BatchCompletedRunner, BatchProblemData, BatchRunnerRequest, BatchSpawnedRunner, BatchSpec,
    };
    use crate::control::sine::StructFofSpec;
    use crate::inout::network::{tcp_string_recv_from, MsgStatus, TcpMessage};
    use crate::inout::scanner::{IoFormat, Scanner, TokenType};
    use crate::terms::{signature::Signature, termbanks::TermBank, typebanks::TypeBank};
    use std::{
        collections::BTreeSet,
        ffi::OsStr,
        fs,
        io::{self, Cursor, Read, Write},
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn scanner(source: &str) -> Scanner {
        Scanner::from_user_string(source, true).unwrap()
    }

    fn axiom_set(name: &str, raw_data: &str, staged_arg: bool) -> AxiomSet {
        let mut clauses = ClauseSet::new();
        clauses.set_identifier(name);
        let mut formulas = FormulaSet::new();
        formulas.set_identifier(name);
        AxiomSet::new(clauses, formulas, raw_data, staged_arg)
    }

    fn empty_problem() -> BatchProblemData {
        BatchProblemData {
            clauses: ClauseSet::new(),
            formulas: FormulaSet::new(),
            problem_type: ProblemType::FirstOrder,
        }
    }

    fn axiom_names(interactive: &InteractiveSpec) -> Vec<String> {
        interactive
            .axiom_sets()
            .map(|axiom_set| axiom_set.name().to_owned())
            .collect()
    }

    fn test_signature() -> Signature {
        Signature::new(TypeBank::new())
    }

    fn parser_bank() -> TermBank {
        let mut signature = Signature::new(TypeBank::new());
        signature.insert_internal_codes().unwrap();
        TermBank::new(signature).unwrap()
    }

    fn unused_block_reader(_: &str) -> Result<String, Diagnostic> {
        panic!("block reader should not run")
    }

    fn unused_run_command(
        _: &BatchSpec,
        _: &mut TermBank,
        _: &mut StructFofSpec,
        _: &str,
        _: &str,
    ) -> Result<InteractiveCommandOutput, Diagnostic> {
        panic!("run command should not run")
    }

    fn packed_tcp_messages(messages: &[&str]) -> Vec<u8> {
        let mut bytes = Vec::new();
        for message in messages {
            bytes.extend_from_slice(TcpMessage::pack(message).unwrap().content_bytes());
        }
        bytes
    }

    fn output_frames(output: &str, frame_offsets: &[usize]) -> Vec<String> {
        let mut start = 0;
        frame_offsets
            .iter()
            .map(|&end| {
                let frame = output[start..end].to_owned();
                start = end;
                frame
            })
            .collect()
    }

    #[derive(Debug)]
    struct Duplex {
        incoming: Cursor<Vec<u8>>,
        written: Vec<u8>,
    }

    impl Duplex {
        fn new(messages: &[&str]) -> Self {
            Self {
                incoming: Cursor::new(packed_tcp_messages(messages)),
                written: Vec::new(),
            }
        }

        fn written_messages(&self) -> Vec<String> {
            let mut cursor = Cursor::new(self.written.clone());
            let mut messages = Vec::new();
            loop {
                let (message, status) = tcp_string_recv_from(&mut cursor, false).unwrap();
                if status != MsgStatus::Success {
                    break;
                }
                messages.push(message.unwrap());
            }
            messages
        }
    }

    impl Read for Duplex {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            self.incoming.read(buffer)
        }
    }

    impl Write for Duplex {
        fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
            self.written.extend_from_slice(buffer);
            Ok(buffer.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    struct ScratchDir {
        path: PathBuf,
    }

    impl ScratchDir {
        fn new() -> Self {
            let mut path = std::env::temp_dir();
            path.push(format!(
                "e_rust_port_einteractive_{}_{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            fs::create_dir(&path).unwrap();
            Self { path }
        }
    }

    impl Drop for ScratchDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn command_and_response_strings_match_c_surface() {
        assert_eq!(STAGE_COMMAND, "STAGE");
        assert_eq!(UNSTAGE_COMMAND, "UNSTAGE");
        assert_eq!(REMOVE_COMMAND, "REMOVE");
        assert_eq!(DOWNLOAD_COMMAND, "DOWNLOAD");
        assert_eq!(ADD_COMMAND, "ADD");
        assert_eq!(LOAD_COMMAND, "LOAD");
        assert_eq!(RUN_COMMAND, "RUN");
        assert_eq!(LIST_COMMAND, "LIST");
        assert_eq!(HELP_COMMAND, "HELP");
        assert_eq!(QUIT_COMMAND, "QUIT");
        assert_eq!(END_OF_BLOCK_TOKEN, "GO\n");
        assert_eq!(OK_SUCCESS_MESSAGE, "200 ok : success\n");
        assert_eq!(ERR_UNKNOWN_COMMAND_MESSAGE, "407 Err : unknown command\n");
        assert!(HELP_MESSAGE.contains("%- RUN <NAME> ... GO"));
        assert!(HELP_MESSAGE
            .ends_with("%- QUIT              : Closes the connection with the server.\n"));
    }

    #[test]
    fn axiom_set_name_tokens_match_c_token_mask() {
        let mask = axiom_set_name_tokens();
        for token in [
            TokenType::STRING,
            TokenType::NAME,
            TokenType::POS_INT,
            TokenType::FULLSTOP,
            TokenType::PLUS,
            TokenType::HYPHEN,
        ] {
            assert!(mask.intersects(token));
        }
        assert!(!mask.intersects(TokenType::SLASH));
        assert!(!mask.intersects(TokenType::COMMA));
    }

    #[test]
    fn accept_axiom_set_name_appends_tokens_and_allows_whitespace() {
        let mut scanner = scanner("Alpha . 12 - Beta / tail");
        let mut name = String::new();

        accept_axiom_set_name(&mut scanner, &mut name).unwrap();

        assert_eq!(name, "Alpha.12-Beta");
        assert_eq!(scanner.current_token().kind(), TokenType::SLASH);
    }

    #[test]
    fn accept_axiom_set_name_stops_before_unaccepted_token() {
        let mut scanner = scanner("lib/name rest");
        let mut name = String::new();

        accept_axiom_set_name(&mut scanner, &mut name).unwrap();

        assert_eq!(name, "lib");
        assert_eq!(scanner.current_token().kind(), TokenType::SLASH);
    }

    #[test]
    fn accept_axiom_set_name_accepts_empty_name() {
        let mut scanner = scanner("/not-a-name");
        let mut name = String::from("prefix");

        accept_axiom_set_name(&mut scanner, &mut name).unwrap();

        assert_eq!(name, "prefix");
        assert_eq!(scanner.current_token().kind(), TokenType::SLASH);
    }

    #[test]
    fn get_directory_listings_returns_regular_file_names_only() {
        let scratch = ScratchDir::new();
        fs::write(scratch.path.join("alpha.p"), b"fof(a, axiom, p).").unwrap();
        fs::write(scratch.path.join("beta.ax"), b"fof(b, axiom, q).").unwrap();
        fs::write(scratch.path.join(".hidden"), b"fof(c, axiom, r).").unwrap();
        fs::create_dir(scratch.path.join("nested")).unwrap();

        let listings = get_directory_listings(&scratch.path).unwrap();
        let names: BTreeSet<_> = listings.into_iter().collect();

        assert_eq!(
            names,
            BTreeSet::from([
                String::from(".hidden"),
                String::from("alpha.p"),
                String::from("beta.ax")
            ])
        );
    }

    #[test]
    fn get_directory_listings_returns_none_when_directory_cannot_open() {
        let mut missing = std::env::temp_dir();
        missing.push(format!(
            "e_rust_port_einteractive_missing_{}_{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));

        assert!(get_directory_listings(missing).is_none());
    }

    #[test]
    fn axiom_set_alloc_ignores_staged_argument_and_copies_raw_data() {
        let axiom_set = axiom_set("library", "fof(a,axiom,p).\n", true);

        assert_eq!(axiom_set.name(), "library");
        assert_eq!(axiom_set.clause_set().identifier(), "library");
        assert_eq!(axiom_set.formula_set().identifier(), "library");
        assert!(!axiom_set.is_staged());
        assert_eq!(axiom_set.raw_data(), "fof(a,axiom,p).\n");
    }

    #[test]
    fn add_axiom_set_rejects_duplicate_clause_set_identifier() {
        let mut interactive = InteractiveSpec::new("");

        assert_eq!(
            interactive.add_axiom_set(axiom_set("dup", "first", false)),
            OK_ADDED_MESSAGE
        );
        assert_eq!(
            interactive.add_axiom_set(axiom_set("dup", "second", false)),
            ERR_AXIOM_SET_NAME_TAKEN_MESSAGE
        );

        assert_eq!(interactive.axiom_set_count(), 1);
        assert_eq!(interactive.download_command("dup").output, "first");
    }

    #[test]
    fn add_parsed_axiom_set_sets_identifiers_and_keeps_raw_data() {
        let mut interactive = InteractiveSpec::new("");
        let mut problem = empty_problem();
        problem.problem_type = ProblemType::HigherOrder;

        assert_eq!(
            interactive.add_parsed_axiom_set("parsed", "fof(a,axiom,p).\n", problem),
            OK_ADDED_MESSAGE
        );

        let axiom_set = interactive.axiom_sets().next().unwrap();
        assert_eq!(axiom_set.name(), "parsed");
        assert_eq!(axiom_set.formula_set().identifier(), "parsed");
        assert_eq!(axiom_set.problem_type(), ProblemType::HigherOrder);
        assert_eq!(axiom_set.raw_data(), "fof(a,axiom,p).\n");
    }

    #[test]
    fn add_parsed_axiom_set_rejects_duplicate_after_problem_is_built() {
        let mut interactive = InteractiveSpec::new("");

        assert_eq!(
            interactive.add_parsed_axiom_set("dup", "first", empty_problem()),
            OK_ADDED_MESSAGE
        );
        assert_eq!(
            interactive.add_parsed_axiom_set("dup", "second", empty_problem()),
            ERR_AXIOM_SET_NAME_TAKEN_MESSAGE
        );

        assert_eq!(interactive.axiom_set_count(), 1);
        assert_eq!(interactive.download_command("dup").output, "first");
    }

    #[test]
    fn add_command_parses_uploaded_axioms_through_batch_parser() {
        let mut bank = parser_bank();
        let ctrl = StructFofSpec::new(bank.signature());
        let spec = BatchSpec::new("umlaut", IoFormat::Tstp);
        let mut interactive = InteractiveSpec::new("");

        let status = interactive
            .add_command(
                "uploaded",
                "fof(uploaded_formula, axiom, p(a)).\n",
                &spec,
                &mut bank,
                &ctrl,
            )
            .unwrap();

        assert_eq!(status, OK_ADDED_MESSAGE);
        let axiom_set = interactive.axiom_sets().next().unwrap();
        assert_eq!(axiom_set.name(), "uploaded");
        assert_eq!(axiom_set.formula_set().cardinality(), 1);
        assert_eq!(
            axiom_set.raw_data(),
            "fof(uploaded_formula, axiom, p(a)).\n"
        );
    }

    #[test]
    fn add_command_propagates_parser_error_without_inserting() {
        let mut bank = parser_bank();
        let ctrl = StructFofSpec::new(bank.signature());
        let spec = BatchSpec::new("umlaut", IoFormat::Tstp);
        let mut interactive = InteractiveSpec::new("");

        let error = interactive
            .add_command("bad", "not a problem", &spec, &mut bank, &ctrl)
            .unwrap_err();

        assert_eq!(error.code(), ErrorCode::SYNTAX_ERROR);
        assert_eq!(interactive.axiom_set_count(), 0);
    }

    #[test]
    fn load_command_reports_missing_server_library_configuration() {
        let mut interactive = InteractiveSpec::new("");

        let status = interactive
            .load_command_with("anything.ax", |_, _| Ok(empty_problem()))
            .unwrap();

        assert_eq!(status, ERR_NO_AXIOM_LIBRARY_ON_SERVER_MESSAGE);
    }

    #[test]
    fn load_command_reports_unreadable_server_library() {
        let mut missing = std::env::temp_dir();
        missing.push(format!(
            "e_rust_port_einteractive_missing_load_dir_{}_{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let mut interactive = InteractiveSpec::new(missing.to_string_lossy());

        let status = interactive
            .load_command_with("anything.ax", |_, _| Ok(empty_problem()))
            .unwrap();

        assert_eq!(status, ERR_CANNOT_READ_SERVER_LIBRARY_MESSAGE);
    }

    #[test]
    fn load_command_reports_unknown_file_without_parsing() {
        let scratch = ScratchDir::new();
        fs::write(scratch.path.join("present.ax"), b"fof(a, axiom, p).").unwrap();
        let mut interactive = InteractiveSpec::new(scratch.path.to_string_lossy());

        let status = interactive
            .load_command_with("missing.ax", |_, _| panic!("parser should not run"))
            .unwrap();

        assert_eq!(status, ERR_UNKNOWN_AXIOM_SET_MESSAGE);
    }

    #[test]
    fn load_command_reads_file_parses_and_rewrites_added_to_loaded() {
        let scratch = ScratchDir::new();
        fs::write(scratch.path.join("lib.ax"), b"fof(a, axiom, p).\n").unwrap();
        let mut interactive = InteractiveSpec::new(scratch.path.to_string_lossy());

        let status = interactive
            .load_command_with("lib.ax", |path, raw_data| {
                assert_eq!(path.file_name().unwrap(), OsStr::new("lib.ax"));
                assert_eq!(raw_data, "fof(a, axiom, p).\n");
                Ok(empty_problem())
            })
            .unwrap();

        assert_eq!(status, OK_LOADED_MESSAGE);
        assert_eq!(interactive.axiom_set_count(), 1);
        assert_eq!(
            interactive.download_command("lib.ax").output,
            "fof(a, axiom, p).\n"
        );
    }

    #[test]
    fn load_command_returns_duplicate_name_status_from_add_command() {
        let scratch = ScratchDir::new();
        fs::write(scratch.path.join("dup.ax"), b"fof(a, axiom, p).\n").unwrap();
        let mut interactive = InteractiveSpec::new(scratch.path.to_string_lossy());
        assert_eq!(
            interactive.add_parsed_axiom_set("dup.ax", "existing", empty_problem()),
            OK_ADDED_MESSAGE
        );

        let status = interactive
            .load_command_with("dup.ax", |_, _| Ok(empty_problem()))
            .unwrap();

        assert_eq!(status, ERR_AXIOM_SET_NAME_TAKEN_MESSAGE);
        assert_eq!(interactive.download_command("dup.ax").output, "existing");
    }

    #[test]
    fn load_command_uses_concrete_batch_parser_for_server_file() {
        let scratch = ScratchDir::new();
        fs::write(
            scratch.path.join("real.ax"),
            b"cnf(watch_clause, watchlist, q(a)).\nfof(ax_formula, axiom, p(a)).\n",
        )
        .unwrap();
        let mut bank = parser_bank();
        let ctrl = StructFofSpec::new(bank.signature());
        let spec = BatchSpec::new("umlaut", IoFormat::Tstp);
        let mut interactive = InteractiveSpec::new(scratch.path.to_string_lossy());

        let status = interactive
            .load_command("real.ax", &spec, &mut bank, &ctrl)
            .unwrap();

        assert_eq!(status, OK_LOADED_MESSAGE);
        let axiom_set = interactive.axiom_sets().next().unwrap();
        assert_eq!(axiom_set.name(), "real.ax");
        assert_eq!(axiom_set.clause_set().len(), 1);
        assert_eq!(axiom_set.formula_set().cardinality(), 1);
    }

    #[test]
    fn load_command_propagates_parser_diagnostics_without_inserting() {
        let scratch = ScratchDir::new();
        fs::write(scratch.path.join("bad.ax"), b"not a problem").unwrap();
        let mut interactive = InteractiveSpec::new(scratch.path.to_string_lossy());

        let error = interactive
            .load_command_with("bad.ax", |_, _| {
                Err(Diagnostic::new(
                    ErrorCode::SYNTAX_ERROR,
                    "synthetic parser failure",
                ))
            })
            .unwrap_err();

        assert_eq!(error.code(), ErrorCode::SYNTAX_ERROR);
        assert_eq!(interactive.axiom_set_count(), 0);
    }

    #[test]
    fn read_interactive_text_block_reports_completion_and_partial_eof() {
        let mut complete_reader = Cursor::new(b"one\ntwo\nGO\nignored\n".to_vec());

        let complete =
            read_interactive_text_block(&mut complete_reader, END_OF_BLOCK_TOKEN).unwrap();

        assert!(complete.complete);
        assert_eq!(complete.input, "one\ntwo\n");

        let mut eof_reader = Cursor::new(b"partial".to_vec());

        let partial = read_interactive_text_block(&mut eof_reader, END_OF_BLOCK_TOKEN).unwrap();

        assert!(!partial.complete);
        assert_eq!(partial.input, "partial");
    }

    #[test]
    fn read_interactive_tcp_block_reads_messages_until_exact_terminator() {
        let mut reader = Cursor::new(packed_tcp_messages(&[
            "fof(a, axiom, p).\n",
            END_OF_BLOCK_TOKEN,
            "ignored\n",
        ]));

        let block = read_interactive_tcp_block(&mut reader, END_OF_BLOCK_TOKEN).unwrap();

        assert!(block.complete);
        assert_eq!(block.input, "fof(a, axiom, p).\n");
    }

    #[test]
    fn stage_command_adds_problem_to_control_and_marks_shared_boundary() {
        let signature = test_signature();
        let mut ctrl = StructFofSpec::new(&signature);
        let mut interactive = InteractiveSpec::new("");
        assert_eq!(
            interactive.add_axiom_set(axiom_set("stage_me", "raw", false)),
            OK_ADDED_MESSAGE
        );

        assert_eq!(
            interactive.stage_command(&mut ctrl, &signature, "stage_me"),
            OK_STAGED_MESSAGE
        );

        assert!(interactive.axiom_set_mut("stage_me").unwrap().is_staged());
        assert_eq!(ctrl.clause_set_count(), 1);
        assert_eq!(ctrl.formula_set_count(), 1);
        assert_eq!(ctrl.shared_ax_sp(), 1);
        assert_eq!(ctrl.problem_type(), ProblemType::FirstOrder);
    }

    #[test]
    fn stage_command_preserves_parsed_axiom_problem_type() {
        let signature = test_signature();
        let mut ctrl = StructFofSpec::new(&signature);
        let mut interactive = InteractiveSpec::new("");
        let mut problem = empty_problem();
        problem.problem_type = ProblemType::HigherOrder;
        assert_eq!(
            interactive.add_parsed_axiom_set("higher", "thf(a,axiom,p).\n", problem),
            OK_ADDED_MESSAGE
        );

        assert_eq!(
            interactive.stage_command(&mut ctrl, &signature, "higher"),
            OK_STAGED_MESSAGE
        );

        assert!(interactive.axiom_set_mut("higher").unwrap().is_staged());
        assert_eq!(ctrl.problem_type(), ProblemType::HigherOrder);
        assert_eq!(ctrl.shared_ax_sp(), 1);
    }

    #[test]
    fn stage_command_reports_unknown_or_already_staged_without_extra_problem() {
        let signature = test_signature();
        let mut ctrl = StructFofSpec::new(&signature);
        let mut interactive = InteractiveSpec::new("");
        assert_eq!(
            interactive.add_axiom_set(axiom_set("once", "raw", false)),
            OK_ADDED_MESSAGE
        );

        assert_eq!(
            interactive.stage_command(&mut ctrl, &signature, "missing"),
            ERR_UNKNOWN_AXIOM_SET_MESSAGE
        );
        assert_eq!(
            interactive.stage_command(&mut ctrl, &signature, "once"),
            OK_STAGED_MESSAGE
        );
        assert_eq!(
            interactive.stage_command(&mut ctrl, &signature, "once"),
            ERR_AXIOM_SET_IS_ALREADY_STAGED_MESSAGE
        );

        assert_eq!(ctrl.clause_set_count(), 1);
        assert_eq!(ctrl.shared_ax_sp(), 1);
    }

    #[test]
    fn unstage_command_removes_matching_control_problem_and_updates_boundary() {
        let signature = test_signature();
        let mut ctrl = StructFofSpec::new(&signature);
        let mut interactive = InteractiveSpec::new("");
        for name in ["first", "second"] {
            assert_eq!(
                interactive.add_axiom_set(axiom_set(name, name, false)),
                OK_ADDED_MESSAGE
            );
            assert_eq!(
                interactive.stage_command(&mut ctrl, &signature, name),
                OK_STAGED_MESSAGE
            );
        }

        assert_eq!(
            interactive.unstage_command(&mut ctrl, &signature, "first"),
            OK_UNSTAGED_MESSAGE
        );

        assert!(!interactive.axiom_set_mut("first").unwrap().is_staged());
        assert!(interactive.axiom_set_mut("second").unwrap().is_staged());
        assert_eq!(ctrl.clause_set_count(), 1);
        assert_eq!(ctrl.formula_set_count(), 1);
        assert_eq!(ctrl.shared_ax_sp(), 1);
    }

    #[test]
    fn unstage_command_reports_unknown_or_already_unstaged() {
        let signature = test_signature();
        let mut ctrl = StructFofSpec::new(&signature);
        let mut interactive = InteractiveSpec::new("");
        assert_eq!(
            interactive.add_axiom_set(axiom_set("plain", "raw", false)),
            OK_ADDED_MESSAGE
        );

        assert_eq!(
            interactive.unstage_command(&mut ctrl, &signature, "missing"),
            ERR_UNKNOWN_AXIOM_SET_MESSAGE
        );
        assert_eq!(
            interactive.unstage_command(&mut ctrl, &signature, "plain"),
            ERR_AXIOM_SET_IS_ALREADY_UNSTAGED_MESSAGE
        );

        assert_eq!(ctrl.clause_set_count(), 0);
        assert_eq!(ctrl.shared_ax_sp(), 0);
    }

    #[test]
    fn unstage_command_preserves_c_flag_clear_before_missing_control_set_error() {
        let signature = test_signature();
        let mut ctrl = StructFofSpec::new(&signature);
        let mut interactive = InteractiveSpec::new("");
        assert_eq!(
            interactive.add_axiom_set(axiom_set("orphan", "raw", false)),
            OK_ADDED_MESSAGE
        );
        interactive
            .axiom_set_mut("orphan")
            .unwrap()
            .set_staged(true);

        assert_eq!(
            interactive.unstage_command(&mut ctrl, &signature, "orphan"),
            ERR_UNKNOWN_AXIOM_SET_MESSAGE
        );

        assert!(!interactive.axiom_set_mut("orphan").unwrap().is_staged());
        assert_eq!(ctrl.clause_set_count(), 0);
        assert_eq!(ctrl.shared_ax_sp(), 0);
    }

    #[test]
    fn dispatch_command_routes_immediate_state_commands() {
        let mut bank = parser_bank();
        let mut ctrl = StructFofSpec::new(bank.signature());
        let spec = BatchSpec::new("umlaut", IoFormat::Tstp);
        let mut interactive = InteractiveSpec::new("");
        assert_eq!(
            interactive.add_axiom_set(axiom_set("set.one", "raw", false)),
            OK_ADDED_MESSAGE
        );

        let stage = interactive
            .dispatch_command_with(
                "STAGE set . one",
                &spec,
                &mut bank,
                &mut ctrl,
                unused_block_reader,
                unused_run_command,
            )
            .unwrap();

        assert_eq!(stage.output, OK_STAGED_MESSAGE);
        assert!(!stage.done);
        assert!(interactive.axiom_set_mut("set.one").unwrap().is_staged());
        assert_eq!(ctrl.clause_set_count(), 1);

        let unstage = interactive
            .dispatch_command_with(
                "UNSTAGE set.one",
                &spec,
                &mut bank,
                &mut ctrl,
                unused_block_reader,
                unused_run_command,
            )
            .unwrap();

        assert_eq!(unstage.output, OK_UNSTAGED_MESSAGE);
        assert!(!unstage.done);
        assert!(!interactive.axiom_set_mut("set.one").unwrap().is_staged());
        assert_eq!(ctrl.clause_set_count(), 0);

        let remove = interactive
            .dispatch_command_with(
                "REMOVE set.one",
                &spec,
                &mut bank,
                &mut ctrl,
                unused_block_reader,
                unused_run_command,
            )
            .unwrap();

        assert_eq!(remove.output, OK_REMOVED_MESSAGE);
        assert!(!remove.done);
        assert_eq!(interactive.axiom_set_count(), 0);
    }

    #[test]
    fn dispatch_add_command_reads_block_then_uses_batch_parser() {
        let mut bank = parser_bank();
        let mut ctrl = StructFofSpec::new(bank.signature());
        let spec = BatchSpec::new("umlaut", IoFormat::Tstp);
        let mut interactive = InteractiveSpec::new("");
        let mut terminators = Vec::new();

        let result = interactive
            .dispatch_command_with(
                "ADD uploaded",
                &spec,
                &mut bank,
                &mut ctrl,
                |terminator| {
                    terminators.push(terminator.to_owned());
                    Ok(String::from("fof(dispatch_formula, axiom, p(a)).\n"))
                },
                unused_run_command,
            )
            .unwrap();

        assert_eq!(terminators, vec![String::from(END_OF_BLOCK_TOKEN)]);
        assert_eq!(result.output, OK_ADDED_MESSAGE);
        assert!(!result.done);
        let axiom_set = interactive.axiom_sets().next().unwrap();
        assert_eq!(axiom_set.name(), "uploaded");
        assert_eq!(axiom_set.formula_set().cardinality(), 1);
    }

    #[test]
    fn dispatch_text_command_reads_add_block_from_line_transport() {
        let mut bank = parser_bank();
        let mut ctrl = StructFofSpec::new(bank.signature());
        let spec = BatchSpec::new("umlaut", IoFormat::Tstp);
        let mut interactive = InteractiveSpec::new("");
        let mut block_reader =
            Cursor::new(b"fof(text_formula, axiom, p(a)).\nGO\nignored\n".to_vec());

        let result = interactive
            .dispatch_text_command_with(
                "ADD text_upload",
                &mut block_reader,
                &spec,
                &mut bank,
                &mut ctrl,
                unused_run_command,
            )
            .unwrap();

        assert_eq!(result.output, OK_ADDED_MESSAGE);
        assert_eq!(interactive.axiom_set_count(), 1);
        let axiom_set = interactive.axiom_sets().next().unwrap();
        assert_eq!(axiom_set.name(), "text_upload");
        assert_eq!(axiom_set.formula_set().cardinality(), 1);
    }

    #[test]
    fn dispatch_tcp_command_reads_add_block_from_message_transport() {
        let mut bank = parser_bank();
        let mut ctrl = StructFofSpec::new(bank.signature());
        let spec = BatchSpec::new("umlaut", IoFormat::Tstp);
        let mut interactive = InteractiveSpec::new("");
        let mut block_reader = Cursor::new(packed_tcp_messages(&[
            "fof(tcp_formula, axiom, p(a)).\n",
            END_OF_BLOCK_TOKEN,
            "ignored\n",
        ]));

        let result = interactive
            .dispatch_tcp_command_with(
                "ADD tcp_upload",
                &mut block_reader,
                &spec,
                &mut bank,
                &mut ctrl,
                unused_run_command,
            )
            .unwrap();

        assert_eq!(result.output, OK_ADDED_MESSAGE);
        assert_eq!(interactive.axiom_set_count(), 1);
        let axiom_set = interactive.axiom_sets().next().unwrap();
        assert_eq!(axiom_set.name(), "tcp_upload");
        assert_eq!(axiom_set.formula_set().cardinality(), 1);
    }

    #[test]
    fn add_command_accepts_every_tstp_input_form_supported_by_c_server_parser() {
        let scratch = ScratchDir::new();
        let include_path = scratch.path.join("included.ax");
        fs::write(
            &include_path,
            "fof(included_formula, axiom, included_p(a)).\n",
        )
        .unwrap();
        let include_path = include_path.to_string_lossy().replace('\\', "/");
        let first_order_input = format!(
            "cnf(cnf_axiom, axiom, p(a)).\n\
             cnf(cnf_watch, watchlist, ~p(a)|q(a)).\n\
             fof(fof_axiom, axiom, r(a)).\n\
             fof(fof_distinct, axiom, $distinct(a,b)).\n\
             tff(person_type, type, person: $tType).\n\
             tff(typed_a_type, type, typed_a: person).\n\
             tff(typed_p_type, type, typed_p: person > $o).\n\
             tff(tff_axiom, axiom, typed_p(typed_a)).\n\
             tcf(tcf_watch, watchlist, s(X)|t(X)).\n\
             include('{include_path}', [included_formula]).\n"
        );
        let spec = BatchSpec::new("umlaut", IoFormat::Tstp);
        let mut bank = parser_bank();
        let ctrl = StructFofSpec::new(bank.signature());
        let mut interactive = InteractiveSpec::new("");

        assert_eq!(
            interactive
                .add_command(
                    "all_first_order",
                    &first_order_input,
                    &spec,
                    &mut bank,
                    &ctrl
                )
                .unwrap(),
            OK_ADDED_MESSAGE
        );
        let first_order = interactive.axiom_sets().next().unwrap();
        assert_eq!(first_order.problem_type(), ProblemType::FirstOrder);
        assert_eq!(first_order.clause_set().len(), 2);
        assert_eq!(first_order.formula_set().cardinality(), 8);
        assert!(bank.signature().typed_symbols());

        let higher_order_input = "thf(person_type, type, person: $tType).\n\
                                  thf(a_type, type, a: person).\n\
                                  thf(p_type, type, p: person > $o).\n\
                                  thf(thf_axiom, axiom, p @ a).\n";
        let mut bank = parser_bank();
        let ctrl = StructFofSpec::new(bank.signature());
        let mut interactive = InteractiveSpec::new("");

        assert_eq!(
            interactive
                .add_command(
                    "all_higher_order",
                    higher_order_input,
                    &spec,
                    &mut bank,
                    &ctrl
                )
                .unwrap(),
            OK_ADDED_MESSAGE
        );
        let higher_order = interactive.axiom_sets().next().unwrap();
        assert_eq!(higher_order.problem_type(), ProblemType::HigherOrder);
        assert_eq!(higher_order.clause_set().len(), 0);
        assert_eq!(higher_order.formula_set().cardinality(), 4);
        assert!(bank.signature().typed_symbols());
    }

    #[test]
    fn start_deduction_server_tcp_processes_messages_until_quit() {
        let mut bank = parser_bank();
        let mut ctrl = StructFofSpec::new(bank.signature());
        let spec = BatchSpec::new("umlaut", IoFormat::Tstp);
        let mut stream = Duplex::new(&[
            "ADD uploaded",
            "fof(uploaded_formula, axiom, p(a)).\n",
            END_OF_BLOCK_TOKEN,
            "LIST",
            "QUIT",
        ]);

        let report = start_deduction_server_tcp_with(
            &mut stream,
            "",
            &spec,
            &mut bank,
            &mut ctrl,
            unused_run_command,
        )
        .unwrap();

        assert_eq!(
            report,
            super::InteractiveServerReport {
                commands: 3,
                done: true,
            }
        );
        let messages = stream.written_messages();
        assert_eq!(messages.len(), 3);
        assert_eq!(messages[0], OK_ADDED_MESSAGE);
        assert!(messages[1].contains("Unstaged :\n  uploaded\n"));
        assert_eq!(messages[2], OK_SUCCESS_MESSAGE);
    }

    #[test]
    fn start_deduction_server_tcp_sends_unknown_command_status() {
        let mut bank = parser_bank();
        let mut ctrl = StructFofSpec::new(bank.signature());
        let spec = BatchSpec::new("umlaut", IoFormat::Tstp);
        let mut stream = Duplex::new(&["BOGUS", "QUIT"]);

        let report = start_deduction_server_tcp_with(
            &mut stream,
            "",
            &spec,
            &mut bank,
            &mut ctrl,
            unused_run_command,
        )
        .unwrap();

        assert_eq!(report.commands, 2);
        assert!(report.done);
        assert_eq!(
            stream.written_messages(),
            vec![String::from(ERR_UNKNOWN_COMMAND_MESSAGE)]
        );
    }

    #[test]
    fn dispatch_load_command_uses_server_library_parser() {
        let scratch = ScratchDir::new();
        fs::write(scratch.path.join("dispatch.ax"), b"fof(a, axiom, p).\n").unwrap();
        let mut bank = parser_bank();
        let mut ctrl = StructFofSpec::new(bank.signature());
        let spec = BatchSpec::new("umlaut", IoFormat::Tstp);
        let mut interactive = InteractiveSpec::new(scratch.path.to_string_lossy());

        let result = interactive
            .dispatch_command_with(
                "LOAD dispatch.ax",
                &spec,
                &mut bank,
                &mut ctrl,
                unused_block_reader,
                unused_run_command,
            )
            .unwrap();

        assert_eq!(result.output, OK_LOADED_MESSAGE);
        assert!(!result.done);
        assert_eq!(interactive.axiom_set_count(), 1);
        assert_eq!(
            interactive.download_command("dispatch.ax").output,
            "fof(a, axiom, p).\n"
        );
    }

    #[test]
    fn dispatch_run_command_uses_only_one_identifier_token_for_name() {
        let mut bank = parser_bank();
        let mut ctrl = StructFofSpec::new(bank.signature());
        let spec = BatchSpec::new("umlaut", IoFormat::Tstp);
        let mut interactive = InteractiveSpec::new("");
        let mut terminators = Vec::new();
        let mut seen_run = None;

        let result = interactive
            .dispatch_command_with(
                "RUN job-1",
                &spec,
                &mut bank,
                &mut ctrl,
                |terminator| {
                    terminators.push(terminator.to_owned());
                    Ok(String::from("fof(run_formula, axiom, q(a)).\n"))
                },
                |_, _, _, job_name, input_axioms| {
                    seen_run = Some((job_name.to_owned(), input_axioms.to_owned()));
                    Ok(InteractiveCommandOutput {
                        output: String::from("run output\n"),
                        frame_offsets: vec!["run output\n".len()],
                        status: OK_SUCCESS_MESSAGE,
                    })
                },
            )
            .unwrap();

        assert_eq!(terminators, vec![String::from(END_OF_BLOCK_TOKEN)]);
        assert_eq!(
            seen_run,
            Some((
                String::from("job"),
                String::from("fof(run_formula, axiom, q(a)).\n")
            ))
        );
        assert_eq!(result.output, format!("run output\n{OK_SUCCESS_MESSAGE}"));
        assert_eq!(
            output_frames(&result.output, &result.frame_offsets),
            ["run output\n", OK_SUCCESS_MESSAGE]
        );
        assert!(!result.done);
    }

    #[test]
    fn run_command_with_parses_job_runs_batch_process_and_backtracks() {
        let mut bank = parser_bank();
        let mut ctrl = StructFofSpec::new(bank.signature());
        let spec = BatchSpec::new("umlaut", IoFormat::Tstp);
        let mut interactive = InteractiveSpec::new("");
        assert_eq!(
            interactive.add_axiom_set(axiom_set("shared", "shared raw", false)),
            OK_ADDED_MESSAGE
        );
        assert_eq!(
            interactive.stage_command(&mut ctrl, bank.signature(), "shared"),
            OK_STAGED_MESSAGE
        );
        let mut requests = Vec::<BatchRunnerRequest>::new();

        let report = run_command_with(
            "job1",
            "fof(job_formula, axiom, q(a)).\n",
            &spec,
            &mut bank,
            &mut ctrl,
            || 100,
            |request| {
                requests.push(request.clone());
                Ok(BatchSpawnedRunner {
                    name: request.name,
                    start_time: 100,
                    prob_time: request.cpu_time,
                })
            },
            |active| {
                assert!(!active.is_empty());
                Ok(Some(BatchCompletedRunner {
                    runner: active[0].clone(),
                    result: ProverResult::Theorem,
                    output: String::from("% run proof\n"),
                }))
            },
        )
        .unwrap();

        assert!(report.process.solved);
        assert!(!requests.is_empty());
        assert_eq!(requests[0].cpu_time, 15);
        assert_eq!(ctrl.clause_set_count(), 1);
        assert_eq!(ctrl.formula_set_count(), 1);
        assert!(interactive.axiom_set_mut("shared").unwrap().is_staged());
        assert_eq!(report.command.status, OK_SUCCESS_MESSAGE);
        assert_eq!(
            report.command.output,
            "\n% Processing started for job1\n% run proof\n\n% Processing finished for job1\n\n"
        );
        assert_eq!(
            output_frames(&report.command.output, &report.command.frame_offsets),
            [
                "\n% Processing started for job1\n",
                "% run proof\n",
                "\n% Processing finished for job1\n\n",
            ]
        );
        assert!(report.global_output.starts_with("job1"));
        assert!(report
            .global_output
            .contains("% SZS status Theorem for job1\n"));
        assert!(report.global_output.ends_with("% run proof\n"));

        let dispatched = super::dispatch_command_output(report.command, false);
        assert_eq!(
            output_frames(&dispatched.output, &dispatched.frame_offsets),
            [
                "\n% Processing started for job1\n",
                "% run proof\n",
                "\n% Processing finished for job1\n\n",
                OK_SUCCESS_MESSAGE,
            ]
        );
    }

    #[test]
    fn run_command_with_uses_configured_per_problem_limit() {
        let mut bank = parser_bank();
        let mut ctrl = StructFofSpec::new(bank.signature());
        let mut spec = BatchSpec::new("umlaut", IoFormat::Tstp);
        spec.per_prob_limit = 12;
        let mut requests = Vec::<BatchRunnerRequest>::new();

        let report = run_command_with(
            "limited",
            "fof(job_formula, axiom, q(a)).\n",
            &spec,
            &mut bank,
            &mut ctrl,
            || 200,
            |request| {
                requests.push(request.clone());
                Ok(BatchSpawnedRunner {
                    name: request.name,
                    start_time: 200,
                    prob_time: request.cpu_time,
                })
            },
            |active| {
                Ok(Some(BatchCompletedRunner {
                    runner: active[0].clone(),
                    result: ProverResult::Theorem,
                    output: String::from("% limited proof\n"),
                }))
            },
        )
        .unwrap();

        assert!(report.process.solved);
        assert!(!requests.is_empty());
        assert_eq!(requests[0].cpu_time, 6);
        assert_eq!(report.command.status, OK_SUCCESS_MESSAGE);
        assert!(report
            .command
            .output
            .contains("% Processing finished for limited\n\n"));
    }

    #[test]
    fn dispatch_printing_commands_append_statuses() {
        let mut bank = parser_bank();
        let mut ctrl = StructFofSpec::new(bank.signature());
        let spec = BatchSpec::new("umlaut", IoFormat::Tstp);
        let mut interactive = InteractiveSpec::new("");
        assert_eq!(
            interactive.add_axiom_set(axiom_set("download_me", "raw axioms\n", false)),
            OK_ADDED_MESSAGE
        );
        assert_eq!(
            interactive.add_axiom_set(axiom_set("empty_download", "", false)),
            OK_ADDED_MESSAGE
        );

        let download = interactive
            .dispatch_command_with(
                "DOWNLOAD download_me",
                &spec,
                &mut bank,
                &mut ctrl,
                unused_block_reader,
                unused_run_command,
            )
            .unwrap();

        assert_eq!(
            download.output,
            format!("raw axioms\n{OK_DOWNLOADED_MESSAGE}")
        );
        assert_eq!(
            output_frames(&download.output, &download.frame_offsets),
            ["raw axioms\n", OK_DOWNLOADED_MESSAGE]
        );
        assert!(!download.done);

        let empty_download = interactive
            .dispatch_command_with(
                "DOWNLOAD empty_download",
                &spec,
                &mut bank,
                &mut ctrl,
                unused_block_reader,
                unused_run_command,
            )
            .unwrap();
        assert_eq!(
            output_frames(&empty_download.output, &empty_download.frame_offsets),
            ["", OK_DOWNLOADED_MESSAGE]
        );

        let list = interactive
            .dispatch_command_with(
                "LIST",
                &spec,
                &mut bank,
                &mut ctrl,
                unused_block_reader,
                unused_run_command,
            )
            .unwrap();

        assert!(list.output.contains("Unstaged :\n  download_me\n"));
        assert!(list.output.ends_with(OK_SUCCESS_MESSAGE));
        let list_frames = output_frames(&list.output, &list.frame_offsets);
        assert_eq!(list_frames.len(), 2);
        assert!(list_frames[0].contains("Unstaged :\n  download_me\n"));
        assert_eq!(list_frames[1], OK_SUCCESS_MESSAGE);
        assert!(!list.done);

        let help = interactive
            .dispatch_command_with(
                "HELP",
                &spec,
                &mut bank,
                &mut ctrl,
                unused_block_reader,
                unused_run_command,
            )
            .unwrap();

        assert_eq!(help.output, format!("{HELP_MESSAGE}{OK_SUCCESS_MESSAGE}"));
        assert_eq!(
            output_frames(&help.output, &help.frame_offsets),
            [HELP_MESSAGE, OK_SUCCESS_MESSAGE]
        );
        assert!(!help.done);
    }

    #[test]
    fn dispatch_quit_unstages_all_sets_and_marks_done() {
        let mut bank = parser_bank();
        let mut ctrl = StructFofSpec::new(bank.signature());
        let spec = BatchSpec::new("umlaut", IoFormat::Tstp);
        let mut interactive = InteractiveSpec::new("");
        for name in ["first", "second"] {
            assert_eq!(
                interactive.add_axiom_set(axiom_set(name, name, false)),
                OK_ADDED_MESSAGE
            );
            assert_eq!(
                interactive.stage_command(&mut ctrl, bank.signature(), name),
                OK_STAGED_MESSAGE
            );
        }

        let result = interactive
            .dispatch_command_with(
                "QUIT",
                &spec,
                &mut bank,
                &mut ctrl,
                unused_block_reader,
                unused_run_command,
            )
            .unwrap();

        assert_eq!(result.output, "");
        assert!(result.frame_offsets.is_empty());
        assert!(result.done);
        assert!(!interactive.axiom_set_mut("first").unwrap().is_staged());
        assert!(!interactive.axiom_set_mut("second").unwrap().is_staged());
        assert_eq!(ctrl.clause_set_count(), 0);
        assert_eq!(ctrl.formula_set_count(), 0);
        assert_eq!(ctrl.shared_ax_sp(), 0);
    }

    #[test]
    fn dispatch_unknown_command_reports_protocol_error() {
        let mut bank = parser_bank();
        let mut ctrl = StructFofSpec::new(bank.signature());
        let spec = BatchSpec::new("umlaut", IoFormat::Tstp);
        let mut interactive = InteractiveSpec::new("");

        let result = interactive
            .dispatch_command_with(
                "BOGUS",
                &spec,
                &mut bank,
                &mut ctrl,
                unused_block_reader,
                unused_run_command,
            )
            .unwrap();

        assert_eq!(result.output, ERR_UNKNOWN_COMMAND_MESSAGE);
        assert_eq!(result.frame_offsets, [ERR_UNKNOWN_COMMAND_MESSAGE.len()]);
        assert!(!result.done);
    }

    #[test]
    fn list_command_groups_staged_unstaged_and_missing_server_library() {
        let mut interactive = InteractiveSpec::new("");
        assert_eq!(
            interactive.add_axiom_set(axiom_set("loaded", "loaded raw", false)),
            OK_ADDED_MESSAGE
        );
        assert_eq!(
            interactive.add_axiom_set(axiom_set("queued", "queued raw", false)),
            OK_ADDED_MESSAGE
        );
        interactive
            .axiom_set_mut("queued")
            .unwrap()
            .set_staged(true);

        let result = interactive.list_command();

        assert_eq!(result.status, OK_SUCCESS_MESSAGE);
        assert_eq!(
            result.output,
            "Staged :\n  queued\nUnstaged :\n  loaded\nOn Disk :\n\tNo axioms directory was specified on server startup.\n"
        );
    }

    #[test]
    fn list_command_reports_empty_memory_and_directory_open_failure() {
        let mut missing = std::env::temp_dir();
        missing.push(format!(
            "e_rust_port_einteractive_missing_list_{}_{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let interactive = InteractiveSpec::new(missing.to_string_lossy());

        let result = interactive.list_command();

        assert_eq!(result.status, OK_SUCCESS_MESSAGE);
        assert_eq!(
            result.output,
            "No Axiom Sets currently in memory.\nOn Disk :\n\tCould not open current directory.\n"
        );
    }

    #[test]
    fn list_command_prints_disk_files_in_stack_pop_order() {
        let scratch = ScratchDir::new();
        fs::write(scratch.path.join("only.ax"), b"fof(a, axiom, p).").unwrap();
        fs::create_dir(scratch.path.join("nested")).unwrap();
        let interactive = InteractiveSpec::new(scratch.path.to_string_lossy());

        let result = interactive.list_command();

        assert_eq!(result.status, OK_SUCCESS_MESSAGE);
        assert_eq!(
            result.output,
            "No Axiom Sets currently in memory.\nOn Disk :\n\tonly.ax\n"
        );
    }

    #[test]
    fn download_command_prints_raw_data_then_ok_status() {
        let mut interactive = InteractiveSpec::new("");
        assert_eq!(
            interactive.add_axiom_set(axiom_set("download_me", "raw axioms\n", false)),
            OK_ADDED_MESSAGE
        );

        let result = interactive.download_command("download_me");

        assert_eq!(result.output, "raw axioms\n");
        assert_eq!(result.status, OK_DOWNLOADED_MESSAGE);
    }

    #[test]
    fn download_command_reports_unknown_axiom_set_without_output() {
        let interactive = InteractiveSpec::new("");

        let result = interactive.download_command("missing");

        assert_eq!(result.output, "");
        assert_eq!(result.status, ERR_UNKNOWN_AXIOM_SET_MESSAGE);
    }

    #[test]
    fn remove_command_removes_unstaged_set_and_restores_stack_order() {
        let mut interactive = InteractiveSpec::new("");
        for name in ["first", "remove_me", "last"] {
            assert_eq!(
                interactive.add_axiom_set(axiom_set(name, name, false)),
                OK_ADDED_MESSAGE
            );
        }

        assert_eq!(interactive.remove_command("remove_me"), OK_REMOVED_MESSAGE);

        assert_eq!(
            axiom_names(&interactive),
            vec![String::from("first"), String::from("last")]
        );
    }

    #[test]
    fn remove_command_preserves_c_staged_error_stack_side_effect() {
        let mut interactive = InteractiveSpec::new("");
        for name in ["first", "staged", "last"] {
            assert_eq!(
                interactive.add_axiom_set(axiom_set(name, name, false)),
                OK_ADDED_MESSAGE
            );
        }
        interactive
            .axiom_set_mut("staged")
            .unwrap()
            .set_staged(true);

        assert_eq!(
            interactive.remove_command("staged"),
            ERR_AXIOM_SET_IS_STAGED_MESSAGE
        );

        assert_eq!(axiom_names(&interactive), vec![String::from("first")]);
    }

    #[test]
    fn remove_command_reports_unknown_and_restores_all_sets() {
        let mut interactive = InteractiveSpec::new("");
        for name in ["first", "second"] {
            assert_eq!(
                interactive.add_axiom_set(axiom_set(name, name, false)),
                OK_ADDED_MESSAGE
            );
        }

        assert_eq!(
            interactive.remove_command("missing"),
            ERR_UNKNOWN_AXIOM_SET_MESSAGE
        );

        assert_eq!(
            axiom_names(&interactive),
            vec![String::from("first"), String::from("second")]
        );
    }
}
