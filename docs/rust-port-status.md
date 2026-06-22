# Rust Port Status

This document tracks Rust implementation slices and the original C source units they are intended to mirror.

## Initial Crate And CLI Foundation

Rust files:

- `Cargo.toml`
- `src/basics/ddarrays.rs`
- `src/basics/dstacks.rs`
- `src/basics/dstrings.rs`
- `src/basics/error.rs`
- `src/basics/pdarrays.rs`
- `src/basics/pstacks.rs`
- `src/inout/basicparser.rs`
- `src/inout/commandline.rs`
- `src/inout/scanner.rs`
- `src/inout/streams.rs`
- `src/prover/version.rs`
- `src/prover/options.rs`
- `src/prover/eprover.rs`
- `src/bin/eprover.rs`

Original C references:

- [`BASICS/clb_ddarrays.h`, `BASICS/clb_ddarrays.c`](c_source_docs/BASICS/clb_ddarrays.md)
- [`BASICS/clb_dstrings.h`, `BASICS/clb_dstrings.c`](c_source_docs/BASICS/clb_dstrings.md)
- [`BASICS/clb_dstacks.h`, `BASICS/clb_dstacks.c`](c_source_docs/BASICS/clb_dstacks.md)
- [`BASICS/clb_error.h`, `BASICS/clb_error.c`](c_source_docs/BASICS/clb_error.md)
- [`BASICS/clb_pdarrays.h`, `BASICS/clb_pdarrays.c`](c_source_docs/BASICS/clb_pdarrays.md)
- [`BASICS/clb_pstacks.h`, `BASICS/clb_pstacks.c`](c_source_docs/BASICS/clb_pstacks.md)
- [`INOUT/cio_basicparser.h`, `INOUT/cio_basicparser.c`](c_source_docs/INOUT/cio_basicparser.md)
- [`INOUT/cio_commandline.h`, `INOUT/cio_commandline.c`](c_source_docs/INOUT/cio_commandline.md)
- [`INOUT/cio_scanner.h`, `INOUT/cio_scanner.c`](c_source_docs/INOUT/cio_scanner.md)
- [`INOUT/cio_streams.h`, `INOUT/cio_streams.c`](c_source_docs/INOUT/cio_streams.md)
- [`PROVER/e_version.h`](c_source_docs/PROVER/e_version.md)
- [`PROVER/e_options.h`](c_source_docs/PROVER/e_options.md)
- [`PROVER/eprover.c`](c_source_docs/PROVER/eprover.md)

Implemented behavior:

- A root Rust package with library and `eprover` binary targets.
- The `DStr` byte-buffer behavior from `clb_dstrings`, including append, byte-buffer append, integer append, string-array append, last-character deletion, reset, minimize, line reading, and the distinct C growth rules for string and byte appends.
- The `PStack` and `DStack` growth/access patterns from `clb_pstacks` and `clb_dstacks`, including explicit logical capacity doubling, reset without shrinking, top/below-top/element access, swap-remove discard, stack copying/pushing, C-shaped binary search and merge behavior, and integer average/deviation computation.
- Dynamic `PDArray` and `DDArray` storage from `clb_pdarrays` and `clb_ddarrays`, including exponential and fixed-multiple growth, zero/`NULL` initialization, mutating element access that extends arrays like the C macros, delete/store/add/increment helpers, and `DDArraySelectPart` partition selection.
- C-compatible numeric exit-code constants, including the duplicate `NO_ERROR`/`PROOF_FOUND` value.
- The `TestLetterString`/`CheckOptionLetterString` behavior from `clb_error`.
- Initial stream and scanner support for string sources, including C-compatible lookahead windows, line/column updates, token bit layout, whitespace/comment skipping, comment accumulation, identifiers and trailing-number identifiers, unsigned integer tokens, quoted strings, semantic `$` identifiers, common TPTP/FOF punctuation and operators, token tests, token descriptions, and position formatting.
- Shared basic parser helpers from `cio_basicparser`: booleans, signed and unsigned integer parsing, floats, number-string classification, double arrays, filenames, basic include syntax, dotted identifiers, continuous token spans, and balanced delimiter skipping.
- The core `CLStateGetOpt` command-line parser rules from `cio_commandline`: long options require `--name=value` for required arguments, long optional arguments default when `=` is absent, short required arguments accept attached or following values, short optional arguments use the default, `--` stops option parsing, and processed options are removed from the remaining argument list.
- Initial `eprover` handling for `--help`, `--version`, and a small option subset used by the compatibility harness setup path.

Known gaps:

- The prover core, parser, clausification, saturation loop, indexing, ordering, heuristics, proof output, resource limits, and most CLI options are not implemented yet.
- Scanner support is currently limited to string sources and does not yet implement file streams, stacked include handling, or `ScannerSetFormat` format inference.
- The help option table is intentionally partial until the full option table is ported.
- Running the Rust binary on a problem currently reports that proof search is not implemented.
