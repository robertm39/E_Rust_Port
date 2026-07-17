# Documentation Index For Agents

Agent-made documentation belongs in this file and in the documentation locations linked from this file. Do not modify `AGENTS.md`; add or update agent-facing documentation here instead.

## Rust Port Standards

Rust implementation work must follow [`docs/rust-code-standards.md`](docs/rust-code-standards.md), including clippy pedantic checks. Unsafe Rust is prohibited except when narrowly required for interacting with external DLLs or shared libraries; this includes runtime-loaded solver libraries and native OS/CRT APIs such as libc, Win32, Winsock, Kernel32, or UCRT when the platform ABI requires FFI. Any such boundary must document its safety invariants and expose safe Rust APIs where practical.

Implemented Rust-port history and compatibility evidence are recorded in [`docs/rust-port-status.md`](docs/rust-port-status.md). Active work is tracked canonically in Beads under root epic `E_Rust_Port-j76`; use `bd ready`, `bd list`, and `bd search` to inspect current status.

## WSL C/Rust Comparison

Build the upstream C references, compare the Windows Rust executable against them, and run the WSL-native benchmark with [`docs/windows-wsl-comparison.md`](docs/windows-wsl-comparison.md). That runbook also records the per-Windows-user WSL distro caveat that matters for Codex sandbox sessions.

## Runtime PicoSAT Selection

The Rust executable selects a runtime-loaded PicoSAT backend when `E_RUST_PORT_PICOSAT_LIBRARY` names a PicoSAT DLL/shared-library path. When that environment variable is unset or empty, the executable also looks for a bundled PicoSAT library next to `eprover`, under `lib/` next to the executable, and under `../lib/` relative to the executable directory. If no library is found, the port falls back to the internal solver.

## C Source Documentation

The original C implementation in `eprover/` is documented under [`docs/c_source_docs/`](docs/c_source_docs/). Treat `eprover/` as read-only original source; update documentation around it, not the source itself.

### C Change Later Notes

When porting new code or reviewing already-ported code, document C implementation details that may make sense to change after drop-in compatibility is secured. This includes accidental behavior, portability hazards, obsolete allocation patterns, global-state quirks, confusing API boundaries, ignored parameters, counter overflows, and performance tradeoffs. Put these notes in the relevant C-source page's manual-review `Change Later` section, or in a linked status/design doc when the issue spans multiple source units. The review text remains the technical source-analysis record, while task state is canonical in Beads under epic `E_Rust_Port-j76.4`. Every new top-level `Change Later` item must also create or update a Beads task labeled `source-c-review-change-later`, with source-file and content-hash metadata.

Retroactive audit status as of 2026-07-11: the existing C-source manual-review pages have been checked against this rule with `check_change_later_notes.py`, `generate_c_source_docs.py --check`, `check_regeneration_preserves_manual.py`, and `check_markdown_links.py`; later indexed-paramodulation, higher-order-dispatch, proof-state global-index ownership, typed `PStack` allocation, derivation-stack memory, term-bank release-assertion, shared term-argument, intrusive term-tree, shared help-footer, support-tool option/help, feature-line, learning-protocol, PCL statistics, term-DAG, TSM-output, autoschedule partial-match-output, scanner resolved-source-path, higher-order proof-rendering/type-declaration, formula input-marker, dummy-quote-collapse, AC-resolution parent-collapse asymmetry, proof-quote input-marker side effects, derived-PCL layout/formula-dialect, formula-to-clause normalization-order, pointer-tree traversal/destructive-merge ownership, free-variable definition-order, parser-probe/intrusive-term-store ownership, typed/higher-order clause-rendering allocation-order, demodulator-index coverage/lifecycle, selected-sort type-UID/allocator ordering, post-cache discrimination-tree query/cursor, indexed unit-subsumption side expansion, recursive clause-subsumption orientation backtracking, shared-variable live-PDTree rewrite, paramodulation normalization-order, unindexed-paramodulation derivation-target, and PDTree leaf pointer-order reviews also removed stale status claims and recorded C behaviors that should remain compatibility-visible. The WSL compatibility benchmark baseline is recorded in `docs/rust-port-status.md` so later ports cannot silently treat current performance gaps as complete. Continue applying the rule to newly ported code and to any stale `pending` or `remaining` status notes discovered during later reviews.

