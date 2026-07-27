# Experiment 295: Current proof-search line attribution

## Status

Diagnostic experiment for Bead `E_Rust_Port-j76.5.3`; production source is
unchanged.

## Question

After the accepted proof-search optimizations through Experiment 293, which
remaining Rust/C gap is both large and not already covered by a rejected
ownership, representation, or forced-inlining experiment?

## Baseline

- Accepted source: commit `b6f59ac1` (`perf: fuse always-deref app check`);
  Experiment 294 restored that source byte-for-byte.
- Exact default-feature LUSK6 Callgrind:
  `8,718,487,029` instructions.
- Original FOL C Callgrind: `5,254,361,329` instructions.
- Exact Rust/C ratio: `1.659286`.
- Deterministic workload: upstream `LUSK6.lop` with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.

The accepted source was rebuilt with release optimization, default features,
and line tables:

```text
CARGO_PROFILE_RELEASE_DEBUG=1
CARGO_TARGET_DIR=target-wsl-295-current-line-attribution
cargo build --locked --release --bin eprover
```

The fingerprint records exactly `features=["default"]`. The line-table binary
proves the problem with the expected `Unsatisfiable` result and retires
`8,718,868,051` instructions. That is only `381,022` instructions, or
`0.004370%`, above the accepted exact build, so its attribution is
representative.

## Comparative profile

The largest exact Rust owners remain concentrated:

| Boundary | Rust instructions | C instructions | Rust/C |
| --- | ---: | ---: | ---: |
| PD-tree matching cursor | 1,560,083,792 | 1,119,601,083 | 1.393 |
| Term-tree insertion/splay | 643,023,396 | about 333,621,671 | 1.928 |
| Substitution normalization | 444,445,091 | 192,675,144 | 2.307 |
| Replacement insertion | 300,626,087 | 175,848,570 | 1.710 |
| Term-bank top insertion | 260,571,383 | 127,261,512 | 2.048 |

Those boundaries have already received current line profiles and multiple
bounded ownership, representation, traversal, and inlining trials. The fresh
profile instead exposes a narrower C-invariant mismatch in the variable bank.

`VarBank::var_assert_alloc` retires `127,259,653` instructions across
`1,571,112` calls. Exactly `1,571,101` calls find an existing variable and
only `11` allocate. Line attribution splits the owner as follows:

| Attributed component | Instructions |
| --- | ---: |
| `termvars.rs` | 47,133,279 |
| `Cell`/`RefCell` operations | 21,995,469 |
| `Option` operations | 15,711,060 |
| move/drop machinery | 15,711,032 |
| slice indexing | 7,855,557 |
| signed-integer machinery | 6,284,437 |
| `Rc`, intrinsics, and unsigned-integer machinery | 9,426,606 |
| non-null and pointer machinery | 3,142,213 |
| **Total** | **127,259,653** |

The original `VarBankVarAssertAlloc` performs its negative-code, variable
count, type-presence, and identical-type checks with C `assert()` calls. They
are absent from the maintained optimized C executable under `NDEBUG`. Rust
currently evaluates corresponding unconditional release assertions,
including cloning and comparing the existing variable's shared type on the
99.9993% existing-variable path.

This is distinct from Experiment 219's borrowed active type comparison and
Experiment 272's scalar UID comparison: both deliberately preserved the
release assertion and changed how it was evaluated. It is also distinct from
Experiment 218's rejected dense fresh-counter representation. The untested
boundary is whether the port should match C's release/debug assertion
semantics and remove the check entirely from optimized execution.

## Result

Diagnostic only. Production source remains byte-identical to accepted
Experiment 293.

Experiment 296 should convert only the C-assert-equivalent variable-bank
preconditions and existing-variable invariants to Rust `debug_assert!` /
`debug_assert_eq!`. Debug builds and the normal test suite must retain the
diagnostics. The variable table, ordered per-type maps, fresh-variable
algorithm, shadow-bank behavior, and public return values must remain
unchanged.

Raw evidence:

```text
.artifacts/experiments/2026-07-24-022-current-line-attribution/callgrind-current-lines.out
```

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=callgrind-current-lines.out \
  target-wsl-295-current-line-attribution/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```

```bash
callgrind_annotate --inclusive=no --threshold=100 --auto=no \
  callgrind-current-lines.out
```
