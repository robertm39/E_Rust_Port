# Experiment 272: Reject scalar variable-bank type assertions

## Status

Rejected performance candidate for Bead `E_Rust_Port-j76.5.3`; accepted
production source is restored.

## Question

Can `VarBank::var_assert_alloc` preserve its release-time existing-variable
type assertion by comparing unique shared type UIDs instead of cloning and
dropping two `Rc<Type>` handles for `Option<Type>` equality?

## Candidate

The candidate reads the input type UID once, checks it against
`INVALID_TYPE_UID`, and compares the existing variable's scalar UID with that
value. It preserves:

- the negative function-code assertion;
- the requirement that input types be shared through a `TypeBank`;
- the existing variable's `v_count == 1` assertion;
- rejection of an existing code with a different shared type;
- the paged variable table, allocation path, shadow bank, and fresh-variable
  state.

A focused regression allocates one code at the individual sort and confirms
that requesting the same code at a different shared user sort still panics.
The candidate uses the existing `Term::type_uid()` accessor and holds no
borrow guard across another operation.

## Deterministic result

The candidate passes all 18 focused variable-bank tests with default and all
features, passes strict all-feature library pedantic Clippy, preserves the
exact LUSK6 proof, and exits zero under Callgrind.

Exact instructions fall from 8,992,812,925 to 8,974,523,143:

- global delta: -18,289,782;
- global improvement: 0.203382%;
- `VarBank::var_assert_alloc`: 124,117,451 to 106,835,296;
- owner delta: -17,282,155, or -13.924033%;
- hypothetical Rust/C ratio: 1.708014.

The raw candidate profile is:

```text
.artifacts/experiments/2026-07-23-034-scalar-varbank-type-assertion/rust-callgrind-scalar-varbank-type-assertion.out
```

## Native result

The accepted and candidate Windows executables are both 8,952,320 bytes,
exit zero, and emit byte-identical standard output and standard error. Four
alternating warmup pairs were excluded. All 128 measured processes prove and
exit zero.

Across 64 alternating measured pairs, every distribution statistic rejects
the candidate:

- wall means regress 0.795113%, from 1.533837 to 1.546032 seconds;
- CPU means regress 0.912349%, from 1.498535 to 1.512207 seconds;
- wall and CPU medians regress 1.191244% and 1.554404%;
- mean paired wall and CPU changes regress 0.972846% and 1.059046%;
- median paired wall and CPU changes regress 1.134202% and 1.030928%;
- the candidate wins only 28 wall and 24 CPU pairs, with six CPU ties.

The final 32 pairs remain negative:

- wall and CPU means regress 0.704696% and 0.426929%;
- wall median regresses 1.612123%, while CPU medians tie;
- mean paired wall and CPU changes regress 0.921244% and 0.607791%;
- the candidate wins 14 wall and 12 CPU pairs, with four CPU ties.

Raw warmup and measured rows are in `native-warmup.csv` and
`native-lusk.csv`.

## Validation and restoration

- The candidate passes 18 focused tests in both feature modes, including the
  new mismatched-type regression.
- Strict all-feature library pedantic Clippy and formatting pass.
- Direct, Callgrind, warmup, and all measured native processes prove exactly
  and exit zero.
- Compatibility matrices and full repository gates are skipped after the
  decisive native rejection.
- Candidate production code and its candidate-specific regression are
  removed.
- The accepted `src/terms/termvars.rs` is restored byte-for-byte.
- All 17 accepted variable-bank tests and formatting pass after restoration.
- The original `eprover/` checkout remains untouched.

## Decision

Reject. Scalar UID comparison removes real reference-count and generic
`Option<Type>` work under Callgrind, but native wall and CPU time regress
consistently across the full sample and stable half. Preserve the owning type
assertion and the accepted Experiment 270 baseline at 8,992,812,925
instructions, or 1.711495 times C.

This result is narrower than Experiment 219's rejected borrowed structural
type comparison, but it confirms the same practical lesson: eliminating
temporary type ownership in isolation changes native code generation
unfavorably on this host.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-scalar-varbank-type-assertion.out \
  target-wsl-272-varbank-type-uid/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```

```powershell
& .\experiments\2026-07-22-214-move-termtree-insert-links\run-native.ps1 `
  -ParentExe .\target\native-270-borrow-active-pdt-frame\release\eprover.exe `
  -CandidateExe .\target\native-272-scalar-varbank-type-assertion\release\eprover.exe `
  -Pairs 64 `
  -OutputCsv .\experiments\2026-07-23-034-scalar-varbank-type-assertion\native-lusk.csv
```
