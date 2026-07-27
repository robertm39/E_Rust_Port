# Detailed PROVER reconciliation

## Status

Accepted for the 82 remaining open `prover` records under Beads
`E_Rust_Port-j76.4`. Direct review found no missing production executable
behavior. Earlier "temporary" or "pending" notes are superseded where formula
owners, preprocessing, ordered proof extraction, resource handling, and the
main pipeline subsequently landed. Other records are intentional command-line
quirks, safe replacements for undefined C behavior, or source representation
details with no legitimate observable result. No Rust or C source changed.

## Review decisions

| Record | Decision |
|---|---|
| 1002 | Keep `checkproof --version` long-only. A `-V` alias would expand the C-compatible option surface. |
| 1006 | Preserve `scheme-setheo` parsing and the release build's unchecked/failure split; do not silently rename a public C option. |
| 1007 | Keep owned temporary-file cleanup plus compatible SIGTERM/SIGINT setup. Library callers do not inherit unmanaged temporary names. |
| 1012 | Preserve `classify_problem`'s early output create/truncate ordering; transactional output belongs to a separate API. |
| 1017 | Treat the classifier phase concern as resolved: represented formula owners enter CNF before the exact caller-specific clause preprocessing gates. |
| 1020 | Preserve early output opening in both feature-line and real-input classifier modes. |
| 1021 | Keep missing serialized feature fields initialized to zero. C stack residue is undefined and cannot be an exact contract. |
| 1023 | Keep one owned direct-examples writer while preserving early create/truncate behavior. C's redundant double-open has no supported distinct result. |
| 1027 | Preserve `e_axfilter` output creation before filter-file and missing-problem errors. |
| 1029 | Omit unused `app_encode` and statistics globals from the Rust API; no executable option or owner observes them. |
| 1030 | Preserve the seeded-largest/diverse last-handle effects because the exact seeded output matrix observes C's implementation. |
| 1034 | Keep `e_client` on the legacy `hello`/`add`/`prove` handshake; the deduction-server command protocol is a distinct executable. |
| 1036 | Preserve reserved-port warning and output creation before input/network failure. |
| 1037 | Preserve the separate stat, regular-file, and open diagnostics. |
| 1038 | Preserve the no-port stdout-unimplemented executable result while retaining the reusable internal text-session helper. |
| 1039 | Keep omitted and explicit-zero wall-clock limits distinct in typed configuration. |
| 1040 | Omit stale deduction-server globals/options that have no table entry or use. |
| 1041 | Preserve first-positional prover selection and ignored trailing arguments. |
| 1042 | Keep isolated per-client state and exact response framing; safe concurrent workers replace fork snapshots without shared mutation or output reordering. |
| 1045 | Preserve the accepted `division.category.training_data` token despite the paired printer's `training_directory` spelling. |
| 1047 | Preserve LTB output creation before positional-argument validation. |
| 1055 | Keep the exhaustive E option table, including aliases and executable no-ops, because parsing compatibility is public behavior. |
| 1056 | Preserve the legacy server's placeholder `Received`/`wait`/`ready` loop; do not manufacture a `result` response. |
| 1059 | Keep explicit startup-output flushing in the safe owner. It preserves visible output without reproducing an unreachable C close. |
| 1067 | Keep the reviewed release-compatible `3.3.5` version. Debug/LFHO build suffixes are not separate Rust deployment targets. |
| 1069 | Keep the C `http://www.eprover.org` URL in help/version output. |
| 1070 | Preserve `edpll` as the trace-producing unfinished driver ending in `Not completed yet!`; a working solver would change the C tool. |
| 1085 | Preserve KB creation order and partial-directory failure side effects in the compatibility executable. |
| 1090 | Preserve the generic "temporary file" removal diagnostic even for stored KB problems. |
| 1095 | Keep clause-variable policy as explicit parser configuration while preserving executable behavior; process-global mutation is unnecessary. |
| 1097 | Keep ownership-safe `ekb_ginsert` completion. Reproducing the unchanged C heap corruption is neither safe nor semantic compatibility. |
| 1106 | Continue allowing enormalizer runs with rules but no normalization targets. |
| 1112 | Preserve every visible C help typo and caveat in compatibility output. |
| 1122 | Keep `epclanalyse --version` long-only. |
| 1125 | Preserve `epclanalyse` output creation before default-input insertion and scanning. |
| 1129 | Always flush and unwind `epclextract` ownership. C `FAST_EXIT` leaks are not observable output or performance requirements. |
| 1132 | Preserve `epclextract` early output creation. |
| 1134 | Preserve comment forwarding during parsing, including output already emitted before later failure. |
| 1135 | Keep `epcllemma --version` long-only. |
| 1139 | Preserve early lemma-output creation and stdout status routing. |
| 1141 | Preserve C's effective double assignment to `pas_simpl_w`, leaving `act_simpl_w` unchanged. |
| 1142 | Preserve the single-precision relative-limit calculation and `+0.99` truncation, including a zero limit on tiny inputs. |
| 1145 | Prefer represented proof-state formula owners for supported input; retain the exact bridge only for C-accepted spellings the owner parser deliberately delegates. |
| 1147 | Preserve selected includes, `$TPTP` fallback, missing-selector diagnostics, and observable repeated includes through explicit parser state. |
| 1148 | Preserve mixed old-TPTP and modern TSTP wrappers when TSTP mode is selected. |
| 1149 | Treat full formula/CNF ownership as landed. The remaining bridge is a compatibility parser adapter, not a missing clausifier or owner. |
| 1150 | Keep the unified Rust executable capable of represented THF owners while retaining explicit diagnostics for unsupported/full-pipeline residues. |
| 1151 | Preserve beta normalization of named lambda applications before supported lowering. |
| 1152 | Preserve transparent parenthesized application lookahead until the represented parser consumes those spellings directly. |
| 1153 | Preserve bare-arrow lambda equality recovery and represented higher-order promotion. |
| 1154 | Preserve THF application parsing before equality/connective lowering. |
| 1155 | Preserve typed quantified THF application atoms through the represented/bridge boundary. |
| 1156 | Preserve unary negation consuming a simple following application. |
| 1157 | Preserve nested non-Boolean THF application arguments. |
| 1158 | Preserve left-associative bare application and rejection of a bare Boolean argument that would require implicit regrouping. |
| 1160 | Preserve parenthesized logical heads and Boolean operand boundary handling. |
| 1161 | Keep parsed problem type explicit through proof-state phases; the parser's aggregate dialect state remains a compatibility boundary, not a proof-search global. |
| 1162 | Continue threading explicit parsed problem type into app-encode type rendering. |
| 1163 | Treat represented FOOL ownership/CNF as primary and retain the fallback only for exact accepted surface spellings. |
| 1164 | Preserve let-formal masking in free-variable and dependency scans plus positive generated-definition lowering. |
| 1166 | Preserve the LPO recursion-limit mutation, immediate warning, and fallthrough into restricted literal comparison. |
| 1167 | Reject executable `RPO` early while retaining it in shared strategy parameter enums, matching C's split surface. |
| 1169 | Keep typed executable configuration and explicit conversion into `HeuristicParmsCell`, index parameters, and initialized `ProofControl`; global mutation is not needed. |
| 1170 | Preserve the coupled observable CNF-only option/output state behind a typed mode and explicit output policy. |
| 1172 | Treat auto preprocessing/search scheduling and option provenance as complete for clause and formula owners. |
| 1173 | Treat SInE ordering and Threshold/GSinE/LambdaDef replacement across live clause/formula owners as complete. |
| 1174 | Preserve prune-only's strict exit before CNF, clause preprocessing, and search. |
| 1175 | Preserve order-sensitive auto-detected output/documentation format in per-run configuration. |
| 1178 | Preserve app-encode's phase order, stdout side channel, include echo/no-load policy, and represented formula rendering. |
| 1180 | Treat every result banner/status branch as implemented and regression-pinned, including higher-order exhaustion and resource exits. |
| 1185 | Preserve the level-two success quote before the proof-found banner. |
| 1188 | Treat ordered mixed proof-object extraction as complete: root policy, 56 derivation codes, list/DOT/statistics, formula/AC ancestry, and level-zero suppression are pinned. |
| 1189 | Preserve detailed-statistics GC ordering and keep optional measurement counters behind explicit Cargo features. |
| 1190 | Keep the exact performance-counter block position and represented call-site ownership; unrepresented compile-time instrumentation is not ordinary output. |
| 1192 | Preserve formula-set pretty TSTP rendering for syntax-only `--print-formulas`, independent of clause output format. |
| 1193 | Keep fixed internal symbol-code reservation in every executable parser bank. |
| 1194 | Keep `print_types` as explicit per-run rendering configuration rather than a process global. |
| 1195 | Preserve resource footer shape and successful-exit placement, with timeout suppression owned by the signal/resource layer. |
| 1196 | Preserve CPU/core/memory setup order and exact warning/perror text without mutating host limits in unit tests. |
| 1197 | Treat the main stateful pipeline as complete: formula preprocessing, schedules, watchlists, indexed saturation, ordered proof output, and statistics all have owned production paths. |
| 1198 | Return false when the malformed `termprops` commutativity probe lacks the second unary-child argument; never reproduce C's out-of-bounds read. |
| 1202 | Preserve TSM classifier file/stdin concatenation with no inserted separators; scanner chaining may not alter diagnostics or ordering. |