The 2026-07-17 KBO6 traversal follow-up applied this rule retroactively to C's local pointer-stack balance walkers and direct argument-array traversal. The paired `cto_kbolin` review records the ownership and push-order behavior that Rust now mirrors without constructing a temporary argument vector at every visited term.

The 2026-07-17 proofcheck-marker follow-up records an intentional correctness divergence from C's misuse of printf-escaped `COMCHAR` in a raw substring search. The paired `pcl_proofcheck` review documents why Rust recognizes real `% Proof found!` output while preserving C's fixed output-chunk and trace boundaries.

The 2026-07-17 DPLL reconciliation applied this rule to the reference's unfinished solver surface: `edpll` only allocates and frees state, its clause-update helpers are empty, and its declared retraction has no definition. The paired `cpr_dpll` review records that Rust's state shell is drop-in complete and that a real SAT solver would be an explicit post-compatibility extension.

The same 2026-07-17 propositional ownership audit reconciled `cpr_varset`, `cpr_propsig`, `cpr_propclauses`, and `cpr_dpllformula` against the exact 15-case `edpll` matrix. Their safe handles, owned strings, explicit bank, stable clause indices, and deterministic normalization are completed compatibility decisions rather than missing raw-pointer surfaces; the paired source reviews retain the C hazards and post-compatibility considerations.

The 2026-07-17 `ccl_propclauses` routing audit extends the permanent `eground` matrix with exact compact non-unit cases for LOP fallback, explicit TPTP, explicit TSTP, and auto-detected TSTP. This validates Rust's explicit format threading as the completed replacement for C `ClausePrint`'s process-global `OutputFormat` dependency.

The paired 2026-07-17 `ccl_grounding` audit records C's unconstrained `--give-up` bug: the constant count is assigned to `bool tmp`, so positive thresholds see `1^vars`; constrained grounding keeps a real estimate. Rust preserves both executable behaviors behind explicit outcomes, with exact archived-C cases for the inert and stopping branches.

The 2026-07-17 eground diagnostic audit closes the permanent 22-case executable matrix at zero mismatches. Rust now uses C's `<stdin>` scanner source, `stat`-before-open file diagnostics, `Opened`/`Closing` scanner lifecycle, and ordered recovery counts from the real formula-CNF term-bank sweeps; no diagnostic-only garbage collections are introduced.

The 2026-07-17 `cte_termvars` ownership audit closes the stale live-`TypeBank` concern. C uses the retained bank pointer only to fetch its immutable shared default type during untyped name allocation; typed sorts are explicit. Rust's retained shared default handle and dynamic type-UID maps preserve that behavior even when user sorts are inserted after VarBank construction.

The 2026-07-17 `cte_termpos` audit closes both `TermPosDebugPrint` branches. Rust has exact coverage for comment-prefixed hexadecimal identity output and for term-bank-backed `DEREF_NEVER...DEREF_ALWAYS` output, including C's higher-order behavior where `$let` remains an ordinary `@` application while FOOL and lambda surfaces use their conventional printers.

The 2026-07-17 `TermFree`/`TermTopFree` ownership audit confirms that Rust's reference-counted `Term` drop boundaries are the completed safe equivalent: unretained unshared descendants are released with their final root, VarBank variables remain bank-owned, and children retained elsewhere survive disposal of a temporary top wrapper. No manual-free API is needed.

The 2026-07-17 base `TermCell` reconciliation closes the raw flexible-array representation item as a measured Rust design decision. `Term` and `Option<Term>` stay one pointer wide, compact link storage has exact heap and proof-search evidence, and the separate LFHO owner-bank/binding-cache work remains explicitly tracked under its existing post-compatibility Beads.

