# FormulaAndClauseSetParse completion audit

## Status

Completed for Bead `E_Rust_Port-j76.1.44`. This slice completes the shared
outer-record, include-selector, watchlist, and termination contract around the
represented formula owners. Lower term and formula body grammar remains owned
by its dedicated parser Beads rather than being duplicated in this dispatcher.
The vendored C source remained unchanged.

## C termination and caller contract

`FormulaAndClauseSetParse` does not itself require end of input. Its LOP branch
loops only while `ClauseStartsMaybe(in)`. Its non-LOP branch loops only while
the current identifier is one of `input_formula`, `input_clause`, `fof`, `cnf`,
`tff`, `thf`, `tcf`, or `include`. In both branches, the first token outside
that recognized-start set remains unread.

The observable result depends on the caller:

- `eprover.c` and `eground.c` immediately call `CheckInpTok(in, NoToken)` after
  the shared parser and therefore reject an unrecognized top-level tail;
- `epatternize.c` proceeds directly to preprocessing and later destroys the
  scanner, so the same tail is ignored; and
- a recursively included scanner is destroyed after the recursive shared
  parse without an additional EOF check, preserving the same inner boundary.

The Rust dispatcher now preserves that split. Its shared entry-list functions
leave the first unrecognized outer token current, while `eprover`,
`--app-encode`, and `eground` perform their caller-owned EOF checks.
`epatternize` deliberately does not add one.

## Known-record dispatch

Stopping on an unknown identifier must not turn a known record into a silent
no-op. C uses the same eight-name outer guard in old-TPTP and TSTP scanner
modes, then lets `WFormulaParse` or `WFormClauseParse` apply the selected
format. Thus explicit old-TPTP input that starts with `fof(...)` still enters
the formula parser and fails expecting `input_formula`; `cnf(...)` similarly
enters the clause parser and fails expecting `input_clause`.

Rust now uses the same recognized-name guard in the normal represented-owner
and app-encode entry lists. Focused tests prove both the silent stop for an
unknown name and lower-parser dispatch for known modern names under explicit
old-TPTP mode.

## Existing wrapper and include coverage

The concentrated record matrix from experiment 048 already covers raw LOP;
legacy `input_clause` and `input_formula`; modern `cnf`, `fof`, `tff`, `tcf`,
and `thf`; type declarations; numeric and quoted names; optional source and
useful-info fields; legacy and modern watchlists; nested selector order;
discarded THF records; and repeated includes. This completion audit treats
those outer wrapper/ownership contracts as complete.

It does not claim every `TFormula` or term grammar spelling is complete. Known
gaps remain tracked at their lower parser owners, including Beads
`E_Rust_Port-j76.2.89`, `.2.103`, `.2.105`, and `.2.111`.

## Permanent executable evidence

The `epatternize` matrix grows from 13 to 16 cases with one unrecognized-tail
case per accepted input format. Each optimized run exits 0, writes exactly
`$or1($eq(f1_1(f0_1),$true))`, and produces no diagnostic. The existing
`eground/trailing-token` case remains exit 3 with its exact EOF diagnostic,
proving that moving strictness out of the shared parser did not weaken the C
caller contract.

`run_native.py` reuses the preceding 13-case experiment's exact goldens, adds
the three tail cases in permanent-matrix order, and validates all 16 optimized
`epatternize` cases before separately asserting the exact `eground` caller
failure. The archived C report still proves the original help/version/basic
executable cases. This session has no available C binary or WSL reference
environment for rerunning the three new tail cases; their expected behavior is
directly source-audited from the loop and caller code above.

## Validation

- focused shared-parser, app-encode, syntax-only, and eground caller tests:
  passed;
- full library suite: 4,164 passed;
- all binary targets passed;
- integration targets `eprover_schedule`, `e_stratpar`, and
  `executable_inventory`: 4, 1, and 1 passed;
- `cargo check --locked --all-targets`: passed;
- `cargo clippy --locked --all-targets -- -D warnings`: passed;
- `cargo fmt --all -- --check`: passed;
- bundled-Python `tools/e-interop` discovery: 32 passed;
- release build of all binaries: passed;
- optimized `epatternize` matrix: all 16 expected outcomes passed;
- optimized `eground` caller boundary: exact status/stdout/stderr passed;
- C-source documentation coverage: 492 sources / 266 unit docs;
- Change Later wording and local-link checks: 269 Markdown files each; and
- regeneration preservation: 268 Markdown files.
