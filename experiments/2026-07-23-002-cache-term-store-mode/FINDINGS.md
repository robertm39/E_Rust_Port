# Experiment 240: Cache term-store problem mode

## Question

Can the sole production owner of term trees cache its first initialized
problem type and pass it to bucket operations, avoiding one thread-local mode
read on every nonempty insertion while preserving uninitialized construction
and higher-order comparator keys?

## Baseline

- Accepted source: Experiment 231.
- Exact LUSK6 Callgrind: 9,923,564,772 instructions.
- Original C Callgrind: 5,254,361,329 instructions.
- Rust/C ratio: 1.888634.
- `TermTree::insert`: 658,858,502 exclusive instructions.
- Experiment 239 attributes 12,234,290 instructions to the
  `problem_type()` body and thread-local access across 2,446,858 nonempty
  insertion calls.

## Candidate

Add one `Cell<ProblemType>` to `TermCellStore`, initialized to
`NotInitialized`. Before each bucket find, insert, extract, or delete:

1. use the cached mode when it is initialized;
2. otherwise read the thread-local global mode; and
3. cache that result only when it is no longer `NotInitialized`.

Private explicit-mode `TermTree` operations receive the selected mode. The
existing public operations remain global-mode wrappers so standalone callers
and tests retain their prior contract. An empty-tree public insertion avoids
reading the global because it performs no comparison.

The focused regression test constructs the store before global mode
initialization, proves that the sentinel is not cached, switches to
`HigherOrder`, inserts equal-symbol terms with distinct type identities, and
proves that they remain distinct nodes under the cached higher-order key. It
also proves that `exit()` resets the cache.

## Validation

- `cargo fmt --all -- --check`
- `cargo clippy --lib --all-features -- -D warnings -D clippy::pedantic`
- Four `terms::termtrees::tests` passed.
- Seven `terms::termcellstore::tests` passed, including the new cache
  transition and higher-order identity test.
- The Callgrind candidate found the exact LUSK6 proof, reported
  `Unsatisfiable`, and exited zero.
- Direct native parent/candidate runs had byte-exact stdout and stderr,
  including `% Proof found!` and `% SZS status Unsatisfiable`.
- All 128 measured native runs exited zero.

## Measurement

The candidate retires 9,909,052,509 instructions, down 14,512,263 or
0.146240% from the accepted 9,923,564,772-instruction baseline. The implied
C/Rust ratio improves from 1.888634 to 1.885872.

Four alternating warmup pairs preceded 64 alternating measured native pairs.
Positive percentages mean the candidate is faster; every measured aggregate
is negative:

| Native metric | All 64 pairs | Last 32 pairs |
| --- | ---: | ---: |
| Wall mean | -2.310137% | -2.538463% |
| CPU mean | -2.073093% | -2.334371% |
| Wall median | -2.321100% | -1.329853% |
| CPU median | -1.712329% | -0.362319% |
| Mean paired wall change | -1.917382% | -1.936872% |
| Mean paired CPU change | -1.737069% | -1.827383% |
| Median paired wall change | -1.986011% | -1.051809% |
| Median paired CPU change | -1.428863% | -1.428863% |
| Candidate wall wins | 25 of 64 | 15 of 32 |
| Candidate CPU wins | 24 of 64, 2 ties | 14 of 32 |

The candidate binary shrinks 1,536 bytes, from 8,654,336 to 8,652,800 bytes.
Neither its deterministic instruction reduction nor its size reduction
translates to native throughput.

## Result

Reject. The cache preserves the tested comparator semantics and removes real
deterministic work, but the native regression is roughly 2% and persists in
the stable half. Compatibility matrices and full repository gates were
skipped after the candidate failed the native performance requirement.

All candidate production code and tests were removed. The accepted source is
restored byte-for-byte to Experiment 231 at 9,923,564,772 instructions and
1.888634 times C.

Raw evidence:

- Callgrind:
  `.artifacts/experiments/2026-07-23-002-cache-term-store-mode/rust-callgrind-cache-term-store-mode.out`
- Warmups:
  `.artifacts/experiments/2026-07-23-002-cache-term-store-mode/native-warmup.csv`
- Measured pairs:
  `experiments/2026-07-23-002-cache-term-store-mode/native-lusk.csv`

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-cache-term-store-mode.out \
  target-wsl-240-cache-term-store-mode/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```

```powershell
& .\experiments\2026-07-22-214-move-termtree-insert-links\run-native.ps1 `
  -ParentExe .\target\native-231-specialize-pdt-cursor\release\eprover.exe `
  -CandidateExe .\target\native-240-cache-term-store-mode\release\eprover.exe `
  -Pairs 64 `
  -OutputCsv .\experiments\2026-07-23-002-cache-term-store-mode\native-lusk.csv
```
