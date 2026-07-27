pub const PROGRAM_NAME: &str = "umlaut";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const E_BASELINE_VERSION: &str = "3.3.5";
pub const E_BASELINE_NICKNAME: &str = "Countess Grey";
pub const ECOMMITID: &str = "facc36eaf92d70896d830140efc4382df9e8dcdb";
pub const VERSION_QUALIFIER: &str = "(E 3.3.5 compatibility baseline)";
pub const PROJECT_URL: &str = "https://github.com/robertm39/E_Rust_Port";
pub const ISSUE_URL: &str = "https://github.com/robertm39/E_Rust_Port/issues";
pub const E_URL: &str = "https://www.eprover.org";
pub const STS_MAIL: &str = "schulz@eprover.org";

#[must_use]
pub fn version_line() -> String {
    format!(
        "Umlaut {VERSION}\n\
E compatibility baseline: {E_BASELINE_VERSION} {E_BASELINE_NICKNAME} ({ECOMMITID})\n"
    )
}

#[must_use]
pub fn footer() -> String {
    format!(
        "Umlaut is an independent automated theorem prover.\n\
Project source: {PROJECT_URL}\n\
Bug reports: {ISSUE_URL}\n\
\n\
Umlaut originated as a Rust port of E and retains E compatibility and\n\
provenance information. The E baseline is version {E_BASELINE_VERSION}\n\
\"{E_BASELINE_NICKNAME}\" ({ECOMMITID}). Upstream E is available at\n\
{E_URL}.\n\
\n\
E copyright 1998-2026 by Stephan Schulz, {STS_MAIL}, and the E\n\
contributors (see the bundled E source's DOC/CONTRIBUTORS file).\n\
\n\
This program is free software; you can redistribute it and/or modify\n\
it under the terms of the GNU General Public License as published by\n\
the Free Software Foundation; either version 2 of the License, or\n\
(at your option) any later version.\n\
\n\
This program is distributed in the hope that it will be useful,\n\
but WITHOUT ANY WARRANTY; without even the implied warranty of\n\
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the\n\
GNU General Public License for more details.\n\
\n\
When reporting unexpected behavior, include:\n\
\n\
- What you observed\n\
- What you expected\n\
- The output of `umlaut --version`\n\
- The full command line\n\
- The input file(s)\n\
\n"
    )
}

#[cfg(test)]
pub(crate) fn assert_help_matches_fixture(actual: &str, expected: &str) {
    fn help_body(text: &str) -> &str {
        [
            "Umlaut is an independent automated theorem prover.",
            "Copyright",
        ]
        .into_iter()
        .filter_map(|marker| text.split_once(marker).map(|(body, _)| body))
        .min_by_key(|body| body.len())
        .unwrap_or(text)
    }

    assert_eq!(
        help_body(actual).split_whitespace().collect::<Vec<_>>(),
        help_body(expected).split_whitespace().collect::<Vec<_>>()
    );
    assert!(
        actual.ends_with(&footer()),
        "help must end with the canonical Umlaut footer"
    );
}

#[cfg(test)]
mod tests {
    use super::{footer, version_line, VERSION};

    #[test]
    fn version_identifies_umlaut_and_e_only_as_baseline() {
        let rendered = version_line();

        assert!(rendered.starts_with(&format!("Umlaut {VERSION}\n")));
        assert!(rendered.contains("E compatibility baseline: 3.3.5"));
        assert!(!rendered.starts_with("E "));
    }

    #[test]
    fn footer_preserves_license_and_e_attribution_without_e_product_claim() {
        let rendered = footer();

        assert!(rendered.contains("Umlaut is an independent automated theorem prover."));
        assert!(rendered.contains("E copyright 1998-2026"));
        assert!(rendered.contains("WITHOUT ANY WARRANTY"));
        assert!(rendered.contains("- The output of `umlaut --version`"));
        assert!(!rendered.contains("This program is a part of the distribution"));
    }
}