The 2026-07-17 simple-type ownership audit confirms that `Type`/`Option<Type>` remain one pointer wide and that Rust `TypesCmp` uses actual `Rc` allocation addresses just as C uses `PCmp`. C explicitly documents allocator-dependent clause-sort differences, so exact address order and reuse are process-local in both implementations; shared identity remains stable for the TypeBank lifetime.

The 2026-07-17 subsumption integration follow-up routes proof control, contextual simplify-reflect, watchlists, and split-definition variant lookup through each `ClauseSet`'s owned FV anchor. Indexed insertion and extraction now define the production lookup lifecycle; explicit-anchor APIs remain only as lower-level test and interop surfaces. Simplify-reflect documentation remains explicit-session output with compact `DCSR` parents until separate stable-handle proof reconstruction work needs stronger identity.

The 2026-07-17 full PCL-step ownership audit confirms that Rust's discriminated logical-content enum, boxed clause arm, protocol-owned term-bank parameter, and explicit shell parse option preserve C's effective ownership and tool behavior without raw union or borrowed-bank hazards. Clause addresses remain stable when protocol vectors relocate steps, and the sole C shell-mode opt-in remains `epclextract` in Rust as well.

The 2026-07-17 full PCL-protocol audit confirms that a sorted owning step vector replaces C's raw-pointer tree and cached pointer stack without changing C-comparator lookup or serialized output. Duplicate errors keep membership counts truthful, comment forwarding uses `epclextract`'s explicit output owner, dangling parents are diagnostics, parent traversal is deterministic and deduplicated, and FOF stripping retains C's justification-only reset.

The 2026-07-17 PCL-position storage audit confirms that Rust's term-path vector structurally replaces C's nullable `PDArray` plus separate length: both avoid allocation until the first component, while Rust cannot represent a stale pointer/length pair. Exact multi-digit coverage retains C's intentionally tracked dotless printer shape.

The 2026-07-17 PCL-mini-step audit confirms that an owning logic enum and caller-supplied protocol bank replace C's untagged union and raw `TB_p` back-pointer without changing production parsing or printing. Executable shell modes are reproduced with call-scoped options, while numeric ids, narrow extras, zero-id parsing, and shell TSTP punctuation retain their tracked legacy behavior.

The 2026-07-17 PCL-mini-protocol audit confirms that owned optional step slots replace C's raw-pointer `PDArray` with constant-time lookup, amortized growth, non-allocating misses, and single-owner destruction. Duplicate collisions preserve the stored step, the maximum-id watermark and legacy printing/fast-marking rules remain exact, and explicit fast-mode comment forwarding plus deterministic id-based preconditions remove non-semantic global-output and pointer-order dependencies.

The 2026-07-13 contextual-simplify-reflect audit applied this rule retroactively to FV-index routing, indexed unit-query preconditions, and pointer-keyed FV-index leaf order; the detailed notes are in the paired `ccl_context_sr` and `ccl_subsumption` pages.

The 2026-07-13 indexed-paramodulation follow-up applied this rule retroactively to active-substitution lifetime and noncommutative metadata-parent ordering; the detailed notes are in the `cco_paramodulation` page.

The 2026-07-13 HEN011 throughput follow-up applied this rule retroactively to raw-parent HCB liveness, intrusive clause-set position lookup, per-call clause-subsumption scratch allocation, first-order matching job stacks, and raw term-argument-array access; the detailed notes are in the paired `che_hcb`, `ccl_clausesets`, `ccl_subsumption`, `cte_match_mgu_1-1`, and `cte_termfunc` pages.

The 2026-07-14 FV-index traversal follow-up applied this rule retroactively to the generic 64-pointer `PLocalStack` allocation used by each first-order match. The measured four-pair Rust inline capacity and the later C cleanup options are recorded in the paired `clb_plocalstacks` and `cte_match_mgu_1-1` pages.

The 2026-07-12 retroactive follow-up also reviewed C parent-liveness/archive coupling, object-tree payload ownership, and long-equation-list tautology search behavior under this rule.

