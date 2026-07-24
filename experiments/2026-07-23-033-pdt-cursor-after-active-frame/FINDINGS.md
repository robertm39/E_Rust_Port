# Experiment 271: Re-rank after the active-frame borrow

## Status

Diagnostic experiment for Bead `E_Rust_Port-j76.5.3`; production source is
unchanged.

## Question

After Experiment 270's active-frame borrow, where did the PD-tree cursor
saving occur, which costs remain, and is another fresh bounded owner now a
better target than another cursor representation change?

## Setup

- Source: commit `17d22969`.
- Accepted compact profile: 8,992,812,925 instructions.
- Diagnostic build: ordinary release optimization plus
  `CARGO_PROFILE_RELEASE_DEBUG=1`.
- Workload: exact LUSK6 under WSL Callgrind with `--auto --silent
  --cpu-limit=600 --memory-limit=2048 --detsort-rw --detsort-new`.
- Raw diagnostic profile:
  `.artifacts/experiments/2026-07-23-033-pdt-cursor-after-active-frame/rust-callgrind-pdt-cursor-after-active-frame.out`.

## Representativeness

The line-table binary preserves the exact `Unsatisfiable` proof and retires
8,994,036,876 instructions. It is 1,223,951 instructions or 0.013610% above
the accepted compact profile.

All optimized-source entries for
`search_next_matching_occurrence_impl::<true>` sum to exactly 1,560,083,792
instructions, identical to the compact accepted profile. The attribution is
therefore directly comparable.

## Active-frame result

Compared with the pre-candidate Experiment 269 attribution:

| Cursor attribution | Experiment 269 | Experiment 271 | Change |
| --- | ---: | ---: | ---: |
| Slice indexing | 138,042,494 | 99,509,979 | -38,532,515 (-27.914%) |
| `alloc::vec` | 254,796,834 | 279,175,517 | +24,378,683 (+9.568%) |
| Whole first-order cursor | 1,581,288,798 | 1,560,083,792 | -21,205,006 (-1.341%) |

The single active-frame borrow removes real checked-index work. LLVM
reattributes some of the resulting code to vector access, but the exact
cursor aggregate and whole-prover profile both improve.

The largest remaining visible cursor lines are the existing state machine:

- active node and terminal-state branch: 20,248,986 instructions each;
- next-step load and completion test: 20,138,314 plus 40,276,628;
- traversal-order selection and dispatch: 30,235,126 each;
- variable-link load/update: 29,475,486 plus 9,065,522;
- standalone `advance_variable_query`: 69,100,975 inclusive instructions.

These boundaries are not fresh candidates. Direct traversal phases,
traversal-order specialization, a terminal sentinel, widened variable scans,
cached query types, compact bindings, and forced variable-query inlining have
already been falsified by Experiments 194, 233-235, 253, and 259.

## Fresh bounded owner

The same profile records 1,571,112 calls and 124,117,451 exclusive
instructions in `VarBank::var_assert_alloc`. Its existing-variable path
retains C's release assertions but implements the type-identity assertion as:

```text
var.type_() == Some(type_.clone())
```

Both handles are already shared through a `TypeBank`, whose nonzero unique UID
is the identity key. Returning the stored type UID through the existing
non-owning accessor can preserve the release assertion while avoiding two
temporary `Rc<Type>` handles and the generic `Option<Type>` equality path.

Test that identity-only assertion as the next isolated candidate. Cache the
input UID once so the shared-type validity assertion and existing-variable
identity assertion do not read it twice. Preserve the paged variable table,
allocation path, public panics, shadow-bank behavior, and all fresh-variable
state.

This is distinct from Experiment 219's rejected structural term-ordering
borrow guards: it uses the existing scalar `type_uid()` API in one assertion
path and does not hold `Ref` guards across a comparator.

## Decision

Keep production source unchanged in Experiment 271. The active-frame result
is confirmed and the remaining cursor alternatives are exhausted enough that
the variable-bank type-identity assertion is the better fresh candidate.

## Reproduction

```bash
CARGO_PROFILE_RELEASE_DEBUG=1 cargo build --locked --release --bin eprover \
  --target-dir target-wsl-271-pdt-lines
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-pdt-cursor-after-active-frame.out \
  target-wsl-271-pdt-lines/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```
