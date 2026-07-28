# Documentation Index For Agents

Agent-made documentation belongs in this file and in the documentation locations linked from this file. Do not modify `AGENTS.md`; add or update agent-facing documentation here instead.

## Umlaut Rust Standards

Rust implementation work must follow [`docs/rust-code-standards.md`](docs/rust-code-standards.md), including clippy pedantic checks. Unsafe Rust is permitted for a concrete interoperability, compatibility, correctness, or measured performance reason when safe Rust cannot adequately meet the requirement; convenience alone is not sufficient. Keep unsafe implementation details narrowly scoped and behind safe APIs. Unsafe traits and their implementations are permitted, but every unsafe operation, function, trait, and implementation must document the applicable safety invariants and explain why Undefined Behavior cannot occur.

Umlaut is an independent theorem prover that began as an E port. E remains a
read-only compatibility, regression, provenance, and algorithmic reference,
not the product identity or a universal architecture and performance
authority. Umlaut retains E's substantive feature coverage and broadly
compatible interfaces, while its package and executable names intentionally
have no E aliases.

The completed E-to-Umlaut porting history and compatibility evidence are
recorded in [`docs/e-port-history.md`](docs/e-port-history.md). Its old names
and parity language are historical evidence, not active policy. Current work
is tracked canonically in Beads; strategic improvement research is organized
under `E_Rust_Port-9jt`. Use `bd ready`, `bd list`, and `bd search` to inspect
current status.

## Beads And Source Control

This repository explicitly opts into the Beads `team-maintainer` workflow.
Tracked Beads exports are project state and must be committed. Include them in
the same scoped commit as the work they describe; use a dedicated
`chore(beads): ...` commit only when the Beads update has no associated source
or documentation change. Keep automatic export enabled and automatic Git
staging disabled. Never commit ignored Dolt databases, locks, caches, or
temporary files. At successful session close, close completed Beads, run
quality gates, run `bd dolt push`, commit all intended tracked exports, push
Git, and verify both stores and the worktree are clean.

## VIRAS Clean-Room Research

An implementation-grade, paper-derived description of Virtual Integer-Real
Arithmetic Substitution is in
[`viras_docs/README.md`](viras_docs/README.md). The packet covers the base
quantifier-elimination calculus, its conflict-driven extension, a Rust-oriented
implementation blueprint, validation vectors, source provenance, and paper
errata. It was prepared without inspecting or using the unlicensed VIRAS
GitHub implementation.

## Third-Party Licenses

The licenses for the bundled CaDiCaL, E, GMP, MiniSat, Vampire, VIRAS, and Z3
projects are inventoried in
[`docs/third-party-licenses.md`](docs/third-party-licenses.md). Verbatim copies
of the available license notices are stored in [`licenses/`](licenses/).

The reviewed product boundary, source-derived implementation paths, optional
backend disablement rules, CASC source/runtime package split, and clean-package
audit are maintained in
[`docs/dependency-packaging-matrix.md`](docs/dependency-packaging-matrix.md).

## Pinned Vampire 5.0.1 Reference Build

The native Linux x86-64 Vampire 5.0.1 reference executable built on the
ephemeral Ubuntu 24.04 Linode is stored locally at
`.artifacts/vampire/3677326861181f990ce3ef461e90471ba9749225/linode-ubuntu24.04-x86_64/vampire`.
Its SHA-256 is
`3fd88f402d2b74ddf6bf96d49a2bf3c9383710b19d1c9c2c5ecb740265a5c665`;
the adjacent ignored `vampire.sha256` sidecar contains the same value. The
existing `.artifacts/` rule in `.gitignore` covers both files, as confirmed
with `git check-ignore -v`.

For every routine Vampire invocation on a Linode, treat this archived
executable as canonical. After provisioning the runner, transfer the executable
explicitly from the path above with the active runner's dedicated SSH identity;
the normal source `sync` excludes `.artifacts/`. Verify that the remote
SHA-256 equals the value above, grant executable permission, and run the
transferred file. Do not clone or recompile Vampire, CaDiCaL, VIRAS, or Z3
merely to run Vampire. Rebuild only when a task explicitly replaces the pinned
artifact with a newly verified and documented revision.