The 2026-07-14 live-PDTree-substitution follow-up applied this rule retroactively to the already ported compact query and demodulator-index paths. The paired `ccl_pdtrees` review now records C's process-global traversal order, mutable cursor state and reusable traversal stack stored in every shared tree node, and raw-address leaf priority as later cleanup candidates while preserving the live-substitution performance contract.

The 2026-07-14 PDTree-query-reuse follow-up applied this rule retroactively to C's tree-owned reusable term traversal stack and callback. The paired `ccl_pdtrees` review records that allocation reuse is worth preserving, but later C should put the reusable query buffer and traversal continuation in an explicit search object rather than coupling them to a non-reentrant shared tree.

The 2026-07-14 iterative-PDTree-query follow-up applied this rule retroactively to C's reversible `TermLRTraverseNext`/`TermLRTraversePrev` pointer-stack API. The paired `ccl_pdtrees` review records its precedence-sensitive first-argument expression, assertion-only stack-shape contract, and shared-tree ownership as later cleanup candidates while retaining direct argument-array traversal performance.

The 2026-07-14 PDTree-query-metadata follow-up applied this rule retroactively to repeated higher-order term classification and root-weight evaluation in C's query, insertion, and search-initialization paths. The paired `ccl_pdtrees` review records a later one-pass classification boundary and invariant weight snapshot while retaining C's direct field/argument access and exact branch order.

The 2026-07-14 term-variable-traversal follow-up applied this rule retroactively to C's per-call generic `PStack` allocation in `TermCollectVariables`. The paired `cte_termfunc` review records caller-owned scratch or a small inline traversal stack as later cleanup options while retaining direct argument-array access, left-to-right pushes, and cached-ground pruning.

The 2026-07-14 substitution-normalization traversal follow-up applied this rule retroactively to C's unused `Sig_p` parameter and process-global dereference selection in `SubstNormTerm`. The paired `cte_subst` review records an explicit dereference-policy API as a later cleanup while retaining C's inline local stack, direct reversed argument pushes, and left-to-right binding order.

The 2026-07-14 term-top-comparator follow-up applied this rule retroactively to C's stale masked-properties key comment and process-local `uintptr_t` ordering in `TermTopCompare`. The paired `cte_termtrees` review records a corrected formal key contract and eventual stable-ID ordering as later cleanup candidates while preserving direct argument-array comparison and current allocation-sensitive behavior.

The 2026-07-14 PDTree-root-weight follow-up applied this rule retroactively to C's duplicated `TermStandardWeight` evaluation in `PDTreeSearchInit`. The paired `ccl_pdtrees` review records an invariant root-weight snapshot as a later C cleanup while preserving query normalization, assertion behavior, and the per-node size-constraint contract.

The 2026-07-14 PDTree-variable-metadata follow-up applied this rule retroactively to C's repeated direct reads of indexed-variable type and weight fields during backtracking search. The paired `ccl_pdtrees` review records the implicit shared-term immutability contract and when an explicit edge snapshot may make sense later, while retaining C's compact direct-field representation unless measurement justifies extra storage.

The 2026-07-14 PDTree-eta-normalization follow-up applied this rule retroactively to C's repeated eta dispatch in insertion, deletion, and search initialization. The paired `ccl_pdtrees` review records a normalized-key/index-handle boundary as a later cleanup while preserving the current classification order and compatibility-visible term-bank effects.

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
5. Track every newly discovered pending, remaining, or `Change Later` work item in Beads. When review shows a legacy status claim is stale because Rust already implements that surface, update the historical status evidence and close or update the corresponding Beads task in the same change.
6. For C-source pages, edit manual-review sections by hand when adding source-review knowledge.
7. Use generation only for source inventory and other mechanical updates.
8. Run the coverage, Change Later terminology, link, and regeneration-preservation checks.
9. Confirm the main worktree and the nested `eprover/` checkout are clean except for intended documentation changes.
10. Commit and push scoped documentation changes.