## Evidence

The retained implementation and reference studies cover every decision:

- all 21 reviewed PROVER units have Rust owners, and every executable owner has
  a thin binary wrapper;
- option/help, early-output, file-diagnostic, placeholder, protocol, KB,
  normalizer, PCL, and undefined-state boundaries have focused regressions;
- the maintained support-tool matrix covers all 216 cases with zero unexpected
  differences, including safe declared completion where unchanged C aborts;
- formula-owner modes are exact in 28/28 canonical runs, while permanent tests
  cover old/modern wrappers, includes, THF applications/lambdas, FOOL terms,
  scheduling, SInE, prune, CNF, and app-encode phase order;
- the preprocessing umbrella is 29/29 and the formula SInE owner closure is
  16/16;
- ordered mixed proof extraction owns exact C topology/order and all 56
  derivation codes; and
- resource limits, result/status branches, print types, detailed statistics,
  and the complete owned proof-search pipeline are regression-pinned.

The latest exact candidate passes 4,429 tests, all 50 main-prover cases, and
all 216 support-tool cases with zero unexpected differences.

## Audit

[`audit_prover_reconciliation.py`](audit_prover_reconciliation.py) pins the
exact 82 migrated identities and content hashes, checks ten grouped
source/implementation/evidence contracts, and digests the 21 unchanged C
units, 21 Rust owners, 19 binary wrappers, status ledger, sixteen retained
findings, and current validation reference. The audit is independent of issue
status, so it remains reproducible after closure.

## Validation

The source audit, Python syntax check, C-source documentation coverage, Change
Later wording, local links, manual-regeneration preservation, and
`git diff --check` pass. The unchanged implementation is covered by the exact
Experiment 046 lifecycle:

- Rustfmt and strict all-target/all-feature pedantic Clippy pass;
- 4,418 library plus 11 integration tests pass, 4,429 total;
- native release and compile-only Windows GNU x64 all-target/all-feature builds
  pass; and
- 50 main plus 216 support-tool comparisons have zero unexpected differences.

No Rust or C toolchain ran on the local Windows host. The vendored C checkout
is clean.

Reproduce the source audit locally:

```powershell
.\.venv\Scripts\python.exe experiments/2026-07-25-052-prover-reconciliation/audit_prover_reconciliation.py `
  --repo . `
  --expected experiments/2026-07-25-052-prover-reconciliation/audit-reference.json
```