The 2026-07-27 build used Vampire revision
`3677326861181f990ce3ef461e90471ba9749225`, CaDiCaL revision
`f13d74439a5b5c963ac5b02d05ce93a8098018b8`, VIRAS revision
`8b8928f57f8d6415662cf43289de2c0d36443240`, and Z3 revision
`3c47fd96cf5645d0c42b2c819d9e9a84380aa721`. Z3 was configured with
`CMAKE_BUILD_TYPE=Release`, `Z3_BUILD_EXECUTABLE=OFF`,
`Z3_BUILD_TEST_EXECUTABLES=OFF`, and `Z3_BUILD_LIBZ3_SHARED=OFF`. Vampire was
configured with `CMAKE_BUILD_TYPE=Release`, `CCACHE_PROGRAM=OFF`, and
`Z3_DIR` pointing to that static Z3 build; both build stages used four
parallel jobs.

The authoritative artifact came from normal-profile runner
`e-rust-codex-260727-233253-f75d` (run ID `260727-233253-f75d`, Linode ID
`101559950`) using Ubuntu 24.04, GCC 13.3.0, and CMake 3.28.3. It is an
unstripped x86-64 GNU/Linux ELF PIE. Its dynamic dependencies are only the
standard GNU/Linux loader, `libstdc++`, `libm`, `libgcc_s`, and `libc`;
`libz3` is absent because Z3 is linked statically. The binary reports Vampire
5.0.1 at the pinned commit, CaDiCaL 2.1.3, and Z3 4.14.0.0 at the pinned
revision. It passed the complete upstream `checks/sanity` suite on the
Linode. After download, its hash matched the remote value and it ran under
WSL Ubuntu 24.04, where it proved `checks/Problems/PUZ/PUZ001+1.p` with SZS
status `Theorem`.

The independent WSL build is retained in the local cache at
`/home/rober/.cache/e-rust-port/vampire/3677326861181f990ce3ef461e90471ba9749225/vampire-build/vampire`
with SHA-256
`84a3cc9fe13295c6ef73bb62a75d9a32b9aa3a1316500a49c126277405070184`.
It used the same revisions and build flags and passed the complete upstream
sanity suite through a disposable LF-normalized harness, leaving the Windows
checkout untouched. This WSL build and validation were a user-authorized
one-time exception for this reference artifact; they do not change the
execution policy below.

The pinned VIRAS revision does not declare a license. Consequently, these
Vampire executables are ignored, local-only reference artifacts and must not
be committed, published, or redistributed.

## Execution And Platform Policy

All Rust and C formatting, compilation, tests, execution, compatibility
comparisons, benchmarks, and profiling must run on the ephemeral Ubuntu 24.04
Linode. Do not invoke Cargo, `rustc`, Rust project binaries, the C build, C
binaries, WSL, Valgrind, or Callgrind on the local computer. Local work is
limited to editing, orchestration, Git operations, documentation validation,
PowerShell parsing, and Python controller/compatibility unit tests.

This is a command-routing rule, not a preference:

| Work | Required environment |
| --- | --- |
| Edit files; use Git or Beads; inspect documentation; parse PowerShell; run Markdown checks or Python controller/compatibility unit tests | Local computer is allowed |
| Provision, synchronize, execute, collect artifacts, and tear down through `linode-runner.ps1` | Local PowerShell controller |
| Run Rustfmt, Cargo, `rustc`, Clippy, Rust builds/tests, or any Rust project binary | Linode only |
| Run `configure`, Make, GCC, any C build, or any C reference/support binary | Linode only |
| Run Rust/C comparisons, smoke tests, benchmarks, Valgrind, or Callgrind | Linode only |
| Cross-compile `x86_64-pc-windows-gnu` binaries and tests | Linode only; compile but never execute |

There are no local smoke-test exceptions. WSL, local containers, virtual
machines, and locally installed toolchains are not substitutes for the Linode.
If a command formats, compiles, links, tests, starts, benchmarks, or profiles
Umlaut or the upstream C prover, route it through the Linode controller.

Native Linux is the runtime, behavioral-compatibility, and performance
authority. Umlaut must also compile for `x86_64-pc-windows-gnu` on the
Linode, but Windows is compile-only: do not execute Windows binaries or claim
Windows runtime, behavioral, performance, or MSVC compatibility. Historical
Windows/WSL commands in completed experiments and status records are evidence
of earlier work, not current instructions.

