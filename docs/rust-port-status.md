# Rust Port Status

This document tracks Rust implementation slices and the original C source units they are intended to mirror.

## Initial Crate And CLI Foundation

Rust files:

- `Cargo.toml`
- `src/basics/ddarrays.rs`
- `src/basics/defines.rs`
- `src/basics/dstacks.rs`
- `src/basics/dstrings.rs`
- `src/basics/error.rs`
- `src/basics/fixdarrays.rs`
- `src/basics/floattrees.rs`
- `src/basics/intmap.rs`
- `src/basics/min_heap.rs`
- `src/basics/numtrees.rs`
- `src/basics/numxtrees.rs`
- `src/basics/pdarrays.rs`
- `src/basics/pdrangearrays.rs`
- `src/basics/permastrings.rs`
- `src/basics/plist.rs`
- `src/basics/pqueue.rs`
- `src/basics/properties.rs`
- `src/basics/simple_stuff.rs`
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
- [`BASICS/clb_defines.h`](c_source_docs/BASICS/clb_defines.md)
- [`BASICS/clb_dstrings.h`, `BASICS/clb_dstrings.c`](c_source_docs/BASICS/clb_dstrings.md)
- [`BASICS/clb_dstacks.h`, `BASICS/clb_dstacks.c`](c_source_docs/BASICS/clb_dstacks.md)
- [`BASICS/clb_error.h`, `BASICS/clb_error.c`](c_source_docs/BASICS/clb_error.md)
- [`BASICS/clb_fixdarrays.h`, `BASICS/clb_fixdarrays.c`](c_source_docs/BASICS/clb_fixdarrays.md)
- [`BASICS/clb_floattrees.h`, `BASICS/clb_floattrees.c`](c_source_docs/BASICS/clb_floattrees.md)
- [`BASICS/clb_intmap.h`, `BASICS/clb_intmap.c`](c_source_docs/BASICS/clb_intmap.md)
- [`BASICS/clb_min_heap.h`, `BASICS/clb_min_heap.c`](c_source_docs/BASICS/clb_min_heap.md)
- [`BASICS/clb_numtrees.h`, `BASICS/clb_numtrees.c`](c_source_docs/BASICS/clb_numtrees.md)
- [`BASICS/clb_numxtrees.h`, `BASICS/clb_numxtrees.c`](c_source_docs/BASICS/clb_numxtrees.md)
- [`BASICS/clb_pdarrays.h`, `BASICS/clb_pdarrays.c`](c_source_docs/BASICS/clb_pdarrays.md)
- [`BASICS/clb_pdrangearrays.h`, `BASICS/clb_pdrangearrays.c`](c_source_docs/BASICS/clb_pdrangearrays.md)
- [`BASICS/clb_permastrings.h`, `BASICS/clb_permastrings.c`](c_source_docs/BASICS/clb_permastrings.md)
- [`BASICS/clb_plist.h`, `BASICS/clb_plist.c`](c_source_docs/BASICS/clb_plist.md)
- [`BASICS/clb_pqueue.h`, `BASICS/clb_pqueue.c`](c_source_docs/BASICS/clb_pqueue.md)
- [`BASICS/clb_properties.h`](c_source_docs/BASICS/clb_properties.md)
- [`BASICS/clb_simple_stuff.h`, `BASICS/clb_simple_stuff.c`](c_source_docs/BASICS/clb_simple_stuff.md)
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
- Shared `IntOrP` integer/pointer payload shape from `clb_defines`, represented as a checked Rust enum for containers that mix long integer tags and pointer-like handles.
- `IntMap` multi-representation integer map behavior from `clb_intmap`, including empty/single/array/tree states, density-triggered representation switching, `get_ref` slot creation with null values, assignment and deletion return behavior, entry-count inflation for repeated null array references, inclusive sorted range iteration over non-null values, and debug printing.
- `MinHeap` binary minimum heap behavior from `clb_min_heap`, including comparator-driven ordering, integer/pointer-shaped add helpers, minimum pop, size/peek queries, update/remove operations, the C helper directions for `decr_key` and `incr_key`, optional index-setter callbacks after swaps/removals, and debug printing.
- Permanent string registry behavior from `clb_permastrings`, including duplicate interning, owned-string store, null-shaped optional lookup, explicit global registry clearing, and returned shared strings that remain valid for Rust holders after the registry is cleared.
- `PList` doubly linked list behavior from `clb_plist`, including explicit anchor cells, insertion after arbitrary list cells, extraction and reinsertion across anchors, deletion, clearing/freeing anchors, forward/backward navigation, and mixed integer/pointer payload helpers.
- `PQueue` circular pointer/integer queue behavior from `clb_pqueue`, including head/tail indexing, FIFO extraction, stack-view last extraction, bury-at-front insertion, C-shaped full-ring growth layout, reset without shrinking, absolute tail/index iteration, and mixed integer/pointer helper values.
- Property-bit helpers from `clb_properties`, including set/delete/flip/assign, all-bit and any-bit queries, masked property extraction, and masked equivalence checks.
- Shared simple helpers from `clb_simple_stuff`, including bytewise string distance, weighted-object comparison/sorting, C-shaped JKISS random state and static-state wrapper behavior, bounded indentation strings, prefix tests, null-terminated string-index/cardinality helpers, positive-only GCD, `ProverResult`, `ProblemType`, and first-order/higher-order syntax conflict checks.
- `StrTree` string-keyed map behavior from `clb_stringtrees`, including duplicate-preserving store semantics, lookup, mutable value rewrite, extraction, deletion, and deterministic sorted traversal.
- `FloatTree` floating-point-keyed map behavior from `clb_floattrees`, including duplicate-preserving store semantics, lookup, mutable value rewrite, extraction/deletion, node queries, deterministic sorted traversal for ordered float keys, signed-zero equivalence, infinities, and deterministic NaN bucketing.
- `NumTree` numeric-keyed map behavior from `clb_numtrees`, including duplicate-preserving store semantics, lookup, mutable value rewrite, extraction/deletion, root-like draining, node/max-key queries, debug printing, deterministic sorted traversal, and limited traversal starting at the first key greater than or equal to the limit.
- `NumXTree` numeric-keyed four-slot value map behavior from `clb_numxtrees`, including duplicate-preserving store semantics with defaulted extra slots, full-entry insertion, lookup, mutable value-slot rewrite, extraction/deletion, root-like draining, node/max-key queries, deterministic sorted traversal, and limited traversal starting at the first key greater than or equal to the limit.
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
- `IntMap` preserves the C representation-state decisions but uses safe Rust-owned storage; the C implementation's hidden `PDRangeArr` growth during some read/delete paths and inflated null-slot entry counts should be evaluated later as compatibility risks versus cleanup opportunities.
- `PList` uses safe arena handles instead of raw self-linked pointers; this preserves cell moves between anchors but does not yet reuse freed cell slots like the C allocator/free-list path.
- `PQueue` preserves the observable circular-buffer behavior, but the C implementation exposes `PQueueGrow` even though its copy logic is only valid after a store/bury creates a full ring; the Rust port keeps growth internal until a real direct caller appears.
- `StrTree` currently uses Rust `BTreeMap` to preserve sorted traversal and lookup semantics; the C implementation's splay-tree locality optimization should be revisited if profiling shows this index on hot paths.
- `FloatTree` currently uses Rust `BTreeMap` with an internal total-order float key; exact C splay-root locality and C's accidental behavior for NaN keys are not modeled beyond deterministic handling.
- `NumTree` currently uses Rust `BTreeMap` for the same reason; exact splay-root locality is not modeled beyond tracking a recent root-like key for extraction/draining.
- `NumXTree` currently uses Rust `BTreeMap` for the same reason; exact splay-root locality is not modeled beyond tracking a recent root-like key for extraction/draining.
- `PTree` currently uses Rust `BTreeSet` for deterministic ordered set semantics; exact C splay-root locality and pointer-address ordering should be revisited once stable arena/handle identity is wired into terms.
- Permanent strings are represented as `Arc<str>` rather than raw `char*`; duplicate calls preserve shared allocation identity, while clearing the registry does not invalidate existing Rust handles.
- The JKISS wrapper preserves the exported C module's static-state behavior, including the fact that the `JKISSRand` state argument does not drive the random sequence; call-site-level compatibility should be revisited when random-dependent heuristics are ported.
- The help option table is intentionally partial until the full option table is ported.
- Running the Rust binary on a problem currently reports that proof search is not implemented.

