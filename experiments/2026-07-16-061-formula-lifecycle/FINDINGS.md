# Formula-wrapper lifecycle and derivation allocation

## Question

Can Rust close the remaining `WFormulaFree`/`FormulaSetDeleteEntry` ownership
gap and use C's formula-specific `PStackVarAlloc(3)` derivation allocation
without repeating the proof-order regression caused by shrinking clause and
formula derivation stacks together?

## Source audit

C `WFormulaFree` asserts that the wrapper has a formula and is detached from
its intrusive set links. It frees `ClauseInfo`, the optional derivation stack,
and the wrapper cell, but leaves the term-formula payload to term-bank garbage
collection. `FormulaSetDeleteEntry` first detaches the wrapper and then calls
`WFormulaFree`; extraction only detaches and returns the still-live wrapper.
`FormulaSetFreeFormulas` repeatedly deletes the first member.

Rust represents the same boundary through ownership rather than nullable raw
links. `extract_entry` transfers a complete `WrappedFormula` value to the
caller, while `delete_entry`, `clear`, and `FormulaSet` drop release the boxed
`ClauseInfo`, derivation vector, and wrapper storage through RAII. Shared terms
remain in `TermBank` until the explicit mark/sweep pass. Stable `entry_id`
values and `FormulaDerivationRef { ident, source }` replace `WFormula_p`
identity in derivation stacks and proof-state lookup.

## Retained change

C `WFormulaPushDerivation` allocates a three-slot stack on the first push. Rust
previously gave formula and clause derivation stacks the same six-entry physical
capacity chosen from C's aggregate clause-memory estimate. Formula stacks now
use an exact three-entry logical and physical capacity and double to six on the
fourth entry. Clause derivation allocation is unchanged.

The distinction matters because Rust's typed `DerivationEntry` is 32 bytes,
not C's pointer-sized union. The retained change therefore matches C's element
count and growth point, not its byte layout or allocator implementation.

## Setup

The 20,000-owner repeated-symbol corpus and baseline scaling script are reused
from `experiments/2026-07-15-009-formula-owner-memory-scaling/`. The immediate
baseline differs from the candidate only in
`WrappedFormula::ensure_derivation`: six physical entries versus three. Both
Linux binaries were built from the current source with locked release
dependencies.

```powershell
wsl.exe -d Ubuntu-24.04 -- bash `
  experiments/2026-07-16-061-formula-lifecycle/benchmark-paired.sh `
  /home/rober/.cache/e-rust-port/bin/17026b1bfe61aaf223cfaae54947c8d2679c31a0/fol/eprover `
  /home/rober/.cache/e-rust-port/candidates/formula-three-slot/eprover `
  .artifacts/experiments/2026-07-15-009-formula-owner-memory-scaling/corpus/repeated-20000.p `
  .artifacts/experiments/2026-07-16-061-formula-lifecycle/paired-20000.csv

.\e-interop.ps1 compare `
  -Corpus .\eprover\EXAMPLE_PROBLEMS\LFHOL `
  -RustExe .\target\release\eprover.exe `
  -TimeoutSeconds 20

.\e-interop.ps1 compare `
  -RustExe .\target\release\eprover.exe `
  -TimeoutSeconds 60
```

Valgrind Massif used `--time-unit=B` on the same repeated-owner problem for the
immediate six-slot baseline and three-slot candidate. Raw CSV and Massif output
are under `.artifacts/experiments/2026-07-16-061-formula-lifecycle/` and are
intentionally ignored by Git.

## Results

Five-run medians on the 20,000-owner CNF workload:

| Implementation | Wall | Peak RSS |
| --- | ---: | ---: |
| C reference | 0.48 s | 34,864 KiB |
| Rust six-slot baseline | 0.23 s | 62,028 KiB |
| Rust three-slot candidate | 0.23 s | 57,964 KiB |

The candidate removes 4,064 KiB (6.55%) from process peak RSS without a median
wall-time change. Its peak RSS is 1.663x C's on this focused workload, down from
1.779x for the immediate Rust baseline.

Massif attributes the reduction exactly to the two live formula derivation
owner groups at peak. Each group falls from 3,840,192 bytes to approximately
1,920,096 bytes. Peak `mem_heap_B` falls from 53,186,437 to 49,346,345 bytes, a
3,840,092-byte reduction (7.22%).

The new lifecycle regression establishes the ownership boundary directly:
extraction preserves source metadata and derivation storage, an extracted
wrapper keeps its term alive when marked, dropping it makes that term
collectable, and deletion consumes the wrapper while leaving its term for the
next bank sweep. A second regression pins the three-to-six formula-stack growth
point.

## Falsification checks

The earlier global three-entry experiment changed `lists.p` quantified-variable
order and was reverted. This candidate changes only formula stacks; clause
derivation allocation and proof-search stack size remain untouched. The focused
five-case LFHOL report at
`.artifacts/e-compare/20260716-210524-511955/comparison.json` keeps `lists.p`,
both permutation cases, and `SEV286^5.p` exact. The sole mismatch is the
established `sledgehammer.p` normalized proof-text difference.

The full 50-case report at
`.artifacts/e-compare/20260716-211527-699999/comparison.json` also keeps
`lists.p` exact. Its five mismatches are established search/resource surfaces:
`BOO020-1.p`, `GEO288+1.p`, `HEN011-2.p`, `sledgehammer.p`, and the synthetic
one-second `cpu-limit-LUSK6.lop` case. No formula-owner output mismatch was
added.

The lifecycle test does not rely on allocator counters or external reference
counts. It observes the term-bank membership boundary that C exposes through
its explicit wrapper free and later GC phases.

## Compatibility decision

Rust will not reproduce C's free-list allocator, intrusive link fields, or raw
pointer invalidation. RAII destruction plus typed set ownership implements the
same live-object and term-GC side effects without unsafe code. Stable source
keys preserve formula-parent identity across deque movement, archive copies,
and proof-state lookup.

The formula-specific three-entry stack is retained because it restores C's
logical allocation/growth contract, materially reduces owner memory, and does
not perturb the allocator-sensitive proof case that rejected the broader
change. Exact byte parity is neither possible nor desirable while Rust entries
carry typed, relocation-safe parent identities.

## Validation

The standard five-run benchmark is
`.artifacts/e-compare/20260716-213127-387845-benchmark/benchmark.json`. Its
aggregate Rust/C wall ratio is 3.056x across nine behavior-matching cases; the
established `BOO020-1.p` outcome difference is excluded. Sustained search stays
within the preceding memory envelope: `LUSK6.lop` measures 2.601x and 241,544
KiB, while `LUSK6ext.lop` measures 2.407x and 467,892 KiB.

The final repository gates passed `cargo fmt --all -- --check`, all-target and
all-feature `cargo check`, pedantic Clippy with warnings denied, the complete
all-target and all-feature test suite (including 4,182 library tests), and a
locked release build of every binary. The 32 Python interoperability tests and
all four C-source documentation checks also pass. The focused lifecycle,
stack-growth, LFHOL, full compatibility, paired RSS, Massif, and standard
benchmark checks above passed their intended assertions; compatibility reports
retain only the documented existing mismatches.
