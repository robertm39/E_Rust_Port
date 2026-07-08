<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CLAUSES / ccl_eqnresolution

## Source Files

- [CLAUSES/ccl_eqnresolution.h](../../../eprover/CLAUSES/ccl_eqnresolution.h)
- [CLAUSES/ccl_eqnresolution.c](../../../eprover/CLAUSES/ccl_eqnresolution.c)

## Purpose

Routines for performing (ordered) equality resolution. the GNU Lesser General Public License. <1> Fri Jun 5 18:36:46 MET DST 1998 New

Within the source tree, this unit belongs to `CLAUSES`. Clause, formula, literal, inference, simplification, indexing, subsumption, grounding, relevance, and proof-state machinery.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- None found in the source scan.

### Macros And Constants

- `CCL_EQNRESOLUTION`

### Globals

- `extern bool EqResOnMaximalLiteralsOnly`

### Exported Functions

- `Clause_p ComputeEqRes(TB_p bank, ClausePos_p pos, VarBank_p freshvars, bool* subst_is_ho, PStack_p res_cls)`
- `Eqn_p ClausePosFirstEqResLiteral(Clause_p clause, ClausePos_p pos)`
- `Eqn_p ClausePosNextEqResLiteral(ClausePos_p pos)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `build_resolvent`: Actually builds eq resolvent
- `ComputeEqRes`: Given a clause and a position, try to perform equality resolution and return the resulting clause. If res_cls is NULL, then it assumes that you want to enumerate only single clause which is returned! Else, it returns NULL but fills res_cls with all clauses
- `ClausePosFirstEqResLiteral`: Find the first negative maximal literal in clause and return it.
- `ClausePosNextEqResLiteral`: Find the next negative maximal literal in clause and return it.

### Dependencies

- `"ccl_eqnresolution.h"`
- `<ccl_clausesets.h>`
- `<cte_ho_csu.h>`

### Compile-Time Conditions

- `CCL_EQNRESOLUTION`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Clause/literal mutation affects indexing, derivation, and proof reconstruction; preserve update ordering.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `CLAUSES/ccl_eqnresolution.h`, `CLAUSES/ccl_eqnresolution.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `CLAUSES` covering 2 source file(s), about 258 lines, 4 scanned public declarations, 0 scanned internal function definitions, and 4 structured function-comment blocks.
- Routines for performing (ordered) equality resolution. the GNU Lesser General Public License. <1> Fri Jun 5 18:36:46 MET DST 1998 New
- Clause/formula code. Pay close attention to mutation ordering because indexes, derivation records, and proof-state sets are updated around the same objects.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Rust Port Status Notes

- Rust now ports the single-result `ComputeEqRes` MGU path used by destructive equality resolution, including higher-order problem-mode arrow-variable bindings and `SubstHasHOBinding` propagation to higher-order `DCDesEqRes`.
- Rust now ports `ComputeAllEqnResolvents` generation over the higher-order CSU iterator in higher-order mode, including C-shaped non-selected literal substitution normalization, optimized copying except the resolved literal, `EqnListLambdaNormalize` before false-literal and duplicate cleanup, negative-literal iteration with an explicit maximal-literal filter, C stack-pop insertion order, aggregate `subst_is_ho` propagation to higher-order `DCEqRes`, proof-state-owned `freshvars` reuse for proof-control generation, and insertion into a caller-owned clause set.
- The all-resolvent wrapper and destructive variable-normalization wrapper expose opt-in proof-documentation output for represented all-resolvent creation and destructive-replacement modification steps.
- Proof-control destructive equality-resolution normalization now also routes the proof-state-owned `freshvars` bank through the helper path. Broader C trace coverage for multi-CSU equality-resolution order/performance remains pending.

### Change Later

- `build_resolvent` uses the caller-provided `freshvars` bank to normalize unbound variables before copying the resolvent. Rust's proof-control paths now pass the proof-state-owned paired `freshvars` bank and reset variable counts at the C `ComputeEqRes` boundary; standalone helper entry points still use a scratch fresh-normalization bank so isolated callers do not need a proof-state owner. Keep that split visible until all inference callers have stable proof-session ownership.
- `build_resolvent` normalizes copied resolvent literals before removing false and duplicate literals, so DB-lambda beta/eta reduction can affect which literals are cleaned up and can trigger `EqnMap` truth/polarity side effects. Rust preserves that ordering explicitly through `EqnList::lambda_normalize`.
- `EqResOnMaximalLiteralsOnly` is a mutable C global controlling the public literal iterators. Rust exposes the default-filter behavior as an explicit boolean argument for now; revisit the API once option/global-state ownership is centralized.
- C `ComputeEqRes` returns either one clause or fills a result stack depending on whether `res_cls` is NULL. Rust separates these into single-resolvent and all-resolvent helpers so callers do not depend on a null-stack mode switch.
- In the higher-order path, C pushes each CSU resolvent onto `res_cls` and `ComputeAllEqnResolvents` later pops that stack, reversing CSU enumeration order before insertion. Rust mirrors this with a temporary vector and `pop`; preserve or intentionally revise the reversal only with proof-output trace data.
- C ORs `subst_is_ho` across all CSU results for one selected literal before derivation entries are pushed, so one higher-order binding marks every popped resolvent from that literal as higher-order. Rust mirrors that aggregate flag; a cleaned internal API could track the flag per resolvent after drop-in proof compatibility is secured.
- C stores generated-resolvent parent pointers in the derivation stack. Rust records compact clause references in `DCEqRes` entries; replace them with stable handles before proof reconstruction traverses parent clauses.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
