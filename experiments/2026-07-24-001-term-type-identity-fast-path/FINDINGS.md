# Experiment 274: Term-type identity fast path

## Status

Rejected for Bead `E_Rust_Port-j76.5.3`.

## Question

Can structural term comparison return immediately when both terms store the
same optional type handle, avoiding temporary `Rc<Type>` clones and the general
`TypesCmp` path while preserving its behavior for every unequal handle?

## Setup

- Parent source: commit `a03003f6` (`perf: reject recursive rewrite descriptor`);
  executable source remains accepted Experiment 270.
- Parent WSL Callgrind profile:
  `.artifacts/experiments/2026-07-23-032-borrow-active-pdt-frame/rust-callgrind-borrow-active-pdt-frame.out`.
- Parent native executable:
  `target/native-270-borrow-active-pdt-frame/release/eprover.exe`.
- Candidate: add a crate-private borrowed type-identity predicate, use it in
  the existing structural-equality helper, and return early from
  `compare_term_types` only when the two optional handles are identical.
- Workload: upstream `LUSK6.lop` with `--auto --silent --cpu-limit=600
  --memory-limit=2048 --detsort-rw --detsort-new`.

The parent executes `compare_term_types` 1,138,621 times and attributes
76,287,607 exclusive instructions to it. The candidate deliberately does not
use type UIDs: an invalid UID or a UID from a different type bank is not a
process-wide identity key. Unequal handles retain the exact owning
`TypesCmp` fallback, including its pointer-based arrow-argument ordering.

## Results

### Deterministic profile

The candidate proves the expected unsatisfiable result and falls from
8,992,812,925 to 8,952,070,277 instructions, a reduction of 40,742,648 or
0.453058%. The hypothetical Rust/C ratio improves from 1.711495 to 1.703741.

The gain is localized to the intended owner. The two visible
`compare_term_types` entries fall from 76,287,607 to 35,297,251 exclusive
instructions, saving 40,990,356 or 53.731343%. That explains 100.607982% of
the whole-program reduction; the small excess is ordinary code-layout
movement elsewhere. Call counts remain exactly 1,138,621.

### Native timing

Production timing decisively reverses the instrumented result. After four
alternating warmup pairs, 64 alternating measured pairs report:

- wall mean and median regress 2.083312% and 1.126949%;
- CPU mean and median regress 1.819820% and 1.162791%;
- mean and median paired wall changes regress 2.010130% and 1.335948%;
- mean and median paired CPU changes regress 1.861945% and 1.162791%;
- the candidate wins only 20 of 64 wall pairs and 17 CPU pairs, with nine CPU
  ties.

The stable tail repeats and strengthens the reversal. The last 32 pairs
regress 2.321625% wall and 2.040816% CPU by aggregate means, and the last 16
regress 2.624216% wall and 1.893664% CPU. All 128 measured processes exit zero.
The parent and candidate produce byte-identical direct output, including the
expected proof and SZS status.

The candidate executable is 8,936,448 bytes, 15,872 bytes smaller than the
8,952,320-byte parent. Neither the instruction reduction nor the smaller
binary predicts production throughput on this host.

## Validation

- All 19 candidate term-type tests and all 46 term-function tests pass in both
  default and all-feature configurations.
- The identity regression covers absent types, one shared type handle, and
  structurally equal but distinct type handles.
- The structural-comparator regression covers the shared identity fast path,
  the equal-result fallback for distinct simple-type handles, and ordering of
  genuinely different types.
- Strict all-feature library pedantic Clippy passes.
- Candidate and parent direct proof output is byte-identical.
- After rejection, candidate source and tests are removed and the accepted
  `termfunc.rs` and `termtypes.rs` are restored byte-for-byte.
- Compatibility matrices are skipped because the native production gate
  rejects this performance-only change.

## Decision

Reject. The borrowed identity fast path is semantically exact and removes more
than half of the target owner's instrumented work, but it causes a stable
roughly 2% production regression. Keep Experiment 270 as the accepted
executable baseline at 8,992,812,925 instructions, or 1.711495 times C.

## Reproduction

```powershell
& .\experiments\2026-07-22-214-move-termtree-insert-links\run-native.ps1 `
  -ParentExe .\target\native-270-borrow-active-pdt-frame\release\eprover.exe `
  -CandidateExe .\target\native-274-term-type-identity-fast-path\release\eprover.exe `
  -Pairs 64 `
  -OutputCsv .\experiments\2026-07-24-001-term-type-identity-fast-path\native-lusk.csv
```

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=.artifacts/experiments/2026-07-24-001-term-type-identity-fast-path/rust-callgrind-term-type-identity-fast-path.out \
  target-wsl-274-term-type-identity-fast-path/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```
