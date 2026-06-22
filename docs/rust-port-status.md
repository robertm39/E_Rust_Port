# Rust Port Status

This document tracks Rust implementation slices and the original C source units they are intended to mirror.

## Initial Crate And CLI Foundation

Rust files:

- `Cargo.toml`
- `src/basics/dstrings.rs`
- `src/basics/error.rs`
- `src/inout/commandline.rs`
- `src/prover/version.rs`
- `src/prover/options.rs`
- `src/prover/eprover.rs`
- `src/bin/eprover.rs`

Original C references:

- [`BASICS/clb_dstrings.h`, `BASICS/clb_dstrings.c`](c_source_docs/BASICS/clb_dstrings.md)
- [`BASICS/clb_error.h`, `BASICS/clb_error.c`](c_source_docs/BASICS/clb_error.md)
- [`INOUT/cio_commandline.h`, `INOUT/cio_commandline.c`](c_source_docs/INOUT/cio_commandline.md)
- [`PROVER/e_version.h`](c_source_docs/PROVER/e_version.md)
- [`PROVER/e_options.h`](c_source_docs/PROVER/e_options.md)
- [`PROVER/eprover.c`](c_source_docs/PROVER/eprover.md)

Implemented behavior:

- A root Rust package with library and `eprover` binary targets.
- The `DStr` byte-buffer behavior from `clb_dstrings`, including append, byte-buffer append, integer append, string-array append, last-character deletion, reset, minimize, line reading, and the distinct C growth rules for string and byte appends.
- C-compatible numeric exit-code constants, including the duplicate `NO_ERROR`/`PROOF_FOUND` value.
- The `TestLetterString`/`CheckOptionLetterString` behavior from `clb_error`.
- The core `CLStateGetOpt` command-line parser rules from `cio_commandline`: long options require `--name=value` for required arguments, long optional arguments default when `=` is absent, short required arguments accept attached or following values, short optional arguments use the default, `--` stops option parsing, and processed options are removed from the remaining argument list.
- Initial `eprover` handling for `--help`, `--version`, and a small option subset used by the compatibility harness setup path.

Known gaps:

- The prover core, parser, clausification, saturation loop, indexing, ordering, heuristics, proof output, resource limits, and most CLI options are not implemented yet.
- The help option table is intentionally partial until the full option table is ported.
- Running the Rust binary on a problem currently reports that proof search is not implemented.
