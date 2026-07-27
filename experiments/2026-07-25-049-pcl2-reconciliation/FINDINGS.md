# Detailed PCL2 reconciliation

## Status

Accepted for the 24 remaining open `pcl2` records under Beads
`E_Rust_Port-j76.4`. Direct review found no missing production PCL2 behavior.
The records describe intentional compatibility quirks, accepted diagnostics in
place of C null dereferences or allocator residue, safe Rust ownership
substitutions, or already tested interfaces. No Rust or C source changed.

## Review decisions

| Record | Decision |
|---|---|
| 926 | Keep Rust's syntax diagnostic for a missing quoted proof-distance parent. The unchanged C path dereferences the null lookup and the retained native probe terminates by signal; no supported output depends on that crash. |
| 927 | Preserve the C-shaped silent no-op for a missing quoted generation-reference parent. Direct quotes are leaves and compound fallback recursion remains exact. |
| 932 | Preserve stored positions in PCL rendering and omit them from TSTP inference rendering exactly like C. Adding positions to TSTP would change proof output. |
| 935 | Keep fullstop-separated decimal PCL identifiers. The C prose saying components are space-separated is stale; scanner code and executable syntax agree with Rust. |
| 937 | Keep `Vec<i64>` with authoritative length instead of C's zero-filled `PDArray` plus `-1` terminator. Zero remains a live component, exhaustion still compares through the C sentinel, and all printed forms are exact. |
| 938 | Keep deterministic zero defaults for C's 15 unassigned inference-opcode weight slots. Archived allocator residue happened to be zero in the retained executable probe; exposing uninitialized memory is neither safe nor a stable contract. |
| 942 | Keep missing parents non-fatal during reference-counter updates. C dereferences a missing top-level quoted parent, while nested missing parents are ignored; Rust safely retains the useful counter outcome without the crash. |
| 944 | Preserve miniclauses as literal-only snapshots. Rebuilding resets clause properties, roles, derivations, and other metadata just as `ClauseAlloc` does in C. |
| 945 | Keep cloned shared term handles in the compact representation. They preserve C pointer identity while making the borrowed-term lifetime explicit and safe. |
| 947 | Keep format, problem type, and equation options as print-call arguments. Rebuilding a temporary clause and dispatching through the ordinary printers preserves C output without process-global leakage. |
| 948 | Continue omitting `MiniClauseAddTerms`. The duplicate helper is not declared in the public header and has no C caller; porting it would add an unused API rather than compatibility. |
| 957 | Allow standalone mini-step id zero to drop safely. The parser accepts zero, so reproducing `PCLMiniStepFree`'s contradictory nonzero assertion would introduce an avoidable crash. |
| 958 | Preserve the mini-step extra field's narrow single-quoted-string grammar. Full steps intentionally accept `Name` and `PosInt` as well; widening mini syntax would be incompatible. |
| 963 | Preserve proofcheck's shell command text, temporary files, stdout-only capture, fixed 180-byte `fgets` chunking, C-string truncation, and substring markers. Rust additionally recognizes real E's single-percent `% Proof found!`, the already declared correctness fix. |
| 964 | Preserve clausal-precondition collection and FOF warnings. Full FOF clausification is absent from the C checker and is not required to reproduce its result surface. |
| 965 | Preserve one polarity-flipped hypothesis unit per target literal and the resulting fresh-clause metadata. This is the exact C proof-check problem construction. |
| 966 | Preserve split steps as `CheckNotImplemented`, the “assuming true” report, and exclusion from the checked count. Implementing split validation would be a new checker capability. |
| 967 | Preserve Otter and SPASS problem dialects, truth-literal hacks, command shapes, and markers behind explicit per-call prover selection. These legacy quirks remain deterministic compatibility surfaces. |
| 973 | Keep duplicate insertion fatal while reporting `step_count()` as actual membership. C increments its cached count before detecting the duplicate, but parsing terminates before that value can be observed. |
| 976 | Preserve FOF stripping's justification-only reset: dependents receive an initial expression without setting `PCLIsInitial`. Correcting the property would change later protocol analysis. |
| 979 | Keep shell-PCL permission per parse call, with the exact executable matrix: enabled for `epclextract` and disabled for the other current PCL tools. This replaces mutable process-global state without changing accepted input. |
| 981 | Preserve `que` as an accepted external type while omitting it from the fallback `conj|neg|lemma` diagnostic. Both the accepted token and stale error surface are regression-tested. |
| 986 | Keep example printing explicit about format, problem type, and equation options. The C routine reaches `ClausePrint` globals; call-local ownership produces the same bytes and prevents cross-call leakage. |
| 987 | Preserve the missing final period on formula `input_formula(...)` TPTP output. The clausal path uses its ordinary printer, while C's formula branch closes only with `)`. |

## Evidence

Ten retained PCL2 investigations already exercise these boundaries:

- analysis probes distinguish the C proof-distance null dereference from the
  intentionally silent generation-reference no-op;
- expression and identifier studies pin position omission, fullstop syntax,
  zero components, sentinel comparison, and structural storage;
- lemma tests cover every assigned and unassigned opcode weight plus dangling
  top-level and nested references;
- miniclauses retain literal order, signs, shared term identity, metadata loss,
  temporary-clause printing, and call-local output options;
- mini/full step studies cover id zero, different extra grammars, shell
  permission, external types, examples, and every supported output format;
- protocol tests cover fatal duplicates, membership counts, ownership
  transfer, parent collection, and FOF stripping; and
- proofcheck tests cover clausal/FOF boundaries, target negation, split
  accounting, E/Otter/SPASS problem and command shapes, output chunking,
  markers, warnings, and final summaries.

The latest exact candidate passes 4,429 tests, all 50 main-prover cases, and all
216 support-tool cases with zero unexpected differences.

## Audit

[`audit_pcl2_reconciliation.py`](audit_pcl2_reconciliation.py) pins the exact
24 migrated identities and content hashes, checks ten grouped
source/implementation/evidence contracts, and digests the 18 unchanged C
units, nine Rust owners, status ledger, ten retained PCL2 findings, and current
validation reference. The audit is independent of issue status, so it remains
reproducible after closure.

## Validation

The source audit, Python syntax check, C-source documentation coverage,
Change Later wording, local links, manual-regeneration preservation, and
`git diff --check` pass. The unchanged implementation is covered by the exact
Experiment 046 lifecycle:

- Rustfmt and strict all-target/all-feature pedantic Clippy pass;
- 4,418 library plus 11 integration tests pass, 4,429 total;
- native release and compile-only Windows GNU x64 all-target/all-feature
  builds pass; and
- 50 main plus 216 support-tool comparisons have zero unexpected differences.

No Rust or C toolchain ran on the local Windows host. The vendored C checkout
is clean.

Reproduce the source audit locally:

```powershell
.\.venv\Scripts\python.exe experiments/2026-07-25-049-pcl2-reconciliation/audit_pcl2_reconciliation.py `
  --repo . `
  --expected experiments/2026-07-25-049-pcl2-reconciliation/audit-reference.json
```
