# PCL mini-clause ownership and rendering audit

## Status

Completed for Bead `E_Rust_Port-j76.2.127`. Rust's owning compact literal vector
is an evidence-backed replacement for C's borrowed parallel arrays, and the
temporary-clause rendering path retains output compatibility. The vendored C
source remained unchanged.

## Representation and lifetime

C stores a signed `short literal_no`, a separately allocated `short` sign array,
and a separately allocated array containing two borrowed `Term_p` pointers per
literal. The mini clause does not own term cells; correctness relies on the term
bank outliving it. Allocation, destruction, and loop bounds all trust the
truncated count.

Rust stores one `Vec<MiniLiteral>`. Each literal owns its sign and two cloned
`Term` handles. `Term` equality is shared-cell identity, so cloning retains the
same banked term cells rather than structural copies. The vector has one owner,
one allocation, one authoritative length, and ordinary safe destruction. The
owning mini protocol still supplies its term bank for symbol/type rendering.

Focused coverage constructs 32,768 compact literals, one beyond signed `short`
maximum, and requires the exact count. C's wraparound cannot be a valid
compatibility behavior: it makes allocation sizes and subsequent loops disagree
with the source clause.

## Metadata and conversion

Both implementations snapshot only literal signs and term pairs. C's proposed
properties field is commented out, and `MiniClauseToClause` returns a fresh
`ClauseAlloc` object. Rust reconstruction likewise starts from
`CPIgnoreProps`; a negated-conjecture source therefore reconstructs with unknown
role while retaining literal order, signs, shared terms, and printed core.

Owned `minify`/`unminify` operations encode C's consume-and-free contracts in
the type system. The unused, non-header C `MiniClauseAddTerms` helper has no
vendored callers and does not justify a second Rust API.

## Rendering and performance

C's three printers reconstruct a temporary full clause, call the ordinary
printer, and free the temporary. Rust intentionally follows the same path,
reusing the ported clause printers and their literal/type behavior. Output
format, problem type, and equation options are explicit arguments rather than
process globals; LOP/TPTP/LOP regression rendering proves call isolation.

Reconstruction allocates temporary equation cells in both implementations.
Direct compact rendering could remove that work, but it risks duplicating a
large and compatibility-sensitive print surface. It remains deferred until
proof-output profiling demonstrates that this path is material.

Post-compatibility alternatives remain tracked by Beads
`E_Rust_Port-j76.4.943` through `.948`.

## Validation

- focused mini-clause tests cover term identity, sign/count preservation,
  metadata loss, owned conversions, the signed-short boundary, empty clauses,
  exact PCL/TSTP-core/LOP rendering, format dispatch, and call isolation;
- formatting and strict Clippy pass;
- full all-target/all-feature tests pass; and
- source-document coverage, Change Later wording, links, and regeneration
  preservation pass.