## Ephemeral Linode Compute

Use the single required workflow in
[`docs/linode-runner.md`](docs/linode-runner.md). It provisions a short-lived
runner, synchronizes the exact current worktree without depending on a pushed
branch, runs every Linux Rust/C quality and parity check, cross-compiles
Windows GNU x64 without executing it, collects timing and Callgrind evidence,
and guarantees guarded teardown.

The normal default is the 8 GiB `g8-dedicated-8-4` profile at $0.14 an hour.
Use `.\linode-runner.ps1 run` for normal validation. The explicit
`--high-memory` option selects the 150 GB `g7-highmem-8` profile at $0.74 an hour
when a task should more closely resemble the CASC configuration. For a closer
CASC match, every actual prover command on that host should include
`--memory-limit=131072`, which is the prover's MB value for 128 GiB.

High-memory usage has a four-hour daily base allowance and a bank capped at four
hours. The bank starts full before the earliest trusted run, unused daily time
refills it, usage above the base consumes it, and overshoot beyond the bank
becomes uncapped debt that reduces later capacity. New high-memory `up` and
`run` starts are forbidden once actual usage reaches the base allowance adjusted
by the bank or debt at the start of the fixed-EST day. Fixed EST means UTC-05:00
year-round, without daylight-saving time; the controller uses Linode-controlled
timestamps rather than the local Windows clock. Run
`.\linode-runner.ps1 check --high-memory` to see the base allowance, starting
bank or debt, adjusted capacity, actual and remaining usage, projected next
balance, next accounting boundary, and projected eligibility when blocked. Use
the guarded `up`/`sync`/`exec`/`down` lifecycle documented in the runbook only
when an exceptional task needs individual remote commands.

## Runtime PicoSAT Selection

Umlaut selects a runtime-loaded PicoSAT backend when the legacy configuration
variable `E_RUST_PORT_PICOSAT_LIBRARY` names a PicoSAT DLL/shared-library path.
When it is unset or empty, Umlaut also looks for a bundled PicoSAT library next
to `umlaut`, under `lib/` next to the executable, and under `../lib/` relative
to the executable directory. If no library is found, Umlaut falls back to the
internal solver. The user-configured variables `E_RUST_PORT_COMPAT_ROOT`,
`E_RUST_PORT_COMPAT_ARTIFACT_ROOT`, and `E_RUST_PORT_PICOSAT_LIBRARY` retain
their names for operational compatibility.

## Search Telemetry

The opt-in, versioned JSON search telemetry contract, schedule-worker file
naming, failure boundaries, and measured overhead budget are documented in
[`docs/search-telemetry.md`](docs/search-telemetry.md). The corresponding
reproducible benchmark and diagnosis evidence lives under
[`experiments/2026-07-27-002-search-telemetry/`](experiments/2026-07-27-002-search-telemetry/).

## Memory Representation Profile

The current term, clause, index, garbage-collection, RSS, and cache-locality
profile is recorded in
[`experiments/2026-07-27-003-memory-representation-profile/`](experiments/2026-07-27-003-memory-representation-profile/).
It attributes retained memory across solved and bounded searches, records the
negative general-layout decision, and carries the measured rewrite-derivation
target into the dedicated proof-trace Bead.

## Soundness Validation Gates

The positive-only SZS status, proof, and model validation policy is documented
in [`docs/soundness-validation.md`](docs/soundness-validation.md). The
controller, adversarial tests, pinned independent-checker evidence, and
machine-visible coverage gaps are recorded in
[`experiments/2026-07-27-004-soundness-validation-gates/`](experiments/2026-07-27-004-soundness-validation-gates/).
The follow-up checker bake-off, explicit typed conjecture-negation provenance,
and independent `ContradictoryAxioms` acceptance are recorded in
[`experiments/2026-07-28-001-proof-checker-coverage/`](experiments/2026-07-28-001-proof-checker-coverage/).

## C Source Documentation

The original C implementation in `eprover/` is documented under [`docs/c_source_docs/`](docs/c_source_docs/). Treat `eprover/` as read-only original source; update documentation around it, not the source itself.

### C Change Later Notes

