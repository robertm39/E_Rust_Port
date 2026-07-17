<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# HEURISTICS / che_prio_funs

## Source Files

- [HEURISTICS/che_prio_funs.h](../../../eprover/HEURISTICS/che_prio_funs.h)
- [HEURISTICS/che_prio_funs.c](../../../eprover/HEURISTICS/che_prio_funs.c)

## Purpose

Functions dealing with priorities for clauses. the GNU Lesser General Public License. <1> Sat Dec 5 16:45:41 MET 1998 New

Within the source tree, this unit belongs to `HEURISTICS`. Clause evaluation and strategy machinery: heuristic control blocks, priority functions, weight functions, literal selection, automatic strategy selection, and feature extraction.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `ClausePrioFun`

### Macros And Constants

- `CHE_PRIO_FUNS`

### Globals

- `extern char* PrioFunNames[]`

### Exported Functions

- `ClausePrioFun GetPrioFun(char* name)`
- `ClausePrioFun ParsePrioFun(Scanner_p in)`
- `EvalPriority PrioFunByAppVarNum(Clause_p clause)`
- `EvalPriority PrioFunByCreationDate(Clause_p clause)`
- `EvalPriority PrioFunByDerivationDepth(Clause_p clause)`
- `EvalPriority PrioFunByDerivationSize(Clause_p clause)`
- `EvalPriority PrioFunByHornDist(Clause_p clause)`
- `EvalPriority PrioFunByLiteralNumber(Clause_p clause)`
- `EvalPriority PrioFunByNegLitDist(Clause_p clause)`
- `EvalPriority PrioFunByPosLitNo(Clause_p clause)`
- `EvalPriority PrioFunConstPrio(Clause_p clause)`
- `EvalPriority PrioFunDeferFormulas(Clause_p clause)`
- `EvalPriority PrioFunDeferLambdas(Clause_p clause)`
- `EvalPriority PrioFunDeferNonUnitMaxPosEq(Clause_p clause)`
- `EvalPriority PrioFunDeferSOS(Clause_p clause)`
- `EvalPriority PrioFunDeferWatchlist(Clause_p clause)`
- `EvalPriority PrioFunGoalDifficulty(Clause_p clause)`
- `EvalPriority PrioFunPreferAppVar(Clause_p clause)`
- `EvalPriority PrioFunPreferDemods(Clause_p clause)`
- `EvalPriority PrioFunPreferEasyHO(Clause_p clause)`
- `EvalPriority PrioFunPreferFO(Clause_p clause)`
- `EvalPriority PrioFunPreferFormulas(Clause_p clause)`
- `EvalPriority PrioFunPreferGoals(Clause_p clause)`
- `EvalPriority PrioFunPreferGround(Clause_p clause)`
- `EvalPriority PrioFunPreferGroundGoals(Clause_p clause)`
- `EvalPriority PrioFunPreferHOSteps(Clause_p clause)`
- `EvalPriority PrioFunPreferHorn(Clause_p clause)`
- `EvalPriority PrioFunPreferLambdas(Clause_p clause)`
- `EvalPriority PrioFunPreferMixed(Clause_p clause)`
- `EvalPriority PrioFunPreferNegative(Clause_p clause)`
- `EvalPriority PrioFunPreferNew(Clause_p clause)`
- `EvalPriority PrioFunPreferNonAppVar(Clause_p clause)`
- `EvalPriority PrioFunPreferNonEqUnits(Clause_p clause)`
- `EvalPriority PrioFunPreferNonGoals(Clause_p clause)`
- `EvalPriority PrioFunPreferNonGround(Clause_p clause)`
- `EvalPriority PrioFunPreferNonHorn(Clause_p clause)`
- `EvalPriority PrioFunPreferNonUnits(Clause_p clause)`
- `EvalPriority PrioFunPreferPositive(Clause_p clause)`
- `EvalPriority PrioFunPreferProcessed(Clause_p clause)`
- `EvalPriority PrioFunPreferUnitAndNonEq(Clause_p clause)`
- `EvalPriority PrioFunPreferUnitGroundGoals(Clause_p clause)`
- `EvalPriority PrioFunPreferUnits(Clause_p clause)`
- `EvalPriority PrioFunPreferWatchlist(Clause_p clause)`
- `EvalPriority PrioFunSimulateSOS(Clause_p clause)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `GetPrioFun`: Given an external name, return a priority function or NULL if the name does not match any known function.
- `ParsePrioFun`: Parse a priority function and return it.
- `PrioFunPreferGroundGoals`: Return PrioPrefer for ground goals, PrioNormal for all other clauses.
- `PrioFunPreferUnitGroundGoals`: Return PrioPrefer for unit ground goals, PrioNormal for all other clauses.
- `PrioFunPreferGround`: Return PrioPrefer for ground clauses, PrioNormal for all other clauses.
- `PrioFunPreferNonGround`: Return PrioPrefer for non-ground clauses, PrioNormal for all other clauses.
- `PrioFunPreferProcessed`: Return PrioPrefer for clauses already procesed and eliminated by backwards-contraction.
- `PrioFunPreferNew`: Return PrioPrefer for new clauses, PrioNormal for all others. See PrioFunPreferProcessed.
- `PrioFunPreferGoals`: Return PrioPrefer for goals, PrioNormal for all other clauses.
- `PrioFunPreferNonGoals`: Return PrioPrefer for non-goals, PrioNormal for all other clauses.
- `PrioFunPreferMixed`: Return PrioPrefer for clauses that have both positive and negative literals (or neither), PrioNormal for all other clauses.
- `PrioFunPreferPositive`: Return PrioPrefer for clauses that have no negative literals, PrioNormal for all other clauses. The empty clause is both negative and positive.
- `PrioFunPreferNegative`: Return PrioPrefer for clauses that have no positive literals, PrioNormal for all other clauses. At the moment, this is mostly equivalent to PrioFunPreferGoals, but in the medium term I want to decouple the notion of goal (user-specified) from that of negative clause. The empty clause is both negative and positive.
- `PrioFunPreferUnits`: Return PrioPrefer for unit-clauses, PrioNormal for all other clauses.
- `PrioFunPreferNonEqUnits`: Return PrioPrefer for non-equational unit-clauses, PrioNormal for all other clauses.
- `PrioFunPreferDemods`: Return PrioPrefer for positive equational unit-clauses, PrioNormal for all other clauses.
- `PrioFunPreferNonUnits`: Return PrioPrefer for non-unit-clauses, PrioNormal for all other clauses.
- `PrioFunConstPrio`: Return PrioNormal.
- `PrioFunByLiteralNumber`: Return number of literals in the clause as a priority.
- `PrioFunByAppVarNum`: Assign the priority to be equal to the number of top-level applied variables.
- `PrioFunByDerivationDepth`: Return the derivation depth of the clause.
- `PrioFunByDerivationSize`: Return the derivation size of the clause.
- `PrioFunByNegLitDist`: Give a priority based on the number of negative (ground) literals: A negative-non-ground literal adds 3, a negative ground literal adds 1. Clauses with non-negative literals get a fixed priority.
- `PrioFunGoalDifficulty`: Give a priorty based on how simple a goal seems to be: Unit-Ground, Unit, Ground, General
- `PrioFunSimulateSOS`: Give priority PrioNormal to SOS clauses and initial clauses, and PrioDefer otherwise. Note that CPInitial is intentional and correkt ;-)
- `PrioFunDeferSOS`: Give priority to non-SOS and non-initial clauses.
- `PrioFunPreferHorn`: Return PrioPrefer for Horn clauses, PrioNormal for all other clauses.
- `PrioFunPreferNonHorn`: Return PrioPrefer for Non-Horn clauses, PrioNormal for all other clauses.
- `PrioFunPreferUnitAndNonEq`: Return PrioPrefer for units and all non-equational clauses, PrioNormal for all other clauses.
- `PrioFunDeferNonUnitMaxPosEq`: Return PrioPrefer for units and clauses without maximal positive equational literal, PrioNormal otherwise.
- `PrioFunByCreationDate`: Return the creation date of the clause. This allows us to combine a better FIFO with any other heuristic to sort clauses
- `PrioFunPreferWatchlist`: Prefer clauses that have subsumed a watchlist clause.
- `PrioFunDeferWatchlist`: Defer clauses that have subsumed a watchlist clause (probably useful only for symmetry reasons).
- `PrioFunByPosLitNo`: Class clauses by number of positive literals (more is worse).
- `PrioFunPreferAppVar`: Prefer clauses that have applied variables.
- `PrioFunPreferNonAppVar`: Prefer clauses that have no applied variables.
- `PrioFunPreferHOSteps`: Prefer clauses that have no applied variables.
- `PrioFunPreferLambdas`: Prefer clauses that have lambda subterms.
- `PrioFunPreferFormulas`: Prefer clauses that have formula subterms.
- `PrioFunDeferFormulas`: Prefer clauses that have no formula subterms.
- `PrioFunPreferEasyHO`: Prefer clauses that have no formula subterms.

### Dependencies

- `"che_prio_funs.h"`
- `<ccl_clauses.h>`
- `<ccl_derivation.h>`
- `<clb_simple_stuff.h>`

### Compile-Time Conditions

- `CHE_PRIO_FUNS`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Term sharing and term-bank insertion are semantic constraints, not just memory optimizations.
- Clause/literal mutation affects indexing, derivation, and proof reconstruction; preserve update ordering.
- Parser routines usually advance scanner state and may report fatal errors; keep token-consumption behavior exact.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22; updated for priority-helper port notes on 2026-06-29.

Source files reviewed: `HEURISTICS/che_prio_funs.h`, `HEURISTICS/che_prio_funs.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `HEURISTICS` covering 2 source file(s), about 1419 lines, 46 scanned public declarations, 0 scanned internal function definitions, and 43 structured function-comment blocks.
- Functions dealing with priorities for clauses. the GNU Lesser General Public License. <1> Sat Dec 5 16:45:41 MET 1998 New
- Strategy code. Preserve clause-evaluation semantics, priority ordering, weight formulas, and feature extraction because small changes can alter search behavior.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Parser functions usually consume input and report fatal diagnostics on mismatch; exact token flow matters for compatibility.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- File-static state should be audited for thread-safety and reset behavior in the Rust port.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.
- Priority functions feed clause scheduling directly, so preserve externally visible names, integer priority values, and small order-sensitive quirks until proof-search reference tests say otherwise.
- Several functions use clause/literal layout assumptions from the C allocation order rather than only semantic predicates. Treat those as compatibility constraints when porting.
- Higher-order priority helpers inspect derivation stacks and process-global `problemType`; tests that mutate this global state need serialization.

