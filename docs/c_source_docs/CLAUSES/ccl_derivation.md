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

Manual review status: reviewed for porting-relevant behavior on 2026-06-22; updated for formula-copy proof identity, ordered proof renumbering, typed proof declarations, dummy-quote collapse, and AC-parent extraction on 2026-07-12.

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
- Rust now has a clause derivation entry stack that preserves rewrite-trace demodulator entries and adds C-shaped operation entries, clause-parent references, represented formula-parent references, and numeric arguments. Clause parents and rewrite demodulators carry an internal generation in addition to the visible C id/source fields so archived and requeued objects with the same visible identity remain distinct.
- The reusable `clause_push_derivation` helper ports the clause-parent subset of `ClausePushDerivation`, `clause_push_formula_derivation` covers represented FOF parent entries, `clause_push_numeric_derivation` covers numeric-argument entries, and `clause_push_ac_res_derivation` preserves `ClausePushACResDerivation` by pushing `DCACRes` plus the current AC-axiom stack count.
- Clause derivation pushes are wired for currently ported first-order generation and contraction helpers including factoring, equality resolution, disequality decomposition, condensation, contextual simplify-reflect, simplify-reflect, and local rewriting.
- `ClauseIsEvalGC`, `ClauseIsDummyQuote`, `ClauseIsDummyFOFQuote`, represented `ClauseDerivFindFirst`/`WFormulaDerivFindFirst` dummy-quote cascade following, borrowed represented `DerivedInProof`/`DerivedSetInProof`/`DerivedIsEvalGC` proof-step predicates, represented `DerivedDotNodeColour`/clause-link/formula-link colour helpers, `DerivStackExtractParents`, `DerivStackExtractOptParents`, `DerivStackIndicatesInitialClause`, and `DerivStackCountSearchInferences` are ported over the Rust derivation stack shape. Rust parent extraction returns compact clause references, compact represented formula references, and demodulator handles, and takes AC axiom references explicitly from the signature-owned compact AC axiom list.
- `DerivationStackPCLPrint`/`DerivationStackTSTPPrint` are ported for represented clause-side stacks, including C's direct-parent rendering for both `DCCnfQuote` and `DCFofQuote`, input-name-preserving formula-parent ids, and context-aware `DCACRes` AC axiom parent expansion. Proof extraction now preserves C's asymmetric parent resolution: direct clause parents pass through `ClauseDerivFindFirst`, while signature AC axioms appended by `DCACRes` retain their exact clause node. Proof-found and stopped/saturation list output expands represented mixed clause/formula ancestors before extraction roots, distinguishes flat formula copies by a stable internal source while preserving C's duplicated visible id, and applies C's `ClauseInfoGetIdCounter`-based starting id, direct-parent stack-pop order, and display-only renumbering. The archived `ans_test06.p`, `socrates.p`, and current `ALL_RULES.p` proof lists match C after path normalization. `ALL_RULES.p` retains the positive-predicate rewrite, identity rewrite, associativity rewrites, all three AC-resolution axiom parents, and the final `c=a` ancestry without exposing internal compact ids. Proof-object statistics follow represented formula-parent refs and signature AC axiom parents, including formula dummy-quote cascades and formula roots, and count formula steps, formula conjectures, and initial formulas.
- `ProofObjectGraph` owns borrowed references to the exact extracted clause/formula nodes for its lifetime and now exposes C `DerivationTopoSort` followed by `DerivationPrint` order directly. The shared list/DOT order preserves root and released-derived FIFO processing, separate clause-before-formula parent release, the final LIFO axiom block, and mixed multi-root sibling interleaving without cloning or renumbering the owners. Represented PCL/TSTP derived-step composition is covered by the complete C operation-id/status/theory table plus every structural argument/special case; the archived `ans_test06.p`, `socrates.p`, `ALL_RULES.p`, and `LUSK6.lop` comparisons remain exact after path normalization.

### Change Later