When using E as a reference or reviewing E-derived behavior, document details
that Umlaut should preserve, reject, or improve. This includes soundness and
compatibility contracts, accidental behavior, portability hazards, obsolete
allocation patterns, global-state quirks, confusing API boundaries, ignored
parameters, counter overflows, and performance tradeoffs. Put these notes in
the relevant C-source page's manual-review `Change Later` section, or in a
linked status/design document when the issue spans multiple source units. The
review text remains the technical source-analysis record, while active task
state is canonical in Beads. Every new top-level `Change Later` item must also
create or update a Beads task labeled `source-c-review-change-later`, with
source-file and content-hash metadata.

Retroactive audit status as of 2026-07-11: the existing C-source manual-review pages have been checked against this rule with `check_change_later_notes.py`, `generate_c_source_docs.py --check`, `check_regeneration_preserves_manual.py`, and `check_markdown_links.py`; later indexed-paramodulation, higher-order-dispatch, proof-state global-index ownership, typed `PStack` allocation, derivation-stack memory, term-bank release-assertion, shared term-argument, intrusive term-tree, shared help-footer, support-tool option/help, feature-line, learning-protocol, PCL statistics, term-DAG, TSM-output, autoschedule partial-match-output, scanner resolved-source-path, higher-order proof-rendering/type-declaration, formula input-marker, dummy-quote-collapse, AC-resolution parent-collapse asymmetry, proof-quote input-marker side effects, derived-PCL layout/formula-dialect, formula-to-clause normalization-order, pointer-tree traversal/destructive-merge ownership, free-variable definition-order, parser-probe/intrusive-term-store ownership, typed/higher-order clause-rendering allocation-order, demodulator-index coverage/lifecycle, selected-sort type-UID/allocator ordering, post-cache discrimination-tree query/cursor, indexed unit-subsumption side expansion, recursive clause-subsumption orientation backtracking, shared-variable live-PDTree rewrite, paramodulation normalization-order, unindexed-paramodulation derivation-target, and PDTree leaf pointer-order reviews also removed stale status claims and recorded C behaviors that should remain compatibility-visible. The earlier WSL compatibility benchmark baseline is retained in `docs/e-port-history.md` as historical evidence. Continue applying the rule when E-derived behavior is reviewed and when stale historical status claims are discovered.

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

The 2026-07-17 term-bank ownership follow-up makes `TermBank` and its intrusive `TermCellStore` non-cloneable. Detached parser probes keep independent stores, while proof-object formula rendering borrows canonical handles through immutable print-only literal and clause views; a regression verifies rendering cannot change bank counters, term properties, or canonical lookup identity.

The paired 2026-07-17 low-level parser reconciliation confirms that the `FuncSymbParse`/`TermParseOperator`/unshared and banked term parser stack, TPTP/TSTP/TCF formula entry points, higher-order application-term entry point, `$distinct`, storage accounting, represented formula owners, and nested include scanners are all routed and tested. Remaining FOOL atom and checked-versus-simple grammar cleanup stays in the narrower post-compatibility parser reviews rather than a broad missing-surface claim. The `ccl_tformulae` review also records Rust's deliberate immutable-rendering divergence from C `EqnAlloc` predicate side effects.

The 2026-07-17 proof-pipeline ownership reconciliation retires the migrated all-in-one saturation status item. Formula-owner/CNF breadth remains with the dedicated executable formula items, scheduler state and accounting remain with the auto-schedule items and `cco_scheduling` reviews, and higher-order completeness remains with the higher-order control/inference items. Ordered mixed clause/formula proof extraction and display renumbering are already complete under the derivation-metadata work; focused formula-CNF, proof-list, schedule, and higher-order executable regressions confirm those boundaries.

The 2026-07-17 `eprover` help-parity slice upgrades the production option table from metadata-only compatibility to C declaration order and exact `e_options.h` help prose, including the reference wording and typos. A checked generator in the experiment directory keeps the large table synchronized without embedding or parsing vendored C at runtime, while Rust tests independently compare order, aliases, argument/default metadata, and concatenated C descriptions against the unchanged source header.

The 2026-07-17 random-weight integration audit closes the stale initial-heuristic gap with a live proof-control regression. A named `RandomWeight` now has exact reference coverage from the production WFCB/HCB definition stack through generated-clause evaluation, evaluation-index ordering, and active-HCB selection. The regression deliberately supplies nonzero evaluator seeds yet pins the first two global JKISS-derived C `float` weights, preserving the C wrapper's state-pointer quirk; the cleaned RNG decision remains in the existing post-compatibility Beads.

