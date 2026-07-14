# Documentation Index For Agents

Agent-made documentation belongs in this file and in the documentation locations linked from this file. Do not modify `AGENTS.md`; add or update agent-facing documentation here instead.

## Rust Port Standards

Rust implementation work must follow [`docs/rust-code-standards.md`](docs/rust-code-standards.md), including clippy pedantic checks. Unsafe Rust is prohibited except when narrowly required for interacting with external DLLs or shared libraries; this includes runtime-loaded solver libraries and native OS/CRT APIs such as libc, Win32, Winsock, Kernel32, or UCRT when the platform ABI requires FFI. Any such boundary must document its safety invariants and expose safe Rust APIs where practical.

Current Rust port implementation status is tracked in [`docs/rust-port-status.md`](docs/rust-port-status.md).

## WSL C/Rust Comparison

Build the upstream C references, compare the Windows Rust executable against them, and run the WSL-native benchmark with [`docs/windows-wsl-comparison.md`](docs/windows-wsl-comparison.md). That runbook also records the per-Windows-user WSL distro caveat that matters for Codex sandbox sessions.

## Runtime PicoSAT Selection

The Rust executable selects a runtime-loaded PicoSAT backend when `E_RUST_PORT_PICOSAT_LIBRARY` names a PicoSAT DLL/shared-library path. When that environment variable is unset or empty, the executable also looks for a bundled PicoSAT library next to `eprover`, under `lib/` next to the executable, and under `../lib/` relative to the executable directory. If no library is found, the port falls back to the internal solver.

## C Source Documentation

The original C implementation in `eprover/` is documented under [`docs/c_source_docs/`](docs/c_source_docs/). Treat `eprover/` as read-only original source; update documentation around it, not the source itself.

### C Change Later Notes

When porting new code or reviewing already-ported code, document C implementation details that may make sense to change after drop-in compatibility is secured. This includes accidental behavior, portability hazards, obsolete allocation patterns, global-state quirks, confusing API boundaries, ignored parameters, counter overflows, and performance tradeoffs. Put these notes in the relevant C-source page's manual-review `Change Later` section, or in a linked status/design doc when the issue spans multiple source units.

Retroactive audit status as of 2026-07-11: the existing C-source manual-review pages have been checked against this rule with `check_change_later_notes.py`, `generate_c_source_docs.py --check`, `check_regeneration_preserves_manual.py`, and `check_markdown_links.py`; later indexed-paramodulation, higher-order-dispatch, proof-state global-index ownership, typed `PStack` allocation, derivation-stack memory, term-bank release-assertion, shared term-argument, intrusive term-tree, shared help-footer, support-tool option/help, feature-line, learning-protocol, PCL statistics, term-DAG, TSM-output, autoschedule partial-match-output, scanner resolved-source-path, higher-order proof-rendering/type-declaration, formula input-marker, dummy-quote-collapse, AC-resolution parent-collapse asymmetry, proof-quote input-marker side effects, derived-PCL layout/formula-dialect, formula-to-clause normalization-order, pointer-tree traversal/destructive-merge ownership, free-variable definition-order, parser-probe/intrusive-term-store ownership, typed/higher-order clause-rendering allocation-order, demodulator-index coverage/lifecycle, selected-sort type-UID/allocator ordering, post-cache discrimination-tree query/cursor, indexed unit-subsumption side expansion, recursive clause-subsumption orientation backtracking, shared-variable live-PDTree rewrite, paramodulation normalization-order, unindexed-paramodulation derivation-target, and PDTree leaf pointer-order reviews also removed stale status claims and recorded C behaviors that should remain compatibility-visible. The WSL compatibility benchmark baseline is recorded in `docs/rust-port-status.md` so later ports cannot silently treat current performance gaps as complete. Continue applying the rule to newly ported code and to any stale `pending` or `remaining` status notes discovered during later reviews.
The 2026-07-13 contextual-simplify-reflect audit applied this rule retroactively to FV-index routing, indexed unit-query preconditions, and pointer-keyed FV-index leaf order; the detailed notes are in the paired `ccl_context_sr` and `ccl_subsumption` pages.

The 2026-07-13 indexed-paramodulation follow-up applied this rule retroactively to active-substitution lifetime and noncommutative metadata-parent ordering; the detailed notes are in the `cco_paramodulation` page.

The 2026-07-13 HEN011 throughput follow-up applied this rule retroactively to raw-parent HCB liveness, intrusive clause-set position lookup, per-call clause-subsumption scratch allocation, first-order matching job stacks, and raw term-argument-array access; the detailed notes are in the paired `che_hcb`, `ccl_clausesets`, `ccl_subsumption`, `cte_match_mgu_1-1`, and `cte_termfunc` pages.

The 2026-07-14 FV-index traversal follow-up applied this rule retroactively to the generic 64-pointer `PLocalStack` allocation used by each first-order match. The measured four-pair Rust inline capacity and the later C cleanup options are recorded in the paired `clb_plocalstacks` and `cte_match_mgu_1-1` pages.

The 2026-07-12 retroactive follow-up also reviewed C parent-liveness/archive coupling, object-tree payload ownership, and long-equation-list tautology search behavior under this rule.

The 2026-07-14 live-PDTree-substitution follow-up applied this rule retroactively to the already ported compact query and demodulator-index paths. The paired `ccl_pdtrees` review now records C's process-global traversal order, mutable cursor state and reusable traversal stack stored in every shared tree node, and raw-address leaf priority as later cleanup candidates while preserving the live-substitution performance contract.

