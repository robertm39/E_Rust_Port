# Formula-owner boundaries and lambda-lift indexing

## Question

Which remaining Formula Sets pending items are observable missing behavior, and does Rust preserve C's `PDTree` lookup order and performance for exact/generalized lambda-lift reuse?

## Setup

All commands were run from the repository root on 2026-07-15 (America/New_York). The C reference was upstream commit `17026b1bfe61aaf223cfaae54947c8d2679c31a0` in the existing WSL reference cache.

```powershell
.\e-interop.ps1 compare `
  -Corpus .\experiments\2026-07-15-005-formula-owner-boundaries `
  -RustExe .\target\release\eprover.exe `
  -TimeoutSeconds 30

cargo build --locked --release --bins
.\e-interop.ps1 compare-tools `
  -RustBinDir .\target\release `
  -Tool classify_problem,eground,enormalizer,epatternize `
  -TimeoutSeconds 30

.\experiments\2026-07-15-005-formula-owner-boundaries\generate-lambda-lift-corpus.ps1 `
  -Count 1000
.\e-interop.ps1 benchmark `
  -Corpus .\experiments\2026-07-15-005-formula-owner-boundaries\lambda-lift-corpus `
  -Runs 3 `
  -TimeoutSeconds 60 `
  -RegressionThreshold 10
```

The benchmark command was run immediately before and after replacing the Rust lambda-lift exact-map/linear-vector lookup with the existing bank-normalizing `PdTree`.

## Results

### Parser boundaries

Artifact `.artifacts/e-compare/20260715-203233-989101/comparison.json` contains three exact comparisons and zero mismatches. C and Rust both return syntax error 3 for:

- `embedded-tcf-distinct.p`: embedded `$distinct` remains inside C's typed-clause parser because `WFormulaTSTPParse` special-cases only a direct top-level `$distinct` before calling `TcfTSTPParse`.
- `tcf-non-clause.p`: `TcfTSTPParse` accepts typed clause bodies (optionally under leading universal quantifiers), not arbitrary conjunction formula bodies.
- `top-level-thf-lambda.p`: the lambda root has arrow type rather than Boolean type, so it is not a valid THF formula root even though lambda-valued arguments and lambda equalities are supported.

These are C-compatible grammar/type boundaries, not features to add to the Rust executable.

### Helper comparison boundary

Artifact `.artifacts/e-compare/20260715-203258-985096-tools/tool-comparison.json` contains 12 helper cases. Eleven are exact. The sole mismatch is `classify_problem --parse-features` on the legacy 22-field `SpecFeature` input.

C's `SpecFeaturesParse` writes only the 22 legacy numeric fields and a few invariant class fields into an uninitialized enlarged `SpecFeatureCell`. `SpecFeaturesPrint` and `SpecFeaturesAddEval` then read six newer fields that the parser never wrote. In this run C printed `22319, 32767, 0.000000, 0.000000, true, false`; Rust deterministically printed zero/false defaults. The defined 22-field prefix is byte-identical. Reproducing stack-dependent C values would require undefined behavior and is intentionally rejected as a compatibility target.

### Lambda-lift PDTree

The production change indexes every stored closed body in `PdTree`, uses `prefer_general=false` like C, and resolves the indexed occurrence back to its lifted template and definition. The temporary exact-term map and linear generalized-candidate vector scan are no longer lookup paths.

Regression coverage confirms:

- exact and ordinary generalized reuse still work for both formula-set and post-CNF clause lifting;
- when a specific definition and a more general definition both match, variable-first PDTree traversal chooses the general definition like C;
- a lookup over 1,024 unrelated indexed definitions visits at most four PDTree nodes instead of testing 1,024 candidate terms.

Focused benchmark artifacts:

| Run | Artifact | Rust median | C median | Rust/C | Outcome match |
| --- | --- | ---: | ---: | ---: | --- |
| Before | `.artifacts/e-compare/20260715-204301-290694-benchmark/benchmark.json` | 0.029739 s | 0.008817 s | 3.373x | yes |
| After | `.artifacts/e-compare/20260715-204741-205475-benchmark/benchmark.json` | 0.028691 s | 0.010022 s | 2.863x | yes |

The three-run wall-clock sample is small and noisy; the structural visited-node assertion is the durable complexity regression. The focused wall time nevertheless improved, and neither benchmark had an outcome mismatch.

## Falsification checks

- The first generated THF corpus used integer numerals as body discriminators. C correctly rejected them in THF because the numeral symbols were not declared in that context. The generator now emits explicit typed constants, and the corrected small workload gives `Theorem` with exit 0 in both implementations.
- A 10-lambda full-output comparison had matching exit/status but differing normalized proof output, an already established higher-order proof-order surface. Benchmark outcome comparison therefore checks the intended preprocessing path without claiming proof-byte parity for this synthetic workload.
- Existing exact/generalized reuse tests were rerun after removing the exact map; all four passed.

## Conclusion and limits

Embedded TCF `$distinct`, non-clausal TCF bodies, and non-Boolean top-level THF lambdas are resolved as evidence-backed C boundaries. The legacy `classify_problem --parse-features` suffix is an upstream uninitialized-memory boundary and remains deterministic in safe Rust. The substantive lambda-lift indexing gap is resolved: Rust now uses the ported PDTree for C-shaped lookup order and prefix-pruned performance.

This experiment does not claim that every remaining FOF/TFF FOOL spelling, all higher-order THF syntax, helper output, or stable formula-handle surface in the broad Formula Sets Bead is complete.

## Final validation

The final release binary repeated the three focused parser comparisons with zero mismatches in `.artifacts/e-compare/20260715-210643-875363/comparison.json`.

Local quality gates:

```powershell
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings -D clippy::pedantic
cargo test --all-targets --all-features
cargo build --locked --release --bin eprover
.\.venv\Scripts\python.exe tools\c_source_docs\check_markdown_links.py
```

All passed. The full test run reported 4,083 passing library tests plus all binary and schedule targets; the documentation checker validated local links in 269 Markdown files.

The standard 50-case comparison is `.artifacts/e-compare/20260715-205419-618213/comparison.json`. Its six established mismatch names are unchanged; `SWV851-1.p` added a transient seventh result because both first-order implementations reached the 60-second resource boundary and the Windows candidate was externally terminated before emitting its status. The changed lambda-lift path is higher-order data with no terms to process in that first-order case.

The required five-run standard benchmark is `.artifacts/e-compare/20260715-210733-739733-benchmark/benchmark.json`: aggregate Rust/C wall time was 3.095x versus the preceding 3.137x baseline in `.artifacts/e-compare/20260715-200938-546890-benchmark/benchmark.json`. Nine comparable cases matched outcomes; the established resource-boundary `BOO020-1.p` case was excluded from the aggregate.
