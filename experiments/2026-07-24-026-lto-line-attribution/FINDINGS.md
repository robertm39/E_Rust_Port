# Experiment 299: Fat-LTO proof-search line attribution

## Status

Diagnostic experiment for Bead `E_Rust_Port-j76.5.3`; production source is
unchanged.

## Question

After Experiment 298 accepted fat LTO with one codegen unit, which remaining
Rust/C gap is still represented by an out-of-line Rust boundary, rather than
an owner that the linker has already merged and optimized?

## Baseline

- Accepted source and release profile: commit `51aa9926`
  (`perf: enable whole-program release optimization`).
- Exact default-feature LUSK6 Callgrind: `8,400,364,984` instructions.
- Original FOL C Callgrind: `5,254,361,329` instructions.
- Exact Rust/C ratio: `1.598741`.
- Deterministic workload: upstream `LUSK6.lop` with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.

The accepted source was rebuilt with the manifest's fat-LTO and single-CGU
profile plus release line tables:

```text
CARGO_PROFILE_RELEASE_DEBUG=1
CARGO_TARGET_DIR=target-wsl-299-lto-line-attribution
cargo build --locked --release --bin eprover
```

Both executable and library fingerprints record exactly
`features=["default"]`. The line-table binary proves the problem with the
expected `Unsatisfiable` result and retires `8,400,450,231` instructions.
That is only `85,247` instructions, or `0.001015%`, above the accepted exact
build, so its attribution is representative.

## LTO attribution

Whole-program optimization materially changes attribution. In particular,
the former standalone term-tree insertion owner is now merged into
`TermBank::term_top_insert`, whose cost is spread across `termtrees.rs`,
`termtypes.rs`, `termbanks.rs`, `Cell`, `Option`, comparison, and move
machinery. The exact whole-program total remains authoritative when comparing
candidates.

The principal source owners are still the PD-tree cursor, merged term-bank
insertion, substitution normalization, rewriting, structural comparison, and
allocation. The refreshed line profile also confirms these specific
post-LTO boundaries:

| Boundary or source component | Instructions |
| --- | ---: |
| `Term::arguments` | 56,190,374 |
| `Term::arguments` calls | 3,127,396 |
| `TermArgs::as_slice` source lines, across inlined owners | 262,661,138 |
| `Term::is_applied_free_var` | 13,451,950 |
| `VarBank::var_assert_alloc`, `termvars.rs` portion | 56,560,155 |

Fat LTO reduced `Term::arguments` from the pre-LTO 65,174,512 instructions to
56,190,374, but it did not inline the boundary. All 3,127,396 calls are still
the four left/right argument borrows in structural-weight comparison. This is
the clearest small remaining boundary whose prior production measurement was
made under the now-obsolete pre-LTO code-generation regime.

The profile also rules out `panic = "abort"` as a transparent release-profile
candidate: proof-state execution intentionally uses `catch_unwind` and
`resume_unwind`, so abort semantics would remove behavior required by the
current executable.

## Result

Diagnostic only. Production source remains byte-identical to accepted
Experiment 298.

Experiment 300 should repeat Experiment 294's single
`#[inline(always)]` attribute on `Term::arguments`, but build and measure it
under the accepted fat-LTO and single-CGU manifest profile. It must be judged
from fresh deterministic and native production measurements; the pre-LTO
native rejection is not authoritative for the new whole-program optimizer.

Raw evidence:

```text
.artifacts/experiments/2026-07-24-026-lto-line-attribution/callgrind-lto-lines.out
.artifacts/experiments/2026-07-24-026-lto-line-attribution/callgrind-lto-lines.annotate.txt
.artifacts/experiments/2026-07-24-026-lto-line-attribution/callgrind-lto-hot-sources.txt
```

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=callgrind-lto-lines.out \
  target-wsl-299-lto-line-attribution/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```

```bash
callgrind_annotate --inclusive=no --threshold=100 --auto=no \
  callgrind-lto-lines.out
```
