//! Deduction-server interactive command surface from `cco_einteractive_mode`.

use std::{ffi::OsStr, fs, path::Path};

use crate::basics::error::Diagnostic;
use crate::inout::scanner::{Scanner, TokenType};

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

#[cfg(test)]
mod tests {
    use super::{
        accept_axiom_set_name, axiom_set_name_tokens, get_directory_listings, ADD_COMMAND,
        END_OF_BLOCK_TOKEN, ERR_UNKNOWN_COMMAND_MESSAGE, HELP_MESSAGE, OK_SUCCESS_MESSAGE,
        STAGE_COMMAND,
    };
    use crate::inout::scanner::{Scanner, TokenType};
    use std::{
        collections::BTreeSet,
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn scanner(source: &str) -> Scanner {
        Scanner::from_user_string(source, true).unwrap()
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
}