The 2026-07-17 temporary-file ownership audit confirms that Rust preserves C's content-keyed process-global registry, atomic empty-file creation, source copying, explicit unlink-before-unregister ordering, failed-cleanup warning behavior, and SIGTERM/SIGINT cleanup boundary without retaining a raw `mkstemp` descriptor. `TMPDIR` remains authoritative; the no-variable default is `/tmp` on Unix-like targets and the native temporary directory on Windows so scheduled standard-input replay works outside an MSYS filesystem. Exact libc suffix selection, NUL-containing native paths, and eventual scoped run-state ownership remain in the existing post-compatibility Beads.

The 2026-07-17 signal-delivery reconciliation confirms the completed cross-platform boundary. Retained WSL-native logs created after the Linux trampoline change show actual `SIGXCPU` delivery with C-shaped direct `ResourceOut` bytes, fatal diagnostic, and exit status 8; the current same-tree comparison independently matches actual Linux C expiry to cooperative Windows Rust expiry on two 60-second cases. Normal Linux SIGTERM/SIGINT delivery is not overclaimed without a live-injection artifact: its cleanup-once, default-reset/re-raise, and scheduler-latch behavior remain source-aligned and deterministically modelled, while the async-signal-safe redesign stays in the existing post-compatibility review.

The 2026-07-17 network-socket reconciliation closes the safe `TcpListener`/`TcpStream` boundary across `cio_network`, `e_client`, `e_server`, `e_deduction_server`, and the reusable server/session owners. Linux and Windows retain explicit `SO_REUSEADDR`, bind, and backlog-10 setup before ownership transfer; real descriptors remain visible where C uses them. Server and connect failures now keep C's two-line system-error shape, and Unix resolver diagnostics remove only Rust's wrapper around the same `gai_strerror` detail. Rust deliberately closes C's leaked earlier-success/error descriptors; those lifetime and other-platform reuse questions remain in the existing post-compatibility Beads.

The 2026-07-17 TCP-channel ownership reconciliation closes the adjacent `cio_multiplexer` lifetime gap. Exact drop-count regressions prove explicit close, open-channel drop, and `into_inner` transfer each release the stream at the correct boundary; a real loopback test proves stale-session close releases the socket and removes descriptor interest. The reusable session path also retains C's exact verbosity-gated descriptor notice while deliberately preserving C's lack of any `close(2)` failure diagnostic.

The 2026-07-17 global-output-descriptor reconciliation validates the low-level `cio_output`/`clb_defines` bridge functionally on native Windows: the UCRT descriptor and owned Rust file target write in order to the same file, while a duplicated handle gives each owner an independent close boundary. Unix keeps the owned file's native descriptor, stdout remains descriptor 1, and other target ABIs retain an explicit failing sentinel rather than misrepresenting an OS handle as a C-runtime descriptor.

The 2026-07-17 deduction-server `RUN` framing follow-up replaces the earlier source-only decision with two live WSL TCP captures. A deterministic prover fixture confirms the intended start/proof/finish/success messages and exact network-order lengths, and a real-loopback Rust regression pins those bytes through the executable client wrapper. The stock C pairing also exposes a printf-escaped `COMCHAR` bug: default `eprover` emits `% Pid:` while `ECtrlCreateGeneric` searches for `%% Pid:`, so the child aborts after the start frame and its parent still sends success. Rust deliberately keeps the working intended feature; the C repair remains tracked in the existing process-control Change Later Bead.

The 2026-07-17 permanent-string ownership audit confirms that C's registry is a lifetime device, not a production identity protocol. Its only five parameter-parser callers retain scanner-derived configuration text across shallow struct copies and consume it by content. Rust's owned `String` fields and clones preserve that lifetime without global registry coupling, while the standalone `Arc<str>` registry keeps exact live-epoch duplicate identity and safe explicit clearing for compatibility callers. Raw-pointer invalidation, input-allocation identity, and C splay locality remain in their existing post-compatibility Beads.

The 2026-07-17 simple-type ownership audit confirms that `Type`/`Option<Type>` remain one pointer wide and that Rust `TypesCmp` uses actual `Rc` allocation addresses just as C uses `PCmp`. C explicitly documents allocator-dependent clause-sort differences, so exact address order and reuse are process-local in both implementations; shared identity remains stable for the TypeBank lifetime.

