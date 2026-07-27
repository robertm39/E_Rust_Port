# Term-tree line attribution

## Question

Which safe Rust operations dominate the accepted, fully inlined term-tree
insertion path, and can optimized line tables expose a bounded next candidate
without materially changing the deterministic workload?

## Setup

- Source: commit `d72a7537` (`Record rejected PD-tree argument borrow`), whose
  executable source is accepted Experiment 214.
- Build: the ordinary release profile with `CARGO_PROFILE_RELEASE_DEBUG=1`;
  this adds line tables without changing source or optimization level.
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Accepted profile:
  `.artifacts/experiments/2026-07-22-214-move-termtree-insert-links/rust-callgrind-move-termtree-insert-links.out`.
- Line-table profile:
  `.artifacts/experiments/2026-07-22-216-termtree-line-attribution/rust-callgrind-termtree-lines.out`.

## Representativeness

The line-table binary reaches the expected LUSK6 proof and retires
10,633,253,406 instructions. This is only 612,021 instructions or 0.005756%
above the accepted 10,632,641,385-instruction profile, so its attribution is
representative of the pinned optimized binary.

## Attribution

Inlining assigns the term-tree insertion aggregate across its own source,
`Term` accessors, and standard-library machinery. The largest categories are:

| Attributed component | Instructions |
| --- | ---: |
| `termtrees.rs` | 217,044,541 |
| `termtypes.rs` | 97,409,795 |
| `Cell`/`RefCell` operations | 69,958,186 |
| comparison machinery | 64,229,537 |
| `Option` operations | 57,366,430 |
| argument `zip` adapter | 52,603,568 |
| move/drop machinery | 48,539,899 |

The new owned insertion links from Experiment 214 execute 491,585 distinct
left/right branches. Within splaying, 702,328 zig-zig rotations and 334,845
tail-link extensions are visible. Final assembly performs 722,549 tail-link
updates. These counts confirm that safe link mutation and reference ownership
are intrinsically hot, but they do not reveal another redundant clone after
Experiment 214.

The argument comparator still uses
`left_arguments.iter().zip(right_arguments.iter())` after proving equal
lengths. Its 52.6-million-instruction adapter category is a bounded candidate:
the upstream C loop indexes both equal-length argument arrays with one integer.
An indexed Rust loop can test that shape without changing keys, pointer
identity, type handling, or tree topology.

## Decision

Keep executable source unchanged. Use this representative profile to test an
indexed equal-length argument loop as a separate candidate. Do not infer that
all `RefCell` or ownership costs are removable: the safe port must retain live
aliases that C represents with raw pointers.

## Reproduction

```bash
CARGO_PROFILE_RELEASE_DEBUG=1 cargo build --locked --release --bin eprover \
  --target-dir target-wsl-216-termtree-line-attribution
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-termtree-lines.out \
  target-wsl-216-termtree-line-attribution/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```