## C Behaviors To Revisit After Compatibility

These notes are not permission to diverge during porting. They identify inherited C behaviors that may be good cleanup candidates after the Rust executable is demonstrably drop-in compatible.

- `DStr`, `PStack`, `DStack`, `PDArray`, `DDArray`, and `PDRangeArr` preserve C growth and mutating-access patterns; later APIs may want clearer separation between read-only access and access that allocates or extends storage.
- `PStack`/`DStack` discard helpers intentionally use swap-remove behavior, which is efficient but order-destroying; keep auditing callers before exposing order-preserving variants.
- `PList` raw-pointer anchors and extracted-cell ownership are represented with checked arena handles; after list-heavy clause/formula code is ported, revisit whether a generational freelist or typed owner should replace the current simple slot model.
- `PQueue` exposes absolute internal ring indices for iteration and C has a public grow routine that is only coherent for the full-ring state; later callers should keep those details encapsulated behind safer traversal APIs.
- `FixedDArray` currently mirrors C assertion-style size contracts; callers fed by user input may eventually need recoverable error paths instead of invariant panics.
- `MinHeap` preserves the C helper directions where `decr_key` drops down and `incr_key` bubbles up, despite those names appearing reversed for a conventional min-heap. Rename or wrap only after all heap users are audited.
- `StrTree`, `FloatTree`, `NumTree`, `NumXTree`, and `PTree` model ordered semantics with safe Rust containers while documenting splay-locality gaps; hot indexing paths should be benchmarked before deciding whether to recreate splay behavior.
- `IntMap` keeps C density transitions and null-slot quirks, but its hidden read/delete-time array growth in C is a likely cleanup target if compatibility tests show no observable dependency.
- Permanent strings use `Arc<str>` to keep Rust handles valid after registry clearing; later term/signature ownership should decide whether stable arena handles are a better identity model.
- `clb_simple_stuff` includes historical global-state behavior in `ProblemType` and JKISS random generation; the Rust port should centralize these states before parallel parsing or solving is introduced.
- Error handling is currently represented with structured `Diagnostic` values in many Rust paths; final executable compatibility still needs exact fatal-error stream, wording, and exit-status behavior.
- Scanner and command-line code intentionally preserve C quirks already covered by tests, while remaining incomplete for file/include stacks, format inference, and the full option table.
