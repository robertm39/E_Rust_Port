# Mixed-problem VIRAS preprocessing preregistration

Bead: `E_Rust_Port-9jt.5.12`

## Question

Can Umlaut conservatively extract eligible closed typed arithmetic formulas
from mixed TPTP problems, replace them only after native checked VIRAS
elimination, preserve their source ancestry in proof output, and improve
saturation-search results without changing any automatic schedule?

## Frozen implementation boundary

The production path is compiled only by the existing `viras-qe` feature and
is enabled only by the explicit `--viras-qe-preprocess` option. Default
features, ordinary invocations, automatic schedules, and the standalone
`umlaut-viras-qe` interface remain unchanged.

After ordinary parsing and source-formula proof publication, each active
formula is considered independently. A formula is eligible only when it is a
closed typed formula containing at least one quantifier and the unchanged
typed LIRA importer accepts it. Successful elimination must:

1. return a closed quantifier-free formula;
2. carry replay-validated branch derivations;
3. pass a fresh formula-level native recomputation that compares the source,
   result, resource-bounded derivation, and complete branch proof;
4. survive canonical TFF rendering, parsing back into Umlaut's typed term
   bank, and exact re-import comparison; and
5. be published as a named `viras_qe` formula inference whose parent is the
   archived source formula.

An import rejection, kernel Unknown, proof-check failure, or re-embedding
failure leaves that formula byte-for-semantics unchanged. It may be reported
as skipped or Unknown but may not partly replace the problem. A native
checker failure is a hard fail-closed prover error, not permission to insert
an unchecked result.

No arithmetic syntax, resource default, schedule arm, held-out family, or
acceptance threshold may be tuned after observing the held-out report.
Implementation defects require a fresh report and explicit disclosure. The
work may use only the tracked clean-room `viras_docs/` packet, experiments
023/004/005, and the existing audited Rust implementation. It must not
inspect, import, build, execute, or derive tests from the unlicensed VIRAS
source.

## Held-out surface

Evaluate all 100 untouched files in `problems/casc_2025/TFI`. Group metrics by
the ten filename-prefix families present in the corpus: `ARI`, `CSR`, `DAT`,
`HWV`, `ITP`, `NUM`, `SEV`, `SWC`, `SWW`, and `SYO`. These documents were
held out from the mixed-problem implementation; there is no train or tuning
partition, so no family can leak into implementation selection.

Run baseline and opt-in preprocessing with the same release all-feature
`umlaut` binary, strategy, one-second CPU limit, 2 GiB memory limit, and proof
settings. Use status comments when present only as metadata; the primary
solve delta is the exact SZS outcome comparison between arms. Report
baseline-only, opt-in-only, common, and changed-status files explicitly.

## Frozen gates

The bead passes only if:

1. unit and integration tests cover mixed eligible/ineligible formulas,
   source-ancestry-preserving TSTP output, no-quantifier pass-through,
   importer rejection, kernel resource Unknown, and deterministic output;
2. a public native formula-proof checker accepts authentic output and rejects
   at least source, result, replay-flag, and branch-candidate corruptions;
3. every production replacement is checked before insertion and a deliberate
   checker mutation prevents insertion;
4. all unsupported and resource-Unknown formulas remain unchanged, including
   their formula identity, role, properties, and derivation ancestry;
5. the 100-file held-out controller completes both arms and reports per-family
   document/formula coverage, applied transformations, import skips, kernel
   Unknowns, proof-check success, source/result node counts, growth ratio,
   median/p95/max whole-process latency, and solve delta;
6. every reported applied transformation has proof-check success, no
   quantifier in its result, and a TSTP `viras_qe` parent inference;
7. repeated opt-in runs on at least one transformed and one pass-through
   document are byte-identical after excluding ordinary timing/statistics
   lines;
8. ordinary invocations have identical behavior to the pre-change baseline
   compatibility suite, the default feature list stays empty, and the
   default package contains no VIRAS dependency graph or feature-only option
   surface;
9. Linux all-target/all-feature tests, strict Clippy, formatting,
   Windows-GNU all-target/all-feature compile-only, package audit, and the
   repository's comprehensive runner pass; and
10. `src/heuristics/schedule.vars` retains its preregistered SHA-256 and no
    automatic schedule or strategy definition invokes the new option.

The preregistered schedule hash is
`491145ab45477620ed02ed8cd789d6b5e3e6e0d38f413fdbc62163e09a9cb068`.

## Metrics and interpretation

Coverage is measured at both document and active formula level. A document is
covered when at least one formula is replaced; an unsupported formula is not
counted as a failed proof because it never enters publication. Proof success
is checked replacements divided by attempted successful eliminations and
must be 100%.

Formula growth is canonical LIRA result node count divided by canonical LIRA
source node count. Latency is whole-process elapsed time. Report both
absolute opt-in latency and paired opt-in/baseline ratio without imposing an
efficacy cutoff. An opt-in-only SZS success is evidence of complementarity,
not sufficient evidence for schedule adoption.

## Stop rules

Stop without closing the bead if a changed formula lacks a validated native
proof, any corruption is accepted, unsupported or Unknown input is changed,
source ancestry is missing, a result retains a quantifier, the optional graph
leaks into the default package, the schedule hash changes, or any
comprehensive compatibility gate fails. Zero solve improvement or low
coverage is a reportable negative utility result, not a soundness exception
and not permission to enable a schedule.