The 2026-07-17 subsumption integration follow-up routes proof control, contextual simplify-reflect, watchlists, and split-definition variant lookup through each `ClauseSet`'s owned FV anchor. Indexed insertion and extraction now define the production lookup lifecycle; explicit-anchor APIs remain only as lower-level test and interop surfaces. Simplify-reflect documentation remains explicit-session output with compact `DCSR` parents until separate stable-handle proof reconstruction work needs stronger identity.

The 2026-07-17 full PCL-step ownership audit confirms that Rust's discriminated logical-content enum, boxed clause arm, protocol-owned term-bank parameter, and explicit shell parse option preserve C's effective ownership and tool behavior without raw union or borrowed-bank hazards. Clause addresses remain stable when protocol vectors relocate steps, and the sole C shell-mode opt-in remains `epclextract` in Rust as well.

The 2026-07-17 full PCL-protocol audit confirms that a sorted owning step vector replaces C's raw-pointer tree and cached pointer stack without changing C-comparator lookup or serialized output. Duplicate errors keep membership counts truthful, comment forwarding uses `epclextract`'s explicit output owner, dangling parents are diagnostics, parent traversal is deterministic and deduplicated, and FOF stripping retains C's justification-only reset.

The 2026-07-17 PCL-position storage audit confirms that Rust's term-path vector structurally replaces C's nullable `PDArray` plus separate length: both avoid allocation until the first component, while Rust cannot represent a stale pointer/length pair. Exact multi-digit coverage retains C's intentionally tracked dotless printer shape.

The 2026-07-17 PCL-mini-step audit confirms that an owning logic enum and caller-supplied protocol bank replace C's untagged union and raw `TB_p` back-pointer without changing production parsing or printing. Executable shell modes are reproduced with call-scoped options, while numeric ids, narrow extras, zero-id parsing, and shell TSTP punctuation retain their tracked legacy behavior.

The 2026-07-17 PCL-mini-protocol audit confirms that owned optional step slots replace C's raw-pointer `PDArray` with constant-time lookup, amortized growth, non-allocating misses, and single-owner destruction. Duplicate collisions preserve the stored step, the maximum-id watermark and legacy printing/fast-marking rules remain exact, and explicit fast-mode comment forwarding plus deterministic id-based preconditions remove non-semantic global-output and pointer-order dependencies.

The 2026-07-17 PCL-mini-clause audit confirms that one owning literal vector replaces C's separate sign and borrowed-term arrays while preserving shared term identity. Rust retains full counts beyond C's invalid signed-short boundary, deliberately drops clause metadata on reconstruction, and uses C's temporary-clause print path with explicit call-local output controls.

The 2026-07-17 PCL-identifier audit confirms that vector length safely replaces C's `-1` terminator because live decimal components include zero but cannot be negative. Parsing/printing scale geometrically to long identifiers, while protocol comparison still injects the sentinel and preserves C's subtraction-to-`int` truncation—even its wide-component equality collapse—until the separately tracked strict-order cleanup.

The 2026-07-17 PCL-expression audit confirms that typed recursive ownership replaces C's untagged two-slot-per-argument array and separate full/mini destruction paths. Exhaustive coverage pins every opcode and parser/printer spelling, a 2,048-parent case validates amortized argument growth, and the tracked position-parser mismatch, TSTP position omission, absent `URewrite` syntax, and one-or-more variable arities remain exact.

The 2026-07-17 frequency-vector ownership audit closes the old raw-clause-alias blocker by separating ordinary vector snapshots from a non-cloneable packed clause owner. Unpacking is an ownership transfer, compatibility printing borrows the live clause explicitly, source-clause destruction cannot dangle vector metadata, and FV-index insertion no longer clones the feature vector.

The 2026-07-17 FV-index fidelity audit closes the deferred storage and output-routing gap. Rust reproduces C's signed `IntMap` representation-transition deltas, insertion-only `FVIndexStorage` counter, combined same-stream tree output, and exact distinct `out`/`stderr` fragments for LOP/TPTP/TSTP rendering without introducing global output state.

