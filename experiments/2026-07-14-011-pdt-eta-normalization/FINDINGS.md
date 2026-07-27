# PDTree Eta-Normalized Index Paths

## Question

Can Rust reproduce C's `PDTreeInsertTerm`, `PDTreeDelete`, and
`PDTreeSearchInit` rule of eta-expanding non-FO patterns and eta-reducing other
terms without imposing a material cost on ordinary first-order demodulation?

## Setup

- Baseline commit: `6b70ba92` (`Cache PDTree variable edge metadata`).
- Detached baseline worktree:
  `.artifacts/baseline-eta-6b70ba92/`.
- Baseline SHA-256:
  `f36149bca32414b059313ebcd47bb2e5bd3bbefe9489a80d61912f15ef241d46`.
- Candidate SHA-256:
  `281fb168ab3cb5a62c573e5c1be30551c0c06f76aa779180d75a4ebc483f5760`.
- Primary problem:
  `eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop`.
- Common options:
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw --detsort-new`.

Exact setup and execution from the repository root:

```bash
git worktree add --detach .artifacts/baseline-eta-6b70ba92 6b70ba92
cd .artifacts/baseline-eta-6b70ba92
ln -s ../../eprover eprover
cargo build --locked --release --bin eprover
cd ../..
cargo build --locked --release --bin eprover
bash experiments/2026-07-14-011-pdt-eta-normalization/benchmark.sh
bash experiments/2026-07-14-011-pdt-eta-normalization/callgrind.sh
```

Executable-level validation from PowerShell:

```powershell
cargo build --locked --release --bin eprover
.\e-interop.ps1 compare -RustExe .\target\release\eprover.exe
.\e-interop.ps1 benchmark -Runs 3
```

## Implementation

Bank-aware PDTree insertion and search classify a term exactly as C does:
non-FO patterns use eta expansion, while other lambda-bearing terms use eta
reduction. Ordinary first-order terms use a property-bit fast path and enter
the pre-existing index/search implementation directly.

Clause-set insertion normalizes both sides before mutating an unoriented
demodulator index, avoiding a one-sided entry if the second normalization
fails. Proof-state processed sets, watchlist rebuild/reinsertion, rewriting,
and bank-aware unit simplification use the bank-aware entry points.

When normalization changes an indexed key, Rust retains its prefix code and
weight by compact occurrence key. Clause extraction can therefore delete the
normalized entry while holding only the original clause term, without a
second mutable borrow of the term bank. Unchanged first-order occurrences do
not allocate this auxiliary path.

## Results

The initial implementation normalized every first-order search and eagerly
built a second prefix code for every insertion. Seven alternating LUSK6 pairs
measured baseline/candidate medians of 3.01/3.18 wall seconds and 2.81/2.95
user seconds, regressions of about 5.6% and 5.0%. That shape was rejected. Its
raw log is retained at
`.artifacts/experiments/2026-07-14-011-pdt-eta-normalization/alternating-times-pre-fast-path.txt`.

With the property fast path and lazy changed-path construction, seven pairs
measure baseline/candidate medians of 3.01/2.93 wall seconds and 2.93/2.78 user
seconds. Individual wall runs are noisy, so this timing result only falsifies
the earlier material regression; it is not evidence of a speedup.

Matched Callgrind runs execute `19,888,785,947` baseline instructions and
`19,966,434,663` candidate instructions. The required first-order
classification checks add `77,648,716` instructions, or 0.39%, below timing
noise but not zero. Raw timings and profiles are retained under
`.artifacts/experiments/2026-07-14-011-pdt-eta-normalization/`.

The full 50-case differential report is retained at
`.artifacts/e-compare/20260714-233340-593620/`. Five established or
load-sensitive mismatches remain: BOO020 and SWV851 resource/exit behavior,
GEO288's current Windows CPU-limit outcome, `sledgehammer.p` normalized proof
text, and the synthetic one-second CPU-limit case. All other configured
higher-order cases match, including `tffex01`, `lists`, both `permute_func`
cases, `SEV286^5`, and `LUSK6ext`.

The three-run native benchmark is retained at
`.artifacts/e-compare/20260714-234700-666675-benchmark/`. Its aggregate Rust/C
wall-time ratio is 3.136x. LUSK6 measures 2.702 seconds for Rust, a 2.521x
ratio; LUSK6ext measures 6.475 seconds, a 2.701x ratio. Project-wide
performance parity remains incomplete.

## Falsification Checks

- Focused tests cover unchanged first-order insertion, eta reduction, non-FO
  pattern eta expansion, normalized search, and deletion by the original term.
- All 13 bank-aware filtered tests pass.
- The full all-target, all-feature suite passes 4,062 library tests and 3
  schedule integration tests.
- Strict all-target, all-feature Clippy passes with warnings and pedantic lints
  denied.
- The full 50-case C/Rust differential corpus completes with no new
  higher-order mismatch.
- Both experiment scripts pass `bash -n` and are ASCII executables.
- C-source coverage, manual-section preservation, Change Later wording, and
  Markdown-link checks pass.

## Conclusion And Limits

The bank-aware eta-indexing path is accepted because it matches C's branch
order, supports both normalization directions, deletes changed keys reliably,
and limits ordinary first-order overhead to 0.39% deterministic instructions.

The retained-path map is Rust-specific auxiliary storage not included in the
C-shaped `PDTreeStorage` estimate. Its `(clause identifier, side)` key assumes
the same globally unique identifiers as compact candidate lookup. Full live
`ClausePos` ownership should replace that key with a unique occurrence handle
and account for path storage. Raw standalone PDTree APIs remain unnormalized;
callers that can carry higher-order terms must use the bank-aware API until an
explicit bank contract is available everywhere.

C repeats normalization at insertion, deletion, and search initialization.
A later C cleanup could centralize classification and return a normalized
index handle from insertion for deletion, but only after term-bank side
effects, memory cost, and proof-search order are covered by compatibility and
performance tests.
