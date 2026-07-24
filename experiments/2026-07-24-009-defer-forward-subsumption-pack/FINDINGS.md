# Experiment 282: Defer forward-subsumption packing

## Status

Rejected for Bead `E_Rust_Port-j76.5.3`.

## Question

Can production forward contraction consume only the subsumed/not-subsumed
decision and defer Rust's owned `FvPackedClause` construction until the final
clause state, avoiding a clause/frequency-vector clone that both production
callers immediately discard?

## Setup

- Parent source: commit `21f263e0` (`perf: reject no-doc forward
  specialization`); executable source remains accepted Experiment 270.
- Fresh unchanged-source default-feature control: 8,991,960,325
  instructions.
- Archived accepted default-feature profile: 8,992,812,925 instructions.
- Representative optimized line profile:
  `.artifacts/experiments/2026-07-23-033-pdt-cursor-after-active-frame/rust-callgrind-pdt-cursor-after-active-frame.out`.
- Parent native executable:
  `target/native-270-borrow-active-pdt-frame/release/eprover.exe`.
- Candidate: factor the bank-aware subsumption decision into a private Boolean
  helper. The public packed-return API retains its exact contract, while
  ordinary and aggressive production contraction use the decision directly.
- Variant B prevents the factored decision helper from being duplicated into
  its packed and decision-only callers, testing whether code expansion causes
  any whole-program reversal.
- Workload: upstream `LUSK6.lop` with `--auto --silent --cpu-limit=600
  --memory-limit=2048 --detsort-rw --detsort-new`.

Original C's `FVPackedClause` aliases the live clause and is retained through
later selection/maximality mutations. Rust's safe packed value owns a clause
clone, so the existing production caller discards the early clone and packs
again after mutation. The accepted profile records 1,700 bank-aware
forward-subsumption calls and about 14,030,040 inclusive instructions at this
boundary.

## Results

### Factored decision

The original 9,074,044,057-instruction candidate profile was accidentally
built with `--all-features` and is not comparable with the default-feature
accepted baseline. A configuration audit measured the unchanged source at
9,078,864,096 instructions with all features versus 8,991,960,325 with
default features, a build-configuration penalty of 86,903,771 instructions
or 0.966461%. The original rejection metric is therefore superseded.

The corrected default-feature candidate executes 8,988,052,441 instructions.
That is 3,907,884 or 0.043460% below the fresh unchanged-source control and
4,760,484 or 0.052937% below the archived accepted profile. Its hypothetical
ratio to the 5,254,361,329-instruction C reference improves to 1.710589.

The first native correction used an all-feature candidate against the
default-feature parent and is superseded by Experiment 285's native feature
audit. The matched default-feature candidate is 8,950,272 bytes, 2,048 bytes
smaller than the 8,952,320-byte parent. Three parent and five candidate direct
proof runs are all byte-identical and exit zero.

After four alternating warmup pairs, 64 alternating measured native pairs
reverse the small instruction win:

- wall and CPU means regress 0.918190% and 1.199080%;
- wall and CPU medians regress 0.545908% and 1.036269%;
- mean paired wall and CPU changes regress 1.182444% and 1.504408%;
- the candidate wins 26 wall pairs and 22 CPU pairs, with six CPU ties.

The final 32 pairs remain negative at 1.579561% wall and 2.214144% CPU by
aggregate means. The final 16 regress 0.977337% wall and 1.259947% CPU. All
128 measured and eight warmup processes exit zero.

### Out-of-line decision

Variant B applies `#[inline(never)]` to the factored decision helper. Its
original 9,073,719,121-instruction result was also built with all features and
is invalid as a comparison to the accepted default-feature baseline. It was
not rerun because its prior difference from Variant A was only 324,936
instructions while corrected Variant A fails native timing by roughly ten
percent.

## Validation

- All 219 proof-control tests pass in default and all-feature configurations.
- The public packed-return regression retains its exact contract.
- Strict all-feature library pedantic Clippy and formatting pass for Variant
  A.
- Corrected default-feature WSL Callgrind for Variant A proves LUSK6 and exits
  zero.
- The matched candidate fingerprint records exactly `features=["default"]`.
- Three parent and five candidate direct native proof runs are byte-identical
  and exit zero.
- All matched-feature native timing processes prove and exit zero.
- The full maintained compatibility matrix is skipped after the decisive
  native rejection.
- After rejection, the factored helper and decision-only private return are
  removed and accepted `proofcontrol.rs` is restored byte-for-byte.

## Decision

Reject both variants. Removing the immediately discarded Rust-owned pack is
proof-exact and improves corrected default-feature instructions by 0.043460%,
but matched native wall and CPU timing regress across the full sample and
stable tails. Keep Experiment 270 as the accepted baseline at 8,992,812,925
instructions, or 1.711495 times C.

Forward-subsumption packing should retain the accepted code shape until clause
ownership can represent C's stable alias directly; helper factoring and forced
out-of-lining both lose at whole-program scope.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=.artifacts/experiments/2026-07-24-009-defer-forward-subsumption-pack/rust-callgrind-defer-forward-subsumption-pack-default-corrected.out \
  target-wsl-282-corrected-defer-forward-subsumption-pack/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```

```powershell
& .\experiments\2026-07-22-214-move-termtree-insert-links\run-native.ps1 `
  -ParentExe .\target\native-270-borrow-active-pdt-frame\release\eprover.exe `
  -CandidateExe .\target\native-282-default-defer-forward-subsumption-pack\release\eprover.exe `
  -Pairs 64 `
  -OutputCsv .\experiments\2026-07-24-009-defer-forward-subsumption-pack\native-lusk-default.csv
```

The older `native-lusk-corrected.csv` is superseded by Experiment 285's
native feature audit.
