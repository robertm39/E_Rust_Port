//! Deduction-server interactive command surface from `cco_einteractive_mode`.

use std::{ffi::OsStr, fmt::Write as _, fs, path::Path};

use crate::basics::error::{Diagnostic, ErrorCode};
use crate::clauses::{clausesets::ClauseSet, formulasets::FormulaSet};
use crate::control::batch_spec::{BatchProblemData, BatchSpec};
use crate::control::sine::StructFofSpec;
use crate::inout::scanner::{IoFormat, Scanner, TokenType};
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
        let _ = staged;
        Self {
            cset,
            fset,
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
        Self::new(problem.clauses, problem.formulas, raw_data, false)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InteractiveCommandOutput {
    pub output: String,
    pub status: &'static str,
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

        ctrl.add_problem(
            signature,
            self.axiom_sets[index].clause_set().clone(),
            self.axiom_sets[index].formula_set().clone(),
            false,
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

        InteractiveCommandOutput {
            output,
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
                    status: ERR_UNKNOWN_AXIOM_SET_MESSAGE,
                },
                |handle| InteractiveCommandOutput {
                    output: handle.raw_data().to_owned(),
                    status: OK_DOWNLOADED_MESSAGE,
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
        accept_axiom_set_name, axiom_set_name_tokens, get_directory_listings, AxiomSet,
        InteractiveSpec, ADD_COMMAND, END_OF_BLOCK_TOKEN, ERR_AXIOM_SET_IS_ALREADY_STAGED_MESSAGE,
        ERR_AXIOM_SET_IS_ALREADY_UNSTAGED_MESSAGE, ERR_AXIOM_SET_IS_STAGED_MESSAGE,
        ERR_AXIOM_SET_NAME_TAKEN_MESSAGE, ERR_CANNOT_READ_SERVER_LIBRARY_MESSAGE,
        ERR_NO_AXIOM_LIBRARY_ON_SERVER_MESSAGE, ERR_UNKNOWN_AXIOM_SET_MESSAGE,
        ERR_UNKNOWN_COMMAND_MESSAGE, HELP_MESSAGE, OK_ADDED_MESSAGE, OK_DOWNLOADED_MESSAGE,
        OK_LOADED_MESSAGE, OK_REMOVED_MESSAGE, OK_STAGED_MESSAGE, OK_SUCCESS_MESSAGE,
        OK_UNSTAGED_MESSAGE, STAGE_COMMAND,
    };
    use crate::basics::error::{Diagnostic, ErrorCode};
    use crate::clauses::{clausesets::ClauseSet, formulasets::FormulaSet};
    use crate::control::batch_spec::{BatchProblemData, BatchSpec};
    use crate::control::sine::StructFofSpec;
    use crate::inout::scanner::{IoFormat, Scanner, TokenType};
    use crate::terms::{signature::Signature, termbanks::TermBank, typebanks::TypeBank};
    use std::{
        collections::BTreeSet,
        ffi::OsStr,
        fs,
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
        assert_eq!(ADD_COMMAND, "ADD");
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

        assert_eq!(
            interactive.add_parsed_axiom_set("parsed", "fof(a,axiom,p).\n", empty_problem()),
            OK_ADDED_MESSAGE
        );

        let axiom_set = interactive.axiom_sets().next().unwrap();
        assert_eq!(axiom_set.name(), "parsed");
        assert_eq!(axiom_set.formula_set().identifier(), "parsed");
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
        let spec = BatchSpec::new("eprover", IoFormat::Tstp);
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
        let spec = BatchSpec::new("eprover", IoFormat::Tstp);
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
        let spec = BatchSpec::new("eprover", IoFormat::Tstp);
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