### Rust Port Status Notes

- `src/heuristics/prio_funs.rs` ports the C priority constants, name table, parser lookup surface, and the currently represented clause priority functions over an explicit `&TermBank`.
- `PrioFunPreferEasyHO` now preserves the C normal-result behavior for non-ArgCong clauses and returns `PrioBest` for represented `DCArgCong` derivations when the process problem type is higher-order. Production ArgCong generation writes `DCArgCong` plus the exact clause-parent reference, and focused tests pin both generation metadata and the unset/first-order/higher-order priority split.

### Change Later

- `PrioFunPreferHOSteps` scans the derivation stack and computes a higher-order-step flag, but the computed flag is not used before returning `PrioNormal`. Rust preserves the observable result; revisit only with scheduler traces that show whether this was intended.
- `PrioFunPreferEasyHO` computes formula/non-pattern preferences after the ArgCong special case, then discards them through `prio = PrioPrefer ? PrioNormal : PrioDefer`, making every non-ArgCong path return `PrioNormal`. Rust preserves this quirk while documenting it as a later heuristic cleanup candidate.
- `PrioFunPreferEasyHO` walks the raw C derivation `PStack` by reading an opcode and skipping argument slots according to opcode bits. Rust uses tagged derivation entries and compares `op_code(entry)` with `DOArgCong`; keep that compatibility comparison, but avoid exposing raw stack-layout stepping as a general Rust API.
- `PrioFunByNegLitDist` returns fixed priority `400` as soon as it sees any positive literal, so C's literal ordering makes most non-goal clauses skip the accumulated negative-literal distance. Rust mirrors this order-sensitive result until clause-ordering reference tests justify a semantic rewrite.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
- Before replacing a priority rule with a more obviously named semantic rule, pin down the C result with focused tests because small priority changes can alter proof search.
<!-- END MANUAL REVIEW: c_source_docs -->
