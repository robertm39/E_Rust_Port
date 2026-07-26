# Experiment 330: Post-compatibility backlog audit

## Status

The first audit phase is complete for Beads epic `E_Rust_Port-j76.3`.
Detailed-item reconciliation for `E_Rust_Port-j76.4` remains pending.

## Question

Which migrated post-compatibility records already contain a concrete
preserve/change decision, and which still describe transitional or incomplete
Rust surfaces that need individual implementation review?

## Baseline

- The compatibility and performance milestone is closed at commit `43047e50`.
- Its fresh comprehensive Linode report passes 4,419 Rust tests, strict
  formatting and Clippy, Linux and Windows GNU x64 builds, clean FOL and
  higher-order C reference builds, 50 main cases and 216 support-tool cases
  with zero unexpected differences, ten behavior-exact benchmark cases, and
  the maintained `1.0801753448x` Rust/C aggregate performance target.
- `E_Rust_Port-j76.3` contains 649 immutable summaries migrated from the former
  `C Behaviors To Revisit After Compatibility` section.
- `E_Rust_Port-j76.4` contains 1,327 C-source review records, of which 14 were
  already closed by focused implementation slices.

## Method

`audit_backlog.py` reads Beads through `bd list` without changing issue state.
For every child it:

1. extracts the immutable legacy work-item text;
2. recomputes and validates its migrated SHA-256;
3. validates unique ids and contiguous migrated ordinals;
4. records whether its source file and exact text still exist in the current
   documentation; and
5. conservatively routes any record mentioning remaining, incomplete,
   transitional, bridge, or future port/integration work to manual review.

Records outside that conservative signal set are candidates for a documented
post-milestone decision to retain the compatibility behavior. The script does
not close or update Beads.

Exact command:

```powershell
.\.venv\Scripts\python.exe `
  experiments/2026-07-25-029-post-compat-backlog-audit/audit_backlog.py `
  --parent E_Rust_Port-j76.3 `
  --parent E_Rust_Port-j76.4 `
  --output .artifacts/post-compat-backlog-audit/inventory.json
```

## Results

The retained inventory is
`.artifacts/post-compat-backlog-audit/inventory.json`.

All 1,976 migrated child records were inventoried:

| Epic | Children | Standard hashes validated | Nonstandard descriptions | Exact text still in current source |
| --- | ---: | ---: | ---: | ---: |
| `E_Rust_Port-j76.3` | 649 | 649 | 0 | 0 |
| `E_Rust_Port-j76.4` | 1,327 | 1,324 | 3 | 1,125 |

The zero current-source matches for `j76.3` are expected: commit `7d658217`
migrated and then removed the old status-page bullet section. The three
nonstandard detailed descriptions are `j76.4.662`, `j76.4.1326`, and
`j76.4.1327`; their current descriptions do not use the standard immutable
`## Legacy Work Item` envelope, so the script records their hash check as not
applicable rather than claiming a false match.

The summary review closed 609 additional `j76.3` children:

- 334 records contained an affirmative Rust preserve/change decision and no
  incomplete or transitional signal.
- 151 records were manually reviewed from the no-decision and provisional-Rust
  groups; later ownership, parser, autoschedule, platform, proof-state, or
  compatibility evidence superseded the legacy wording, or the recorded safe
  Rust boundary was retained deliberately.
- 124 further records with conservative review signals were checked against
  current source, tests, and focused experiments. This group includes the
  now-exact performance-counter surface, the 18/18 higher-order
  `ForwardModifyClause` audit, complete grounding, stable PD-tree occurrence
  identity, and wired local-rewrite/proof-documentation paths.

Together with six children closed by earlier focused slices, `j76.3` now has
615 closed children and 34 open implementation records.

The 34 retained records are:

- Formula/parser ownership (18): `j76.3.54`, `.55`, `.56`, `.57`, `.58`,
  `.59`, `.61`, `.62`, `.63`, `.90`, `.98`, `.101`, `.427`, `.502`, `.539`,
  `.540`, `.552`, and `.553`.
- LFHO term-bank/cache ownership (12): `j76.3.294`, `.326`, `.331`, `.349`,
  `.366`, `.634`, `.640`, `.642`, `.643`, `.644`, `.646`, and `.649`.
- Other implementation gaps (4): pattern-computation cutoff propagation
  (`j76.3.162`), interpreted arithmetic integration (`j76.3.219`),
  higher-order feature extraction (`j76.3.486`), and typed term-GC owner
  registration (`j76.3.593`).

The Beads close reasons reference both this audit and Experiment 329's fresh
final compatibility/performance run. No code was changed during reconciliation.

## Falsification rule

Do not bulk-close a record merely because it passed mechanical integrity
checks. A record stays open if its text describes missing C functionality,
ambiguous ownership that could affect behavior, a temporary executable bridge,
or another claim that cannot be resolved from the final compatibility evidence
and current source documentation.

## Conclusion

The migrated summary epic was mostly durable historical review, not 649
independent missing implementations. Its resolved compatibility decisions are
now closed with a common evidence trail, while every description that still
identifies unported behavior remains open. The resulting 34-item set is the
actionable implementation boundary for completing `j76.3`.

## Limits

- Signal matching is intentionally conservative but is not semantic proof.
- The current main/tool matrices are broad executable evidence, not exhaustive
  direct coverage for all internal helper contracts.
- Records routed to manual review require source, implementation, and focused
  test evidence before closure.
- This phase does not resolve the 1,313 open detailed `j76.4` children; those
  require a second source-unit reconciliation after the summarized
  implementation gaps are closed.