- C stores raw `Clause_p` and `WFormula_p` parent pointers directly in the derivation `PStack`. Clause archive/requeue cycles and `WFormulaFlatCopy` deliberately duplicate visible numeric ids, so dummy-quote chasing, graph extraction, and later `DerivationRenumber` depend on pointer identity. Rust uses clause generations and formula wrapper-source keys to recover that distinction. A cleaned proof API should use explicit stable arena handles instead of raw pointers or mutable duplicated ids.
- `ClausePushDerivation` accepts `void*` arguments and validates them only through opcode bit assertions. Rust separates clause-parent, represented formula-parent, and numeric helpers for type safety; keep this split unless proof-output parity requires a single untyped stack API.
- `WFormulaGetId` preserves source formula names through the mutable global `FormulasKeepInputNames`, and `DerivationStackTSTPPrint` obtains them by dereferencing raw formula parents. Rust resolves stable formula-source keys to the display graph's input names; a cleaned proof renderer should carry an explicit immutable identifier table rather than depend on mutable wrapper ids and a global name policy.
- `ClausePushACResDerivation` records only the current `sig->ac_axioms` stack length after `DCACRes`, while `DerivStackExtractParents` later expands that count into AC axiom parents from the signature. Rust keeps the count-only helper explicit, stores compact signature AC axiom refs, and now expands them for represented proof-object marking/statistics/graph traversal as well as derivation rendering; replace the count/ref split with stable proof-state parent handles only after full proof-object traversal owns clauses.
- C `ClauseIsEvalGC` reads the top stack element as an integer, so it only works for no-argument derivation entries whose opcode is at the stack top. Rust preserves the no-argument top-op behavior explicitly; revisit only if later callers need to scan through argument entries.
- C `ClauseDerivFindFirst` and `WFormulaDerivFindFirst` follow raw dummy-quote parent pointers without a cycle or shape guard beyond the dummy-quote predicates. Rust exposes the same cascade helpers through caller-resolved source-tagged references and stops on missing, self, or cyclic parents while preserving C's clause behavior of collapsing every structurally valid quote even when its literals were mutated; formula copies with duplicated visible ids remain distinguished.
- C `DerivStackExtractOptParents` mutates raw parent pointers in the derivation stack after chasing dummy-quote parents. Rust exposes represented in-place replacement through caller-resolved compact clause/formula references, while keeping rewrite-demodulator handles and expanded `DCACRes` parents in their existing represented form; replace the resolver API with stable proof-state handles when pointer-like derivation identity is available.
- C `DerivedCell` stores either a raw clause pointer or formula pointer plus graph metadata. Rust's extracted graph keeps exact borrowed clause/formula owners and computes C reference counts and order transiently, but does not persist C's mutable `is_fresh`, `is_root`, or destructive `ref_count` cells; keep those mechanics derived from immutable graph/root data unless another C consumer needs the mutations themselves.
- `DerivedDotNodeColour` and its link-colour wrappers return shared raw GraphViz attribute fragments, using derivation-stack pointer presence to distinguish initial and derived nodes and hard-coded role colours, including formula-conjecture red versus negated-conjecture blue. Rust preserves the string table and parent-proof checks; the borrowed executable graph applies the proof-member colour path for extracted clause/formula nodes without mutating their proof flags and shares the verified mixed-node order with list output. Revisit only the direct string fragments and parent-proof link-colour mutation boundary if a later caller needs C's writable flags.
- `DerivationAnalyse` and proof-object rendering walk the ordered mixed `DerivedCell` graph. Rust resolves represented formula-parent refs and signature AC axiom parents for statistics, follows formula dummy-quote cascades, keeps formula nodes/mixed edges plus AC axiom clause-parent edges in the represented graph, and exposes the C-shaped root-backward order from that owner graph for both list and DOT output. Compact stable parent keys still resolve the borrowed nodes instead of storing raw pointers, as documented above.
- C `DerivationExtract` stores roots as `DerivedCell` nodes over raw clause or formula pointers. Rust represents proof-state extraction roots as separate cloned clause and formula stacks; replace the split clone stacks with stable mixed handles once proof-state clause/formula ownership can provide pointer-like identity.
- `DerivStackExtractParents` pushes `DCACRes` AC axiom parents into the result stack but does not add them to its returned count. Rust preserves that split as a direct-parent count plus appended AC parents; callers should not treat the count as the result length.
- `DerivStackExtractOptParents` runs `ClauseDerivFindFirst` for explicit opcode arguments but appends `DCACRes` parents directly from `sig->ac_axioms` without optimizing their dummy-quote chains. The same clause pointer can therefore denote an original parent in a rewrite subexpression and a selected/requeued parent in the enclosing AC-resolution step. Rust preserves this context-dependent graph identity; a future proof model should encode parent-edge resolution policy explicitly instead of making it an accidental consequence of two loops over raw pointers.
- `DerivStackCountSearchInferences` uses exact opcode cases rather than `DCOpIsGenerating`, so HO-marked variants and some generating-range operations such as `DCDisEqDecompose` are not counted there. Rust preserves the switch behavior until proof-analysis reference tests decide whether the C accounting is intentional.
- `DerivationPrintConditional` uses the process-global `DocOutputFormat`, compile-time `COMCHAR`, and an ordered derivation graph to render proof objects. Rust emits the supported executable's proof-object list framing/final proof-success step plus represented mixed clause/formula list and DOT output, including C-shaped PCL/TSTP derivation/source-info payloads, final/proof root markers only for list roots, proof-found extraction-root stack selection with represented `--full-deriv` additions, stopped ancestor expansion, shared display-only sequential ids, reference-pinned mixed root/sibling ordering with C's separate axiom-stack reversal and direct parent stack-pop order, detailed TSTP graph labels for graph levels above 1, and child-coloured graph edges. Its clause-side ancestor/statistics traversals for proof-object GC/training/`--proof-statistics` walk compact parent references, signature AC axiom parents, and demodulator handles, collapse structurally valid dummy CNF quotes even after literal mutation like C, prefer selected archive copies for proof-step parents, prefer original quote sources for `DCCnfQuote` parents, and remap represented demodulator handles to display ids. Remaining expansion belongs to unrepresented parser/clausification owners rather than this ordered graph/renderer core.
- `ClauseDerivFindFirst` treats any two-entry `DCCnfQuote` stack as a transparent alias without checking that child and parent clauses remain logically or textually equal. Because archive/requeue copies can be mutated after the quote is pushed, proof extraction can silently replace a used clause with older literals. Rust preserves this lossy compatibility behavior; a future proof model should represent aliases explicitly and reject or terminate alias collapse once either clause changes.
- `DerivedPCLPrint` has its own fixed-width, spaced record layout instead of reusing the configurable proof-documentation printer. It ignores `PCLStepCompact`, uses global `PCLFullTerms` for clauses, and hard-codes full terms for formulas. Rust preserves those distinctions in proof-object list output; a cleaned renderer should use one explicit derived-step layout/options object after byte-compatible output no longer depends on the asymmetry.
- When `sig->typed_symbols` is set, `DerivationPrintConditional` rescans the ordered derivation for function codes, expands their complete type closure, and prints sort and symbol declarations immediately before the proof. Rust preserves that output boundary over its display graph. A cleaned proof API should attach the required declaration closure to the extracted proof object, avoiding render-time graph traversal and the hidden signature flag while preserving declaration identifiers and order.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
