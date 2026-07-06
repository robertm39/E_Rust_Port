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

### Rust Port Status Notes

- `src/clauses/derivation.rs` ports the C `ProofOutput` and `ProofObjectType` discriminants, including the typo-preserving `SimpleDeriviation` proof-object name, and the full derivation opcode/argument bit layout used by `DerivationCode`, including clause-generation/simplification entries, formula/CNF-conversion entries reserved for future formula owners, and higher-order entries with their `ArgIsHO` flag combinations.
- Rust now has a clause derivation entry stack that preserves existing rewrite-trace demodulator entries and adds C-shaped operation entries, clause-parent references, represented formula-parent references, and numeric arguments.
- The reusable `clause_push_derivation` helper ports the clause-parent subset of `ClausePushDerivation`, `clause_push_formula_derivation` covers represented FOF parent entries, `clause_push_numeric_derivation` covers numeric-argument entries, and `clause_push_ac_res_derivation` preserves `ClausePushACResDerivation` by pushing `DCACRes` plus the current AC-axiom stack count.
- Clause derivation pushes are wired for currently ported first-order generation and contraction helpers including factoring, equality resolution, disequality decomposition, condensation, contextual simplify-reflect, simplify-reflect, and local rewriting.
- `ClauseIsEvalGC`, `ClauseIsDummyQuote`, `ClauseIsDummyFOFQuote`, represented `ClauseDerivFindFirst`/`WFormulaDerivFindFirst` dummy-quote cascade following, borrowed represented `DerivedInProof`/`DerivedSetInProof`/`DerivedIsEvalGC` proof-step predicates, represented `DerivedDotNodeColour`/clause-link/formula-link colour helpers, `DerivStackExtractParents`, `DerivStackExtractOptParents`, `DerivStackIndicatesInitialClause`, and `DerivStackCountSearchInferences` are ported over the Rust derivation stack shape. Rust parent extraction returns compact clause references, compact represented formula references, and demodulator handles, and takes AC axiom references explicitly from the signature-owned compact AC axiom list.
- `DerivationStackPCLPrint`/`DerivationStackTSTPPrint` are ported for represented clause-side stacks, including represented formula-parent ids using generated `c_0_...`/`i_0_...` formula namespaces and context-aware `DCACRes` AC axiom parent expansion, and are used by current proof-object list and detailed DOT output alongside C-shaped source-info fallbacks. Proof-found and stopped/saturation list output now expands represented mixed clause/formula ancestors before extraction roots, emits formula labels in PCL/TSTP list output, and uses display-only sequential ids with represented clause-parent, formula-parent, rewrite-demodulator, and signature AC axiom parent references remapped for the printed list. Proof-object statistics now follow represented formula-parent refs and signature AC axiom parents, including formula dummy-quote cascades, and count formula steps, formula conjectures, and initial formulas. The represented proof-object graph now retains formula nodes, mixed clause/formula edges, and signature AC axiom clause-parent edges, and list/DOT output share a C-shaped root-backward mixed display order with separate axiom-stack reversal plus C's clause-parent/formula-parent stack-pop order for direct parents.
- Ordered proof-object extraction, reference-verified C sibling ordering/interleaving, exact renumbering, and full PCL/TSTP derived-step printing over ordered clause/formula derivations remain pending.

### Change Later

