# `eprover` option-help parity

## Objective

Resolve `E_Rust_Port-j76.2.101`. The Rust table already matched C long names, short aliases, argument kinds, and default arguments, but its declaration order differed and many descriptions were abbreviated. Because `print_help` uses table order and prose directly, that was a real drop-in help-output gap rather than a harmless internal representation choice.

## Implementation

`generate_options.py` reads the unchanged `eprover/PROVER/e_options.h`, maps each C entry to the existing Rust semantic option by long name, then rewrites only the production table order and description fields. It expands C's `NAME` and `WATCHLIST_INLINE_QSTRING` help-text macros, preserves concatenated literals and reference typos, and refuses missing, extra, duplicate, non-ASCII, or structurally malformed entries. `--check` verifies that the committed Rust table is current.

The Rust source tests now compare declaration order directly instead of sorting the long-name, short-alias, and argument/default surfaces. A fourth test decodes the C concatenated-string expressions from the included header and compares every description. Production remains self-contained: it does not parse or embed C source at runtime, and the existing shared command-line renderer supplies C-compatible option wrapping.

This also resolves the duplicated help-policy reviews `E_Rust_Port-j76.4.1053`, `.1054`, `.1168`, and `E_Rust_Port-j76.3.299`. Runtime effects for C-advertised but unhandled options remain under narrower items such as `E_Rust_Port-j76.3.297` and `E_Rust_Port-j76.4.1055`.

## Reference limitation

The host has neither a WSL distribution nor a native C compiler/make toolchain, so a fresh live C executable could not be built. The attempted reference source was copied to an isolated `C:\tmp` directory and removed; the vendored tree was never modified. The unchanged C header is still compared directly by the Rust tests and generator.

## Validation

- The four source-derived option-table tests and two executable help/control-flow tests passed.
- `generate_options.py --check` passed.
- 4,234 default-feature library tests passed.
- 4,239 all-feature library tests, every binary target, and all 7 integration tests passed.
- Strict all-target, all-feature pedantic Clippy passed.
- The release `eprover` build and `cargo fmt --all -- --check` passed.
- C-source documentation coverage, Change Later wording, Markdown-link integrity, and regeneration-preservation gates passed.
- The vendored `eprover/` worktree remained clean.
