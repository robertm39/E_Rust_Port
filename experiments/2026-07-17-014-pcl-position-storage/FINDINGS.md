# PCL-position term-path storage audit

## Status

Completed for Bead `E_Rust_Port-j76.2.130`. Rust's `Vec<i64>` is the
evidence-backed safe replacement for C's nullable `PDArray` term path, while
the legacy printer remains byte-shaped like C. The vendored C source remained
unchanged.

## C behavior

`PCL2PosCell` stores both `termposlen` and a nullable `PDArray_p`. Allocation
starts with `termpos == NULL` and length zero. Parsing allocates
`PDArrayAlloc(5,10)` only after it sees the first dotted component, then assigns
components by index and separately updates the length. Printing trusts the
length, asserts the array is non-null for every component, and concatenates the
decimal values without dots.

The pointer itself has no observable semantic state: all reads are bounded by
`termposlen`, and the supported allocator/parser path uses null exactly for an
empty term path.

## Rust equivalence

`Pcl2Position::termpos` is `Vec<i64>`. Its length replaces C's manually
synchronized `termposlen`; an empty vector represents the null/zero state and
cannot disagree with its storage. The empty allocation and side-only parse both
retain capacity zero, so Rust also performs no heap allocation until the first
component is pushed.

The printer deliberately does not clean up the C syntax. The strengthened
multi-digit regression parses `3.L.12.5` and requires `3.L125`, pinning both the
missing separators and the resulting ambiguity. The expression opening-bracket
mismatch and the printer's failure to round-trip are already tracked by
post-compatibility Beads `E_Rust_Port-j76.4.931`, `.960`, and related revisit
tasks.

## Performance

Both implementations avoid allocation for the common empty/side-only path and
store populated components contiguously. Rust may choose a different capacity
growth sequence than C's initial five slots followed by increments of ten, but
the component counts are normally tiny and parsing is not a demonstrated hot
path. Exact capacity emulation would add custom storage without semantic or
measured throughput benefit.

## Validation

- focused `pcl2::positions` tests cover defaults, allocation boundaries,
  literal-only and sided forms, multi-digit term paths, diagnostics, and exact
  legacy rendering;
- formatting and strict Clippy pass;
- full all-target/all-feature tests pass; and
- source-document coverage, Change Later wording, links, and regeneration
  preservation pass.
