# Experiment 242: Recover the inserted term-store handle

## Question

Can term-bank top insertion consume its unshared candidate and recover the
stored handle, avoiding an input `Rc` clone on the 78.855% duplicate path
while retaining two strong owners for every fresh canonical cell?

## Baseline

- Accepted source: Experiment 231.
- Exact LUSK6 Callgrind: 9,923,564,772 instructions.
- Original C Callgrind: 5,254,361,329 instructions.
- Rust/C ratio: 1.888634.
- `TermBank::term_top_insert`: 260,571,383 exclusive instructions.
- Calls: 2,479,632 total, 1,955,273 duplicate, 524,359 fresh.

## Candidate

Add a crate-private fresh/duplicate insertion outcome to `TermCellStore`.
`TermBank::term_top_insert` captures the candidate properties, consumes the
candidate in the store call, and:

- merges the captured properties into the returned canonical handle for a
  duplicate; or
- initializes the returned fresh handle's bank metadata.

The existing public `TermCellStore::insert` API remains unchanged. The new
path clones the splay root once only after a fresh insertion, giving the tree
and bank one strong owner each. A focused regression verifies fresh and
duplicate outcomes, canonical identity, and unchanged store accounting.

## Validation

- `cargo fmt --all -- --check`
- `cargo clippy --lib --all-features -- -D warnings -D clippy::pedantic`
- Seven `terms::termcellstore::tests` passed, including the new outcome test.
- All 122 `terms::termbanks::tests` passed, including duplicate property
  merging, recursive metadata, garbage collection, parser, and higher-order
  insertion contracts.
- The deterministic candidate found the exact LUSK6 proof, reported
  `Unsatisfiable`, and exited zero.

## Measurement

Callgrind rises from 9,923,564,772 to 9,935,724,448 instructions: an increase
of 12,159,676 or 0.122533%. The implied C/Rust ratio worsens from 1.888634 to
1.890948.

The intended top-level owner improves, but lower ownership boundaries more
than reverse it:

| Exclusive owner | Baseline | Candidate | Change |
| --- | ---: | ---: | ---: |
| `TermBank::term_top_insert` | 260,571,383 | 251,312,568 | -9,258,815 |
| `TermCellStore::insert` / `insert_with_outcome` | 127,990,740 | 144,441,609 | +16,450,869 |
| `TermTree::insert` | 658,858,502 | 662,981,778 | +4,123,276 |
| Combined | 1,047,420,625 | 1,058,735,955 | +11,315,330 |

The three directly affected owners explain 93.06% of the whole-program
regression. Recovering the fresh root and carrying the outcome changes
insertion/splay code generation enough to cost more than the duplicate input
clone it removes.

## Result

Reject. The candidate preserves tested ownership, metadata, higher-order, and
proof behavior, and it reduces the intended TermBank owner. The lower store
and tree regressions are larger and make the complete prover slower.

Native timing and compatibility matrices were skipped after the deterministic
gate failed. All candidate production code and tests were removed; accepted
Experiment 231 is restored byte-for-byte at 9,923,564,772 instructions and
1.888634 times C.

Raw evidence:

```text
.artifacts/experiments/2026-07-23-004-recover-term-store-insert/rust-callgrind-recover-term-store-insert.out
```

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-recover-term-store-insert.out \
  target-wsl-242-recover-term-store-insert/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```
