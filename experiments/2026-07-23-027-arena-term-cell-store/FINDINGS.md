# Experiment 265: Reject an arena-backed term-cell store

## Status

Rejected in Experiment 265 for Bead `E_Rust_Port-j76.5.3`.

## Hypothesis

The accepted LUSK6 profile assigns 658,797,132 exclusive instructions to
`TermTree::insert`, versus approximately 333.6 million instructions for C
`TermTreeInsert` plus `splay_term_tree`. Rust stores owning `Term` handles in
each intrusive left/right link and mutates them through `RefCell`, while C
stores raw non-owning pointers.

Replace only the production `TermCellStore` buckets with one safe arena:

- one packed `u32` root per hash bucket;
- one arena node containing an owning `Term` plus packed left/right indices;
- a reusable vacant-slot list for extracted and garbage-collected nodes;
- the same top-down splay comparisons, rotations, hash buckets, and traversal
  order as the accepted intrusive tree.

The standalone `TermTree` API and `TermCell` representation remain unchanged.
This isolates whether safe indexed ownership can remove the hot `Rc` and
`RefCell` link traffic without committing to a broader term-cell redesign.

## Baseline

Accepted Experiment 261:

- Rust instructions: 9,106,424,013
- C instructions: 5,254,361,329
- Rust/C ratio: 1.733117
- accepted `TermTree::insert`: 658,797,132 exclusive instructions

## Candidate

`TermCellStore` owned a single `Vec<Option<StoreNode>>`, a packed root table,
and a reusable free-index vector. Find, insert, extract, delete, property
walks, enumeration, distribution reporting, and GC used the arena while
preserving the accepted bucket and tree traversal order.

A collision-heavy regression inserted 96 same-bucket terms, forced repeated
splays, extracted every third term, reused the vacant slots, and verified that
every key and maintained count survived. The seven store tests and all 479
term-domain tests passed. Strict all-feature library pedantic Clippy,
formatting, and diff checks passed.

Two release variants were measured:

1. an ordinary outlined arena splay;
2. a forced-inline arena splay, matching the accepted `TermTree` annotation
   that was previously justified by whole-prover profiling.

Both variants reached the exact 4,873-processed-clause LUSK6 proof, reported
`Unsatisfiable`, and exited zero.

## Measurement

The outlined arena retires 9,189,067,082 instructions:

- global delta: +82,643,069;
- global regression: +0.907525%;
- Rust/C ratio: 1.748846.

Its intended insertion boundary also regresses. `TermCellStore::insert` costs
450,665,967 instructions and `splay_store_tree` costs 436,101,514, for an
886,767,481-instruction aggregate. This is 227,970,349 instructions or
34.604029% above the accepted 658,797,132-instruction `TermTree::insert`.
Packed indexing, occupied-slot checks, and arena access cost more than the
optimized owning-link representation.

Forced inlining worsens the exact profile again to 9,196,986,661
instructions:

- global delta: +90,562,648;
- global regression: +0.994492%;
- Rust/C ratio: 1.750353.

The deterministic and intended-owner gates both reject the representation, so
native timing and compatibility/resource matrices are not proportionate.

## Result

Reject both arena variants. Restore `termcellstore.rs` and `termtrees.rs`
byte-for-byte to accepted Experiment 261 and remove the candidate regression.
The accepted baseline remains 9,106,424,013 instructions, or 1.733117 times C.

The raw profiles are retained at:

```text
.artifacts/experiments/2026-07-23-027-arena-term-cell-store/rust-callgrind-arena-term-cell-store.out
.artifacts/experiments/2026-07-23-027-arena-term-cell-store/rust-callgrind-arena-term-cell-store-inline.out
```

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-arena-term-cell-store.out \
  target-wsl-265-arena-term-cell-store/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```
