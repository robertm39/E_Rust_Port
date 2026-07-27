# Experiment 278: Changed-only recursive normal-form result

## Status

Rejected for Bead `E_Rust_Port-j76.5.3`.

## Question

Can recursive leftmost-innermost normalization return `None` when a borrowed
child remains identical, avoiding an owned `Rc<TermCell>` clone and drop on
the normal-form-date fast path while preserving the public owned result?

## Setup

- Parent source: commit `0c2aaee9` (`perf: reject structural variable
  identity guard`); executable source remains accepted Experiment 270.
- Parent WSL Callgrind profile:
  `.artifacts/experiments/2026-07-23-032-borrow-active-pdt-frame/rust-callgrind-borrow-active-pdt-frame.out`.
- Representative accepted line profile:
  `.artifacts/experiments/2026-07-23-033-pdt-cursor-after-active-frame/rust-callgrind-pdt-cursor-after-active-frame.out`.
- Parent native executable:
  `target/native-270-borrow-active-pdt-frame/release/eprover.exe`.
- Candidate: add a private changed-only wrapper at the recursive child edge.
  It recognizes unchanged no-top-link date and free-variable fast paths before
  the existing worker acquires an owned handle. Slow and changed paths call
  the unchanged normalizer and preserve its exact result.
- Workload: upstream `LUSK6.lop` with `--auto --silent --cpu-limit=600
  --memory-limit=2048 --detsort-rw --detsort-new`.

The accepted line profile records 651,498 recursive child-normalizer calls.
The public normalizer and top-level clause/equation callers retain their owned
return contract; only recursive subterm reconstruction observes the
changed-only result.

## Results

### Deterministic profile

The candidate exits zero and preserves the expected unsatisfiable status.
Exact LUSK6 Callgrind instructions fall from 8,992,812,925 to 8,882,054,008,
a reduction of 110,758,917 or 1.231638%. The hypothetical Rust/C ratio
improves from 1.711495 to 1.690416.

The recursive child edge makes 651,498 calls to the accepted owned worker.
With the changed-only wrapper, only 235,658 of those calls enter the slow
worker, avoiding about 415,840 owned worker calls on confirmed normal-form
fast paths. The top rewrite-chain clone entry falls from 3,322,407 to
1,635,394 calls.

### Native timing

After four alternating warmup pairs, 64 alternating measured pairs confirm
that the deterministic reduction transfers to native Windows:

- wall mean and median improve 0.762040% and 0.440434%;
- CPU mean and median improve 1.137576% and 1.149425%;
- mean paired wall and CPU improvements are 0.647880% and 1.065628%;
- the candidate wins 39 wall and 36 CPU pairs, with ten CPU ties.

The final 32 pairs improve 0.805204% wall and 1.608867% CPU by aggregate
means. The final 16 remain favorable at 0.312706% wall and 1.082251% CPU.
All 128 measured processes and eight warmup processes exit zero. The parent
and candidate executables are both 8,952,320 bytes.

Raw measured results are retained in `native-lusk.csv`; the discarded warmup
CSV remains under `.artifacts/experiments/`.

### Compatibility

A direct status-only parent/candidate run is byte-identical, but enabling the
maintained `--proof-object=1` surface exposes a deterministic proof-order
difference. Three direct parent/candidate pairs all exit zero and reproduce
different proof objects.

Maintained report `.artifacts/e-compare/20260724-031513-905360` completes all
50 cases with one unexpected mismatch plus the declared `sledgehammer`
difference. The unexpected `LUSK6.lop` row has matching unsatisfiable status
and exit zero, but its normalized proof differs from C. The first difference
reorders initial formulas 5 through 7 and leads to a distinct 111-step proof
instead of C's 113-step proof. The accepted parent remains exact for this
case.

The skipped recursive worker has no hidden normal-form-date or trace side
effect on either fast path. The remaining behavioral difference is the
shorter owned `Rc<TermCell>` handle lifecycle, which perturbs later
pointer-sensitive ordering even though the normalized term identity is
unchanged.

## Validation

- All 33 candidate rewrite tests pass in default and all-feature modes.
- A focused regression confirms that the current normal-form-date fast path
  returns the changed-only sentinel.
- Strict all-feature library pedantic Clippy passes.
- Exact WSL Callgrind and all native timing processes exit successfully.
- The maintained 50-case matrix preserves every status and resource outcome,
  but rejects the unexpected LUSK6 proof-object difference.
- After rejection, the wrapper and its regression are removed and accepted
  `rewrite.rs` is restored byte-for-byte.

## Decision

Reject despite the real 1.231638% deterministic and 0.762040%/1.137576%
native wall/CPU improvements. The project requires a drop-in replacement and
the maintained matrix requires zero unexpected differences; changing the
proof object is therefore disqualifying. Keep Experiment 270 as the accepted
executable baseline at 8,992,812,925 instructions, or 1.711495 times C.

This experiment also demonstrates that an apparently read-only `Rc<TermCell>`
clone/drop can be observable through later pointer-sensitive ordering. Future
normal-form ownership work must preserve the owned-handle lifecycle or prove
the full proof-object surface exact before performance acceptance.

## Reproduction

```powershell
& .\experiments\2026-07-22-214-move-termtree-insert-links\run-native.ps1 `
  -ParentExe .\target\native-270-borrow-active-pdt-frame\release\eprover.exe `
  -CandidateExe .\target\native-278-normalform-changed-only-recursion\release\eprover.exe `
  -Pairs 64 `
  -OutputCsv .\experiments\2026-07-24-005-normalform-changed-only-recursion\native-lusk.csv
```

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=.artifacts/experiments/2026-07-24-005-normalform-changed-only-recursion/rust-callgrind-normalform-changed-only-recursion.out \
  target-wsl-278-normalform-changed-only-recursion/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```
