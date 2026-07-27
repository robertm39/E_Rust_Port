# Definition and encoding option-effect reconciliation

## Status

Completed for Bead `E_Rust_Port-j76.2.41`. All 12 migrated option routes pass
an 83-check C/Rust static audit, and all 16 focused executable cases match the
isolated higher-order C reference on their declared comparison surfaces. Two
observable incompatibilities were corrected. The vendored C checkout remained
unchanged.

## Question

Do definition, free-symbol, formula-CNF, typed-output, app-encoding, and
higher-order extension options reach the same production consumers as C, and
do their externally visible effects match?

## Static routing audit

[`audit_option_effects.py`](audit_option_effects.py) follows each CLI spelling
through both implementations for:

- `--define-weight-function` and `--define-heuristic` into production
  `ProofControlInit`/`proof_control_init_with_formula_axioms` parsing;
- `--free-numbers` and `--free-objects` into proof-state signature policy;
- `--definitional-cnf`, `--fool-unroll`, and `--miniscope-limit` into formula
  clausification;
- `--print-types` and `--app-encode` into their output owners; and
- `--arg-cong`, `--neg-ext`, and `--pos-ext` into higher-order generation.

The audit also pins C's historical negative-extensionality gate around
`ComputePosExt`, the corresponding Rust gate, the three zero-exit invalid-mode
paths, and scanner-position ownership for distinct-symbol argument errors. The
retained report passes 83/83 checks and has SHA-256
`f00e74a44161b04d7bbec5c1500c3852fe525f3005523db20bbbbcd2349ca2ef`.

## Executable matrix

[`compare_option_effects.py`](compare_option_effects.py) compares the Windows
Rust release executable with the isolated `--enable-ho` C build from upstream
commit `17026b1bfe61aaf223cfaae54947c8d2679c31a0`. The C executable SHA-256 is
`317e261b4915d16834de9f5a133ecd07fe6e21dfdc8c5f06072ed75b3e56b7e1`;
the final Rust executable SHA-256 is
`61acf0bb516ae3295174f9bc8b7de030ee05de7132e3fa10aae8bc961079ad34`.

All 16 cases match:

| Case group | Cases | Result |
| --- | ---: | :---: |
| current strategy with six CNF/extension overrides | 1 | exact |
| invalid CNF ranges/Boolean value | 3 | exact |
| invalid ArgCong/NegExt/PosExt modes | 3 | exact |
| custom weight-function and heuristic proof run | 1 | exact |
| default rejection and enabled acceptance for free numbers/objects | 4 | exact |
| enabled and disabled FOOL unrolling | 2 | exact |
| typed clause output | 1 | exact |
| application encoding with typed declarations | 1 | exact after declared ordering normalization |

Raw strategy and invalid-option cases compare exit status, stdout, and stderr
byte for byte after line-ending normalization. Free-symbol rejections also
compare the full source path, line, column, current token, message, and exit
status after replacing only the platform-specific repository prefix and path
separator. The proof/CNF/typed cases normalize generated clause identifiers
and repository paths if present; neither affects these retained transcripts.

Application encoding uses the already documented declaration-order decision:
C walks pointer-hashed type buckets and changes order between processes, while
Rust uses stable UID order. The comparison sorts each `%--` type-comment and
`tff(typedecl...)` pair together and normalizes only the generated declaration
label. Type UIDs, symbol declarations, and the encoded formula remain exact.

The retained comparison report has SHA-256
`44adc26c995379a7d4d0802e7a6de9bb55db44883b08b0a4119e22d08250c196`.

## Compatibility fixes

### Invalid extension-mode exit status

C reports invalid `--arg-cong`, `--neg-ext`, and `--pos-ext` values through
`Error(..., 0)`: each exact diagnostic is printed, but the process exits 0.
Rust used the normal usage-error code and exited 5. The shared extension-mode
parser now preserves C's zero-exit diagnostic behavior, with unit and
executable coverage for all three spellings.

### Distinct-symbol scanner context

C's `TBTermParseReal` reports forbidden numeric/object argument lists through
`AktTokenError`, including source, line, column, and the current `(` token.
Rust retained the reason but returned it without scanner context. The shared
checked term parser now constructs the same scanner-position diagnostic for
integer, rational, floating-point, and object branches. Exact permanent tests
pin number/object formatting, and both executable file cases are exact.

## Existing supporting evidence

This focused result composes with three earlier retained investigations:

- the 47/47 exact weight-parser matrix covers all 46 weight-function table
  entries plus anonymous definitions in production proof-state context;
- the app-encode ownership matrix proves exact typed-application declaration
  ownership under the same declaration-order normalization; and
- the 69/69 formula-pipeline audit plus LFHOL comparisons prove the shared CNF,
  free-symbol, typed-output, and higher-order dispatcher owners beyond the
  small option fixtures used here.

## Scope decision

The migrated umbrella is resolved: every named option is parsed, materialized,
and consumed at the C-equivalent production boundary, with focused observable
effects and invalid paths covered. Duplicate definition-name allocation,
remaining full-parser formula shapes, and broader proof-search differences are
owned by narrower existing Beads rather than this option-integration item.

## Validation

- static C/Rust routing audit: 83/83 checks across 12 options;
- focused executable comparison: 16/16 matching cases;
- permanent Rust regressions for zero-exit extension errors and exact
  distinct-symbol scanner diagnostics;
- retained-reference reruns for both experiment scripts;
- full all-target/all-feature Rust suite and strict pedantic Clippy;
- release `eprover` build and all C-source documentation integrity gates; and
- clean nested `eprover/` worktree.