- C stores raw `Clause_p` and `WFormula_p` parent pointers directly in the derivation `PStack`. Rust currently stores compact clause references (`ident` plus CSSCPA source) and compact formula ids because stable proof-state clause/formula handles are not represented yet; replace these with stable clause/formula handles before full proof reconstruction and parent traversal are wired.
- `ClausePushDerivation` accepts `void*` arguments and validates them only through opcode bit assertions. Rust separates clause-parent, represented formula-parent, and numeric helpers for type safety; keep this split unless proof-output parity requires a single untyped stack API.
- `WFormulaGetId` can preserve source formula names through the mutable global `FormulasKeepInputNames`; Rust represented formula derivation refs currently render only generated `c_0_...`/`i_0_...` ids. Revisit named formula ids when real `WFormula` ownership and formula source-name policy are ported.
- `ClausePushACResDerivation` records only the current `sig->ac_axioms` stack length after `DCACRes`, while `DerivStackExtractParents` later expands that count into AC axiom parents from the signature. Rust keeps the count-only helper explicit, stores compact signature AC axiom refs, and now expands them for represented proof-object marking/statistics/graph traversal as well as derivation rendering; replace the count/ref split with stable proof-state parent handles only after full proof-object traversal owns clauses.
- C `ClauseIsEvalGC` reads the top stack element as an integer, so it only works for no-argument derivation entries whose opcode is at the stack top. Rust preserves the no-argument top-op behavior explicitly; revisit only if later callers need to scan through argument entries.
- C `ClauseDerivFindFirst` and `WFormulaDerivFindFirst` follow raw dummy-quote parent pointers without a cycle or shape guard beyond the dummy-quote predicates. Rust exposes the same cascade helpers through caller-resolved compact clause/formula references and stops on missing, self, or cyclic parents because compact identifiers remain a transitional stand-in for pointer-stable handles; proof-object analysis still collapses only literal-identical dummy clause quotes until stable ownership can distinguish all archived copies.
- C `DerivStackExtractOptParents` mutates raw parent pointers in the derivation stack after chasing dummy-quote parents. Rust exposes represented in-place replacement through caller-resolved compact clause/formula references, while keeping rewrite-demodulator handles and expanded `DCACRes` parents in their existing represented form; replace the resolver API with stable proof-state handles when pointer-like derivation identity is available.
- C `DerivedCell` stores either a raw clause pointer or formula pointer plus graph metadata. Rust currently exposes borrowed derived-step views over represented clauses/formulas for the ported proof/eval predicates, but does not yet model ref counts, root/fresh flags, or ordered derivation-graph ownership; keep the borrowed view narrow until full proof extraction owns stable graph nodes.
- `DerivedDotNodeColour` and its link-colour wrappers return shared raw GraphViz attribute fragments, using derivation-stack pointer presence to distinguish initial and derived nodes and hard-coded role colours, including formula-conjecture red versus negated-conjecture blue. Rust preserves the string table and parent-proof checks; the current borrowed executable graph applies the proof-member colour path for extracted clause/formula nodes without mutating their proof flags. Revisit the direct string fragments, parent-proof link-colour checks, and reference-verified mixed-node sibling ordering when the ordered `DerivedCell` graph owner is ported.
- `DerivationAnalyse` and proof-object rendering walk the ordered mixed `DerivedCell` graph. Rust now resolves represented formula-parent refs and signature AC axiom parents for statistics, follows formula dummy-quote cascades, keeps formula nodes/mixed edges plus AC axiom clause-parent edges in the represented graph, and emits formula nodes in list and DOT output using a C-shaped root-backward display order with an axiom stack and direct parent stack-pop order; keep the remaining ordering/identity split visible until ordered mixed-node rendering replaces compact refs.
- `DerivStackExtractParents` pushes `DCACRes` AC axiom parents into the result stack but does not add them to its returned count. Rust preserves that split as a direct-parent count plus appended AC parents; callers should not treat the count as the result length.
- `DerivStackCountSearchInferences` uses exact opcode cases rather than `DCOpIsGenerating`, so HO-marked variants and some generating-range operations such as `DCDisEqDecompose` are not counted there. Rust preserves the switch behavior until proof-analysis reference tests decide whether the C accounting is intentional.
- `DerivationPrintConditional` uses the process-global `DocOutputFormat`, compile-time `COMCHAR`, and an ordered derivation graph to render proof objects. Rust currently emits the supported executable's proof-object list framing/final proof-success step plus represented mixed clause/formula list and DOT output, including C-shaped PCL/TSTP derivation/source-info payloads, final/proof root markers only for list roots, proof-found extraction-root stack selection with represented `--full-deriv` additions, stopped ancestor expansion, shared display-only sequential ids, root-backward topological display ordering with C's separate axiom-stack reversal and direct parent stack-pop order, detailed TSTP graph labels for graph levels above 1, and child-coloured graph edges. It also has limited clause-side ancestor/statistics traversals for proof-object GC/training/`--proof-statistics` that walk compact parent references, signature AC axiom parents, and demodulator handles, collapse only literal-identical dummy CNF quotes, preserve mutated quote nodes as derived clauses, prefer selected archive copies for proof-step parents, prefer original quote sources for `DCCnfQuote` parents, and remap represented demodulator handles to display ids for proof-object list output. Full parity still needs stable pointer-like derivation identity, reference-verified C root/sibling ordering over mixed clause/formula nodes, exact C renumbering, formula-owner traversal, and ordered PCL/TSTP derived-step printers beyond the represented root subset.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