The 2026-07-14 PDTree-query-reuse follow-up applied this rule retroactively to C's tree-owned reusable term traversal stack and callback. The paired `ccl_pdtrees` review records that allocation reuse is worth preserving, but later C should put the reusable query buffer and traversal continuation in an explicit search object rather than coupling them to a non-reentrant shared tree.

The 2026-07-14 iterative-PDTree-query follow-up applied this rule retroactively to C's reversible `TermLRTraverseNext`/`TermLRTraversePrev` pointer-stack API. The paired `ccl_pdtrees` review records its precedence-sensitive first-argument expression, assertion-only stack-shape contract, and shared-tree ownership as later cleanup candidates while retaining direct argument-array traversal performance.

The 2026-07-14 PDTree-query-metadata follow-up applied this rule retroactively to repeated higher-order term classification and root-weight evaluation in C's query, insertion, and search-initialization paths. The paired `ccl_pdtrees` review records a later one-pass classification boundary and invariant weight snapshot while retaining C's direct field/argument access and exact branch order.

The 2026-07-13 retroactive follow-up reviewed proof-state temporary-term-bank ownership, forward-contraction tautology scratch storage, unconditional selected-clause disjoint-copy allocation, term-bank sharing-key commentary, and formula-simplification coupling to bank-global GC roots under this rule.

Start here:

- [`docs/c_source_docs/overview.md`](docs/c_source_docs/overview.md) - subsystem map, coverage counts, porting guidance, and links to every source-unit page.
- [`docs/c_source_docs/review_status.md`](docs/c_source_docs/review_status.md) - review table for all documented C source units.
- Per-subsystem directories such as [`BASICS`](docs/c_source_docs/BASICS/), [`TERMS`](docs/c_source_docs/TERMS/), [`CLAUSES`](docs/c_source_docs/CLAUSES/), [`CONTROL`](docs/c_source_docs/CONTROL/), and [`HEURISTICS`](docs/c_source_docs/HEURISTICS/) contain the individual source-unit pages.

Current C-source documentation coverage:

- 492 original `.c`/`.h` files covered.
- 266 source-unit pages: `.c`/`.h` pairs are documented together; standalone `.c` or `.h` files get their own page.
- 268 Markdown files total under `docs/c_source_docs/`, including `overview.md` and `review_status.md`.

Each C-source documentation page has two protected regions:

- `<!-- BEGIN AUTO-GENERATED: c_source_docs -->` to `<!-- END AUTO-GENERATED: c_source_docs -->` contains mechanical inventory generated from the source tree.
- `<!-- BEGIN MANUAL REVIEW: c_source_docs -->` to `<!-- END MANUAL REVIEW: c_source_docs -->` contains manually reviewed notes and compatibility judgments.

Regeneration must not destroy manual documentation. Generated tooling may replace only the auto-generated region. Put hand-written source review, caveats, and porting observations in the manual-review region or in separate docs linked from this file.

## C Source Documentation Tooling

Use the repo-local virtual environment:

```powershell
.\.venv\Scripts\python.exe tools\c_source_docs\generate_c_source_docs.py --check
.\.venv\Scripts\python.exe tools\c_source_docs\generate_c_source_docs.py --generate
.\.venv\Scripts\python.exe tools\c_source_docs\apply_manual_review_notes.py
.\.venv\Scripts\python.exe tools\c_source_docs\check_change_later_notes.py
.\.venv\Scripts\python.exe tools\c_source_docs\check_markdown_links.py
.\.venv\Scripts\python.exe tools\c_source_docs\check_regeneration_preserves_manual.py
```

Command roles:

- `generate_c_source_docs.py --check` verifies every C/H file under `eprover/` maps to exactly one documented source unit.
- `generate_c_source_docs.py --generate` refreshes mechanical inventory sections while preserving manual-review sections.
- `apply_manual_review_notes.py` updates the preserved manual-review sections from the source-aware review-note helper.
- `check_change_later_notes.py` verifies C-source review docs use the standard `Change Later` section wording and do not reintroduce legacy candidate/observation headings.
- `check_markdown_links.py` checks local Markdown links in the C-source docs and this `DOCS.md` file.
- `check_regeneration_preserves_manual.py` regenerates docs and confirms manual-review sections are unchanged.

## Maintenance Workflow

1. Run `git status --short` before changing documentation.
2. Do not modify `eprover/`.
3. Add new agent-facing documentation to `DOCS.md` or to a linked docs location.
4. For new porting work and retroactive review of already-ported code, document aspects of the C implementation that may make sense to change later, including accidental behavior, portability hazards, obsolete allocation patterns, global-state quirks, or performance tradeoffs that should be revisited after compatibility is secured.
5. When retroactive review shows a `docs/rust-port-status.md` "pending" or "remaining" note is stale because the Rust code already implements that surface, update the status note in the same change.
6. For C-source pages, edit manual-review sections by hand when adding source-review knowledge.
7. Use generation only for source inventory and other mechanical updates.
8. Run the coverage, Change Later terminology, link, and regeneration-preservation checks.
9. Confirm the main worktree and the nested `eprover/` checkout are clean except for intended documentation changes.
10. Commit and push scoped documentation changes.
