# Formula-set front-drain storage

## Question

Does replacing Rust's shifting `Vec` front extraction with append-order `VecDeque` storage recover C `FormulaSetExtractFirst`'s constant-time behavior without changing formula order, ownership, or proof outcomes?

## Setup

All commands were run from the repository root on 2026-07-15 (America/New_York). `generate-corpus.ps1` creates 20,000 append-ordered FOF axiom wrappers followed by an identical conjecture, so both provers must parse, archive, and clausify a large formula set before returning an immediate theorem.

```powershell
.\experiments\2026-07-15-007-formula-set-front-drain\generate-corpus.ps1 -Count 20000

wsl.exe -d Ubuntu-24.04 -- bash `
  /mnt/c/Users/rober/Code/E_Rust_Port/experiments/2026-07-15-007-formula-set-front-drain/benchmark.sh `
  baseline `
  /home/rober/.cache/e-rust-port/rust-target/17026b1bfe61aaf223cfaae54947c8d2679c31a0/release/eprover `
  /mnt/c/Users/rober/Code/E_Rust_Port/.artifacts/experiments/2026-07-15-007-formula-set-front-drain/corpus/formula-drain.p `
  /mnt/c/Users/rober/Code/E_Rust_Port/.artifacts/experiments/2026-07-15-007-formula-set-front-drain/raw

.\e-interop.ps1 benchmark `
  -Corpus .\.artifacts\experiments\2026-07-15-007-formula-set-front-drain\corpus `
  -Runs 5 `
  -TimeoutSeconds 60 `
  -RegressionThreshold 10

# Repeat benchmark.sh with label "deque" after the WSL release build.
.\.venv\Scripts\python.exe `
  experiments\2026-07-15-007-formula-set-front-drain\analyze.py `
  baseline=.artifacts\experiments\2026-07-15-007-formula-set-front-drain\raw\baseline.csv `
  deque=.artifacts\experiments\2026-07-15-007-formula-set-front-drain\raw\deque.csv
```

The cached baseline binary SHA-256 was `805c4911cfa47738514635e1d0c323bc0641f7702d01ee74ad7b84152187a56a`. The deque build SHA-256 was `6c96deb2a5b388706747645fa6be16d456a4695bf03b82a12ccc830bbf94c1d9`.

## Results

Rust `FormulaSet` used `Vec::remove(0)` for C `FormulaSetExtractFirst`. C unlinks `anchor->succ` in constant time; Rust therefore copied every surviving `WrappedFormula` on each archive/CNF drain step and made the full drain quadratic. The owner is now backed by `VecDeque`: insertion remains append ordered, front extraction is `pop_front`, set append and stable entry IDs are preserved, and the one slice-based definition rewrite calls `make_contiguous` without changing logical order.

Five-run focused medians:

| Build | Wall | CPU | Peak/median RSS |
| --- | ---: | ---: | ---: |
| Pre-change Rust | 3.240 s | 3.220 s | 140,120 KiB |
| Deque Rust | 1.760 s | 1.910 s | 140,276 KiB |

The deque/pre-change ratio is `0.543x` wall and `0.593x` CPU: a 45.7% wall-time and 40.7% CPU reduction. All ten direct runs returned `Theorem`.

The C/Rust harness report is `.artifacts/e-compare/20260715-214709-792876-benchmark/benchmark.json`. It has no behavior mismatch. C's median is 0.0738 seconds wall and 0.0700 seconds CPU; deque Rust is 1.7916 seconds wall and 1.9100 seconds CPU, a remaining 24.274x wall ratio on this deliberately parser/owner-heavy synthetic workload.

The source/GC audit found that the rest of this ownership slice is already represented safely:

- C `WFormulaFree` frees wrapper metadata/derivation storage but deliberately leaves `tformula` to term-bank GC. Rust drops the wrapper-owned `ClauseInfo`/derivation through RAII while shared terms remain in the bank until its explicit mark/sweep.
- C `FormulaSetFreeFormulas` and `FormulaSetFree` delete wrappers, the sentinel, identifier, and set cell. Rust `clear`/owner drop perform the corresponding RAII destruction; no standalone sentinel is allocated.
- C proof state registers twelve clause-set roots and four formula-set roots in pointer trees. Rust registers the same owner domains as typed `GcSetHandle`s, resolves them to state-owned sets during collection, and deregisters the optional watchlist when it is discarded.
- Formula-set CNF/simplification helpers pass actual `ClauseSet`/`FormulaSet` roots through `ClauseSetMarker`/`FormulaSetMarker`; they no longer require untyped raw-pointer marker slices.

## Falsification checks

The focused corpus repeats one formula term, so signature growth and theorem search do not explain the before/after delta; the changed cost is wrapper storage and drain behavior. The baseline and deque binaries used the same corpus, arguments, WSL toolchain, and five-run script. Every run returned the same theorem status.

Changing storage could have perturbed append order or slice-based higher-order definition rewriting. Existing insert/extract/move tests pass, a new 4,096-entry drain regression checks every stable entry ID in append order, and `make_contiguous` is used only at the existing whole-set rewrite boundary.

The large remaining Rust/C ratio falsifies any claim that this change completes formula-owner allocation/performance parity. It removes the quadratic component but leaves substantial parser/preprocessing time and a roughly 100 MiB resident-memory difference for later profiling.

## Conclusion and limits

Deque storage restores C's asymptotic front-extraction contract and materially improves large formula-set CNF/archive workloads without changing outcomes or ordering. The broader Formula Sets free/GC mapping uses RAII and typed stable roots rather than reproducing C allocator/pointer internals.

This experiment does not close `E_Rust_Port-j76.1.8`: the remaining 24.274x focused time ratio and memory gap require a separate allocation/parser profile before exact performance parity can be claimed.

## Validation

Focused tests passed:

```powershell
cargo test --lib --all-features formula_set_front_drain_preserves_append_order_at_scale
cargo test --lib --all-features formula_set_insert_extract_and_move_preserve_c_list_order
cargo test --lib --all-features proof_state_collect_term_garbage_marks_registered_formula_roots
cargo test --lib --all-features proof_state_alloc_registers_represented_gc_roots
```

Full repository gates also passed:

```powershell
cargo fmt --all -- --check
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings -D clippy::pedantic
cargo test --all-targets --all-features
cargo build --locked --release --bin eprover
```

The full test run passed 4,084 library tests and all three schedule integration tests. The standard 50-case C/Rust comparison report is `.artifacts/e-compare/20260715-215442-948174/comparison.json`: its five mismatches are a strict subset of the prior six-case baseline because `GEO288+1.p` matched in this run. The remaining established cases are `BOO020-1.p`, `LUSK6ext.lop`, `HEN011-2.p`, `sledgehammer.p`, and `synthetic/cpu-limit-LUSK6.lop`.

The standard five-run benchmark report is `.artifacts/e-compare/20260715-215522-681917-benchmark/benchmark.json`. Its aggregate Rust/C wall ratio is `3.371x`; the already-known `BOO020-1.p` resource-limit outcome difference is the single behavior mismatch and is excluded from that ratio.
