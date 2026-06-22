<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CLAUSES / ccl_derivation

## Source Files

- [CLAUSES/ccl_derivation.h](../../../eprover/CLAUSES/ccl_derivation.h)
- [CLAUSES/ccl_derivation.c](../../../eprover/CLAUSES/ccl_derivation.c)

## Purpose

Datatypes and definitions for compact representation of derivations of a clause.

Within the source tree, this unit belongs to `CLAUSES`. Clause, formula, literal, inference, simplification, indexing, subsumption, grounding, relevance, and proof-state machinery.

Authors noted in source headers: Stephan Schulz (schulz@eprover.org)

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `ArgDesc`
- `DerivationCell`
- `DerivationCode`
- `Derivation_p`
- `DerivedCell`
- `Derived_p`
- `OpCode`
- `ProofObjectType`
- `ProofOutput`

### Macros And Constants

- `CCL_DERIVATION`
- `DCOpHasArg1(op)`
- `DCOpHasArg2(op)`
- `DCOpHasCnfArg1(op)`
- `DCOpHasCnfArg2(op)`
- `DCOpHasFofArg1(op)`
- `DCOpHasFofArg2(op)`
- `DCOpHasNumArg1(op)`
- `DCOpHasNumArg2(op)`
- `DCOpHasParentArg1(op)`
- `DCOpHasParentArg2(op)`
- `DCOpIsGenerating(op)`
- `DPGetIsHO(op)`
- `DPOpGetOpCode(op)`
- `DPSetIsHO(op)`
- `DerivationCellAlloc()`
- `DerivationCellFree(junk)`
- `DerivedCellAlloc()`
- `DerivedCellFree(junk)`
- `DerivedFree(junk)`
- `DerivedGetDerivstack(d)`

### Globals

- `extern ProofObjectType PrintProofObject`
- `extern bool ProofObjectRecordsGCSelection`

### Exported Functions

- `((d)->clause?(d)->clause->derivation:(d)->formula->derivation) bool DerivedInProof(Derived_p derived)`
- `Clause_p ClauseDerivFindFirst(Clause_p clause)`
- `Derivation_p DerivationAlloc(Sig_p sig)`
- `Derivation_p DerivationCompute(PStack_p root_clauses, Sig_p sig)`
- `Derived_p DerivationGetDerived(Derivation_p derivation, Clause_p clause, WFormula_p formula)`
- `Derived_p DerivedAlloc(void)`
- `WFormula_p WFormulaDerivFindFirst(WFormula_p form)`
- `bool ClauseIsDummyQuote(Clause_p clause)`
- `bool ClauseIsEvalGC(Clause_p clause)`
- `bool DerivedIsEvalGC(Derived_p derived)`
- `long DerivStackExtractOptParents(PStack_p derivation, Sig_p sig, PStack_p res_clauses, PStack_p res_formulas)`
- `long DerivStackExtractParents(PStack_p derivation, Sig_p sig, PStack_p res_clauses, PStack_p res_formulas)`
- `long DerivationCollectFCodes(Derivation_p derived, NumTree_p *tree)`
- `long DerivationExtract(Derivation_p derivation, PStack_p root_clauses)`
- `long DerivationMarkProofSteps(Derivation_p derivation)`
- `long DerivationTopoSort(Derivation_p derivation)`
- `long DerivedCollectFCodes(Derived_p derived, NumTree_p *tree)`
- `void ClausePushACResDerivation(Clause_p clause, Sig_p sig)`
- `void ClausePushDerivation(Clause_p clause, DerivationCode op, void* arg1, void* arg2)`
- `void DerivStackCountSearchInferences(PStack_p derivation, unsigned long *generating_count, unsigned long *simplifying_count)`
- `void DerivationAnalyse(Derivation_p derivationt)`
- `void DerivationComputeAndPrint(FILE* out, char* status, PStack_p root_clauses, Sig_p sig, ProofOutput print_derivation, bool print_analysis)`
- `void DerivationDebugPrint(FILE* out, PStack_p derivation)`
- `void DerivationDotPrint(FILE* out, Derivation_p derivation, ProofOutput print_derivation)`
- `void DerivationFree(Derivation_p junk)`
- `void DerivationPrint(FILE* out, Derivation_p derivation)`
- `void DerivationPrintConditional(FILE* out, char* status, Derivation_p derivation, Sig_p sig, ProofOutput print_derivation, bool print_analysis)`
- `void DerivationRenumber(Derivation_p derivation)`
- `void DerivationStackPCLPrint(FILE* out, Sig_p sig, PStack_p derivation)`
- `void DerivationStackTSTPPrint(FILE* out, Sig_p sig, PStack_p derivation)`
- `void DerivedDotPrint(FILE* out, Sig_p sig, Derived_p derived, ProofOutput print_derivation)`
- `void DerivedPCLPrint(FILE* out, Sig_p sig, Derived_p derived)`
- `void DerivedSetInProof(Derived_p derived, bool in_proof)`
- `void DerivedTSTPPrint(FILE* out, Sig_p sig, Derived_p derived)`
- `void WFormulaPushDerivation(WFormula_p form, DerivationCode op, void* arg1, void* arg2)`