The 2026-07-17 SInE ownership-integration audit closes the parser/formula-owner gap. Executable CNF and formula records retain stable `WrappedFormula` ownership through SInE, selected mixed proof-state owners move by stable ids without cloning, duplicate selections retain C's relinking behavior, and CNF drains only the selected wrappers afterward.

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

- [`docs/c_source_docs/overview.md`](docs/c_source_docs/overview.md) - subsystem map, coverage counts, E-reference guidance, and links to every source-unit page.
- [`docs/c_source_docs/review_status.md`](docs/c_source_docs/review_status.md) - review table for all documented C source units.
- Per-subsystem directories such as [`BASICS`](docs/c_source_docs/BASICS/), [`TERMS`](docs/c_source_docs/TERMS/), [`CLAUSES`](docs/c_source_docs/CLAUSES/), [`CONTROL`](docs/c_source_docs/CONTROL/), and [`HEURISTICS`](docs/c_source_docs/HEURISTICS/) contain the individual source-unit pages.

Current C-source documentation coverage:

- 492 original `.c`/`.h` files covered.
- 266 source-unit pages: `.c`/`.h` pairs are documented together; standalone `.c` or `.h` files get their own page.
- 268 Markdown files total under `docs/c_source_docs/`, including `overview.md` and `review_status.md`.

Each C-source documentation page has two protected regions:

- `<!-- BEGIN AUTO-GENERATED: c_source_docs -->` to `<!-- END AUTO-GENERATED: c_source_docs -->` contains mechanical inventory generated from the source tree.
- `<!-- BEGIN MANUAL REVIEW: c_source_docs -->` to `<!-- END MANUAL REVIEW: c_source_docs -->` contains manually reviewed notes and compatibility judgments.

Regeneration must not destroy manual documentation. Generated tooling may
replace only the auto-generated region. Put hand-written source review,
caveats, compatibility judgments, and improvement observations in the
manual-review region or in separate docs linked from this file. Existing dated
manual regions are historical source analysis and may retain porting-era
terminology.

## C Source Documentation Tooling

Use the repo-local virtual environment:

```powershell
.\.venv\Scripts\python.exe tools\c_source_docs\generate_c_source_docs.py --check
.\.venv\Scripts\python.exe tools\c_source_docs\generate_c_source_docs.py --generate
.\.venv\Scripts\python.exe tools\c_source_docs\check_change_later_notes.py
.\.venv\Scripts\python.exe tools\c_source_docs\check_markdown_links.py
.\.venv\Scripts\python.exe tools\c_source_docs\check_regeneration_preserves_manual.py
```

Command roles:

- `generate_c_source_docs.py --check` verifies every C/H file under `eprover/` maps to exactly one documented source unit.
- `generate_c_source_docs.py --generate` refreshes mechanical inventory sections while preserving manual-review sections.
- `apply_manual_review_notes.py` is a historical/bootstrap helper for replacing preserved manual-review sections. It is not part of normal regeneration and must be run only when intentionally revising those sections.
- `check_change_later_notes.py` verifies C-source review docs use the standard `Change Later` section wording and do not reintroduce legacy candidate/observation headings.
- `check_markdown_links.py` checks local Markdown links in the C-source docs and this `DOCS.md` file.
- `check_regeneration_preserves_manual.py` regenerates docs and confirms manual-review sections are unchanged.

## Maintenance Workflow

1. Run `git status --short` before changing documentation.
2. Do not modify `eprover/`.
3. Add new agent-facing documentation to `DOCS.md` or to a linked docs location.
4. When using E as a reference, document which contracts Umlaut must preserve
   and which implementation choices or defects should be improved, including
   portability hazards, obsolete allocation patterns, global-state quirks,
   soundness concerns, and performance tradeoffs.
5. Track every newly discovered pending, remaining, or `Change Later` work item in Beads. When review shows a legacy status claim is stale because Rust already implements that surface, update the historical status evidence and close or update the corresponding Beads task in the same change.
6. For C-source pages, edit manual-review sections by hand when adding source-review knowledge.
7. Use generation only for source inventory and other mechanical updates.
8. Run the coverage, Change Later terminology, link, and regeneration-preservation checks.
9. Confirm the main worktree and the nested `eprover/` checkout are clean except for intended documentation changes.
10. Commit and push scoped documentation and tracked Beads-export changes,
    then run `bd dolt push`.
