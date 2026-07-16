# Formula/parser record and nested-include parity

## Status

Completed for Bead `E_Rust_Port-j76.1.40`. The shared executable parser now
has focused coverage for every outer record family dispatched by C
`FormulaAndClauseSetParse`, and nested include selectors follow C's recursive
filter order. The vendored C source remained unchanged.

## C record dispatch inventory

`FormulaAndClauseSetParse` has two format branches:

- raw LOP repeatedly parses clause starts into formula-backed owners; and
- the non-LOP loop recognizes legacy `input_clause`/`input_formula`, modern
  `cnf`/`fof`/`tff`/`tcf`/`thf`, and `include` records.

The wrapper parsers add the format-specific details: TSTP names may be names,
positive integers, or single-quoted strings; TSTP records may carry source and
useful-info fields; `type` records mutate the signature while retaining a
`$true` formula owner; and `CPTypeWatchClause` entries are routed to the
watchlist rather than the normal formula set. Legacy TPTP record names and
roles retain their narrower grammar.

The concentrated Rust regression covers raw LOP, both legacy record kinds,
all five modern record kinds, typed declarations, numeric and quoted TSTP
names, optional source/useful-info fields, and legacy `input_clause`, TSTP
`cnf`, and TSTP `tcf` watchlists. First-order TSTP and THF are tested in
separate scanners because C rejects mixed first-order/higher-order wrapper
state. This resolves record-dispatch parity; it does not claim that every term
or formula body spelling is already accepted. Those grammar surfaces remain
owned by the dedicated term/formula parser tasks.

## Nested selector discrepancy and fix

C allocates temporary formula and watchlist sets for each include, recursively
parses the complete subtree, filters both completed sets by that include's
selector, checks that every requested name survived the nested filters, and
then inserts the survivors into the parent sets.

Rust previously applied the current selector only to records directly parsed
by that recursive call. A formula arriving through another include therefore
escaped an outer selector, while an outer selector requesting that nested
formula was incorrectly reported missing.

The shared TPTP/TSTP entry walkers now carry owned selector frames and test
each record from the innermost frame outward. This is the streaming equivalent
of C's completed-set filtering:

- records rejected by an inner selector never mark an outer selector found;
- an outer selector can retain a requested record reached through an
  unselected nested include;
- nested records not named by the outer selector cannot leak; and
- per-include missing-name diagnostics still use the included scanner's source
  and include position.

Wrapper dialect remains a parse-time side effect even when the selector later
discards every owner. Rust therefore records an encountered `thf` wrapper as
higher-order before applying selector acceptance, while retained FOOL owners
may still promote the returned CNF problem type through their represented
content. A regression pins an empty selector over THF declarations/formulas:
the owner sets are empty but the parsed dialect remains higher-order, matching
C's pre-filter `SetProblemType` timing.

The focused executable tests cover both TPTP and TSTP nested selection. The
permanent `eground/nested-selected-include` case requests a formula that exists
only in a grandchild file and also proves the unselected child formula is
absent.

## Skip/repeated-include behavior

`ScannerParseInclude` consults a caller-provided `skip_includes` tree, but the
visible standalone executable callers initialize that tree empty and this
source snapshot never populates it. Repeated includes are therefore parsed
repeatedly in C. Rust preserves that existing behavior; no implicit include
deduplication was introduced by the selector-stack fix.

## Reference and validation boundary

This desktop session has no installed WSL distribution, visible cached C
executable, or native POSIX C toolchain, so the new differential case could not
run against C here. The behavior is fixed from direct source inspection and
the permanent case is ready for the normal comparison command when the
reference environment is restored:

```powershell
cargo build --locked --release --bins
.\e-interop.ps1 build-reference
.\e-interop.ps1 compare-tools -RustBinDir .\target\release -Tool eground
```

Native validation at implementation time:

- `cargo test --locked --lib --quiet -- --test-threads=1`: 4,150 passed;
- all binary targets passed under `cargo test --locked --bins`;
- integration targets `eprover_schedule`, `e_stratpar`, and
  `executable_inventory`: 4, 1, and 1 passed;
- `cargo check --locked --all-targets`: passed;
- `cargo clippy --locked --all-targets -- -D warnings`: passed;
- `cargo fmt --all -- --check`: passed;
- `cargo build --locked --release --bin eground`: passed;
- focused record-dispatch and nested-selector regressions: 5 passed;
- bundled-Python `tools/e-interop` discovery: 32 passed; and
- optimized native `eground` matrix: all 17 expected outcomes passed;
- C-source documentation coverage: 492 sources / 266 unit docs;
- Change Later wording and local-link checks: 269 Markdown files each; and
- regeneration preservation: 268 Markdown files.