## Implementation Notes

### Internal Functions

- `derivation_find_max_id`
- `derived_free_wrapper`

### Source-Level Behavior

- `derived_free_wrapper`: Free a Derived cell (for PObjTreeFree).
- `derived_compare`: Compare two derived cells by their clause or formula.
- `derived_get_derivation`: Given a derived cell, return the derivation of the clause or formula (or NULL in none).
- `get_clauseform_id`: Return the identifier of the selected argument of the operator, assuming that clauseform points to the corresponding clause or formula.
- `tstp_get_clauseform_id`: Return a TSTP identifier for a derivation stack clause- or formula reference.
- `derivation_find_max_id`: Find the largest input id (in ClauseInfo fields) of any formula in derivation.
- `DerivedInProof`: Return true if the derived cell is known to be in proof. This is the case if it is the empty clause, or if it marked as being in a proof (presumably because one of its transitive descendants is the empty clause).
- `DerivedSetInProof`: Mark a derived cell as a proof cell.
- `DerivedCollectFCodes`: Collect all f_codes from the logical clause/formula into *tree. Return number of new entries found.
- `ClausePushDerivation`: Push the derivation items (op-code and suitable number of arguments) onto the derivation stack.
- `ClausePushACResDerivation`: Push the derivation items (op-code and suitable number of arguments) onto the derivation stack.
- `WFormulaPushDerivation`: Push the derivation items (op-code and suitable number of arguments) onto the derivation stack.
- `ClauseIsEvalGC`: Return true if the clause is the form of the given clause that was evaluation and then selected for processing. This assumes that the DCCnfEvalGC opcode is on top of the derivation stack of such clauses.
- `ClauseIsDummyQuote`: Return true if the clause is just generated as a quote of its single parent.
- `ClauseIsDummyFOFQuote`: Return true if the clause is just generated as a quote of its single (FOF) parent.
- `ClauseDerivFindFirst`: Given a clause, check if it's part of a reference cascade (i.e. has just on parent and is justified by a simple reference to the parent (via OpCode DCCnfQuote)). If yes, track back the reference cascade and return the first (original) occurrence of the clause. Otherwise return the clause.
- `WFormulaDerivFindFirst`: Given a formula, check if it's part of a reference cascade (i.e. has just on parent and is justified by a simple reference to the parent (via OpCode DCFofQuote)). If yes, track back the reference cascade and return the first (original) occurrence of the formula. Otherwise return the formula.
- `DerivStackExtractParents`: Given a derivation stack (derivation-codes with arguments), return all the (occurances of) all the side premises referenced in the derivation (via the result stacks). Return value is the number of premises found.
- `DerivStackExtractOptParents`: Given a derivation stack (derivation-codes with arguments), return all the (occurrences of) all the original instances of the side premises referenced in the derivation (via the result stacks). Return value is the number of premises found. Modify the derivation to replace references to a parent with references to the original instance of that parent.
- `DerivStackCountSearchInferences`: Given a derivation stack (derivation-codes with arguments), count the number of generating and simplifying inferences in the stack.
- `DerivedAlloc`: Allocate an empty initialized DerivedCell.
- `DerivationStackPCLPrint`: Print a very short description of the derivation for debug purposes.
- `DerivationStackTSTPPrint`: Print the derivation stack as a TSTP expression.
- `DerivedPCLPrint`: Print a "Derived" cell - i.e. the clause or formula, and its derivation, in PCL format
- `DerivedTSTPPrint`: Print a "Derived" cell - i.e. the clause or formula, and its derivation, in TSTP format
- `DerivedDotNodeColour`: Return a string description of the colour to use for a given node in a derivation.
- `DerivedDotClauseLinkColour`: Return a string description of the colour to use for a given link in a derivation.
- `DerivedDotFormulaLinkColour`: Return a string description of the colour to use for a given link in a derivation.
- `DerivedDotPrint`: Print a "Derived" cell - i.e. the clause or formula, and its derivation, in GraphViz DOT format
- `DerivedIsEvalGC`: Return true if the step corresponds to the evaluated and selected form of a given clause.
- `DerivStackIndicatesInitialClause`: Return true if the derivation stack is empty, or if all parents are formulas (not clauses). This is ugly - it cannot reuse DerivStackExtractParents() since the the signature is not known.
- `DerivationAlloc`: Allocate an empty derivation.
- `DerivationFree`: Free a derivation.
- `DerivationGetDerived`: Given a clause or formula, return the associated cell of the derivation. If none exists, create a new one. Only one of "clause", "formula" can be set.
- `DerivationExtract`: Extract the proof tree of the clauses on root_clauses and annotate each "Derived" node with the number of in-references. Return number of roots.
- `DerivationMarkProofSteps`: Go through the derivation, marking all proof steps. Assumes that derivation->roots provides (direct or indirect) access to all proof steps. Sets derivation->has_conjecture if a conjecture-type clause or formula is in the proof tree.
- `DerivationTopoSort`: Perform a topological sort of the derivation. This is slightly hacked because axioms (nodes without further parents) always come first, so that axioms are listed first (for convenience and user expectation).
- `DerivationRenumber`: Renumber clauses and formulas in a derivation in order.
- `DerivationCompute`: Given a set (stack) of final clauses, generate an ordered derivation from it.
- `DerivationAnalyse`: Compute a number of statistics for a derivation.
- `DerivationCollectFCodes`: Collect all f_codes from derivation into tree.
- `DerivationPrint`: Print a derivation.
- `DerivationDotPrint`: Print a derivation as a DOT graph.
- `DerivationPrintConditional`: Print a derivation and its statistics, based on the selected inputs.
- `DerivationComputeAndPrint`: Compute, print, and discard a derivation.

### Dependencies

- `"ccl_derivation.h"`
- `<ccl_clauses.h>`
- `<ccl_formula_wrapper.h>`
- `<ccl_inferencedoc.h>`

### Compile-Time Conditions

- `CCL_DERIVATION`

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

Source files reviewed: `CLAUSES/ccl_derivation.h`, `CLAUSES/ccl_derivation.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `CLAUSES` covering 2 source file(s), about 2940 lines, 46 scanned public declarations, 2 scanned internal function definitions, and 46 structured function-comment blocks.
- Datatypes and definitions for compact representation of derivations of a clause.
- Clause/formula code. Pay close attention to mutation ordering because indexes, derivation records, and proof-state sets are updated around the same objects.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Parser functions usually consume input and report fatal diagnostics on mismatch; exact token flow matters for compatibility.
- Ordering comparisons feed simplification and inference eligibility; preserve tie-breakers, cache use, and incomparability results.
- Proof output/checking code is externally consumed; preserve identifiers, step ordering, and formatting details.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
