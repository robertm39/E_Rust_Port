# Rust Port Status

This document tracks Rust implementation slices and the original C source units they are intended to mirror.

## Initial Crate And CLI Foundation

Rust files:

- `Cargo.toml`
- `src/basics/ddarrays.rs`
- `src/basics/dstacks.rs`
- `src/basics/dstrings.rs`
- `src/basics/error.rs`
- `src/basics/fixdarrays.rs`
- `src/basics/floattrees.rs`
- `src/basics/numtrees.rs`
- `src/basics/pdarrays.rs`
- `src/basics/pdrangearrays.rs`
- `src/basics/properties.rs`
- `src/basics/pstacks.rs`
- `src/basics/ptrees.rs`
- `src/basics/stringtrees.rs`
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
- [`BASICS/clb_fixdarrays.h`, `BASICS/clb_fixdarrays.c`](c_source_docs/BASICS/clb_fixdarrays.md)
- [`BASICS/clb_floattrees.h`, `BASICS/clb_floattrees.c`](c_source_docs/BASICS/clb_floattrees.md)
- [`BASICS/clb_numtrees.h`, `BASICS/clb_numtrees.c`](c_source_docs/BASICS/clb_numtrees.md)
- [`BASICS/clb_pdarrays.h`, `BASICS/clb_pdarrays.c`](c_source_docs/BASICS/clb_pdarrays.md)
- [`BASICS/clb_pdrangearrays.h`, `BASICS/clb_pdrangearrays.c`](c_source_docs/BASICS/clb_pdrangearrays.md)
- [`BASICS/clb_properties.h`](c_source_docs/BASICS/clb_properties.md)
- [`BASICS/clb_pstacks.h`, `BASICS/clb_pstacks.c`](c_source_docs/BASICS/clb_pstacks.md)
- [`BASICS/clb_ptrees.h`, `BASICS/clb_ptrees.c`](c_source_docs/BASICS/clb_ptrees.md)
- [`BASICS/clb_stringtrees.h`, `BASICS/clb_stringtrees.c`](c_source_docs/BASICS/clb_stringtrees.md)
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
- `PDRangeArr` signed-index dynamic arrays from `clb_pdrangearrays`, including low/limit key tracking, upward and downward expansion, C-compatible offset shifts, pointer-member counting, deletion, copying, and integer increment helpers.
- `FixedDArray` fixed-size integer vector helpers from `clb_fixdarrays`, including initialization, component-wise add/subtract/weighted-add/min/max, copying, and C-shaped debug printing.
- Property-bit helpers from `clb_properties`, including set/delete/flip/assign, all-bit and any-bit queries, masked property extraction, and masked equivalence checks.
- `StrTree` string-keyed map behavior from `clb_stringtrees`, including duplicate-preserving store semantics, lookup, mutable value rewrite, extraction, deletion, and deterministic sorted traversal.
- `FloatTree` floating-point-keyed map behavior from `clb_floattrees`, including duplicate-preserving store semantics, lookup, mutable value rewrite, extraction/deletion, node queries, deterministic sorted traversal for ordered float keys, signed-zero equivalence, infinities, and deterministic NaN bucketing.
- `NumTree` numeric-keyed map behavior from `clb_numtrees`, including duplicate-preserving store semantics, lookup, mutable value rewrite, extraction/deletion, root-like draining, node/max-key queries, debug printing, deterministic sorted traversal, and limited traversal starting at the first key greater than or equal to the limit.
- `PTree` pointer/identity-set behavior from `clb_ptrees`, including duplicate-preserving store semantics, lookup, binary lookup alias, extraction/deletion/root-like draining, destructive and non-destructive merge helpers, stack/vector conversion, copying, shared-element and intersection helpers, equivalence/subset checks, in-order visits, and debug printing.
- C-compatible numeric exit-code constants, including the duplicate `NO_ERROR`/`PROOF_FOUND` value.
- The `TestLetterString`/`CheckOptionLetterString` behavior from `clb_error`.
- Initial stream and scanner support for string sources, including C-compatible lookahead windows, line/column updates, token bit layout, whitespace/comment skipping, comment accumulation, identifiers and trailing-number identifiers, unsigned integer tokens, quoted strings, semantic `$` identifiers, common TPTP/FOF punctuation and operators, token tests, token descriptions, and position formatting.
- Shared basic parser helpers from `cio_basicparser`: booleans, signed and unsigned integer parsing, floats, number-string classification, double arrays, filenames, basic include syntax, dotted identifiers, continuous token spans, and balanced delimiter skipping.
- The core `CLStateGetOpt` command-line parser rules from `cio_commandline`: long options require `--name=value` for required arguments, long optional arguments default when `=` is absent, short required arguments accept attached or following values, short optional arguments use the default, `--` stops option parsing, and processed options are removed from the remaining argument list.
- Initial `eprover` handling for `--help`, `--version`, and a small option subset used by the compatibility harness setup path.

Known gaps:

- The prover core, parser, clausification, saturation loop, indexing, ordering, heuristics, proof output, resource limits, and most CLI options are not implemented yet.
- Scanner support is currently limited to string sources and does not yet implement file streams, stacked include handling, or `ScannerSetFormat` format inference.
- `StrTree` currently uses Rust `BTreeMap` to preserve sorted traversal and lookup semantics; the C implementation's splay-tree locality optimization should be revisited if profiling shows this index on hot paths.
- `FloatTree` currently uses Rust `BTreeMap` with an internal total-order float key; exact C splay-root locality and C's accidental behavior for NaN keys are not modeled beyond deterministic handling.
- `NumTree` currently uses Rust `BTreeMap` for the same reason; exact splay-root locality is not modeled beyond tracking a recent root-like key for extraction/draining.
- `PTree` currently uses Rust `BTreeSet` for deterministic ordered set semantics; exact C splay-root locality and pointer-address ordering should be revisited once stable arena/handle identity is wired into terms.
- The help option table is intentionally partial until the full option table is ported.
- Running the Rust binary on a problem currently reports that proof search is not implemented.
