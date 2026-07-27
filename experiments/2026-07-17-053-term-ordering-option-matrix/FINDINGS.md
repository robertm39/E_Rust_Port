# Term-ordering option matrix

## Status

Completed for Bead `E_Rust_Port-j76.2.85`. The executable surface is 73/73
byte-exact against C references from the pinned source commit after one
production compatibility fix. The vendored C checkout remained unchanged.

## Question

Are the executable term-ordering options fully materialized into proof-control
ordering behavior, and do the migrated Lambda-order/owner-bank claims still
describe missing production integration?

## Reference variants

The C option-name tables are compile-time dependent. The ordinary FOL build
omits eight type/combined-frequency weight methods and four type/combined-
frequency precedence methods guarded by `ENABLE_LFHO`; its executable also
rejects THF input. Rust intentionally provides the union of the C FOL and HO
features in one binary.

[`compare_surfaces.py`](compare_surfaces.py) therefore selects the matching C
oracle per case:

- the cached FOL executable from commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0` for ordinary options and FOL proof
  runs; and
- an isolated `./configure --enable-ho` build of the same commit for LFHO-only
  names, their complete diagnostic lists, and the THF Lambda-order proof run.

The isolated HO build was made under the WSL cache, not in `eprover/`. Exit
status, stdout bytes, and stderr bytes are compared without normalization.
Fixture paths are translated to WSL, and arguments containing WSL shell
metacharacters are shell-quoted so a precedence such as `f>g` remains one argv
value.

## Matrix

The 73 cases cover:

| Case group | Cases | Result |
| --- | ---: | :---: |
| six executable orderings | 6 | exact |
| every accepted FOL/HO weight-generation method | 34 | exact |
| every accepted FOL/HO precedence-generation method | 19 | exact |
| combined overrides, optional defaults, literal restriction, LFHO weights | 4 | exact |
| invalid values and large-LPO-warning ordering | 10 | exact |

The combined proof run covers generated weights and precedence, late user
weights, all five precedence occurrence modifiers, constant weight, predefined
precedence, LPO recursion limit, literal comparison, and lambda/DB weights. The
THF run selects KBO6 Lambda-order with non-default lambda and DB weights.

The compact hashes and complete mismatch payloads, if a future run regresses,
are retained in [`results-summary.json`](results-summary.json).

## Corrected user-weight reporting

C's `TOGenerateWeights` writes `setting user weights` directly to stderr before
parsing any non-null `pre_weights` string. The initial Rust implementation
correctly applied late OCB overrides but omitted that observable line.

The executable proof-search path now emits the exact line when final strategy
parameters select KBO or KBO6 and contain `to_pre_weights`. Reusable ordering
helpers remain output-free. A paired permanent regression verifies both the
KBO6 side effect and C's LPO behavior, which ignores the weight override without
printing the line.

## Ownership and residual scope

The migrated production-integration claim is stale. Proof control owns the OCB
and threads the live mutable term bank through KBO6/LPO4 ordering preparation,
forward modification, rewrite and paramodulation side checks, equality
resolution/factoring, and every table-visible ordering-dependent literal
selector. Exact LFHO strategy runs already cover indexed and unindexed
inference use of those paths.

Legacy no-bank helpers and C's implicit owner-bank/cached-WHNF behavior remain
explicit post-compatibility API/performance questions under the existing
`cto_kbolin`, `cto_lpo`, and `cte_termbanks` review Beads. Partial predefined
precedence policy remains under `.3.518`/`.4.834`, and C's CLI help/rejection
quirks remain under `.3.220` through `.3.223`. The lower-level ordering
generation and auto-selection audit umbrellas remain independently tracked by
`.2.67` and `.2.65`; they are not missing executable option integration.

## Permanent Rust coverage

Existing tests pin CLI defaults/overrides, optional arguments, the C LPO-limit
fallthrough and warning timing, invalid diagnostics, conversion into canonical
`OrderParmsCell` values, explicit strategy overlays, OCB creation and selection,
generated/predefined precedence, generated/overridden weights, literal
comparison validation, bank-backed Lambda-order/LPO4 comparison, and production
proof-control selector/inference paths. The new regression adds the remaining
user-weight stderr boundary.

## Validation

- dual-oracle executable matrix: 73/73 exact;
- focused user-weight executable regression: passed;
- full all-target, all-feature Rust suite: 4,260 library tests plus every binary
  and integration target passed under Cargo's default parallel runner;
- strict pedantic Clippy: passed;
- release `eprover` build: passed; and
- formatting, experiment compilation, and all four C-source documentation
  integrity gates: passed.
