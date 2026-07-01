<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# PCL2 / pcl_proofcheck

## Source Files

- [PCL2/pcl_proofcheck.h](../../../eprover/PCL2/pcl_proofcheck.h)
- [PCL2/pcl_proofcheck.c](../../../eprover/PCL2/pcl_proofcheck.c)

## Purpose

Data types and algorithms to realize proof checking for PCL2 protocols. the GNU Lesser General Public License. <1> Mon Apr 3 22:49:51 GMT 2000

Within the source tree, this unit belongs to `PCL2`. PCL protocol and proof-object support: proof steps, mini-protocols, identifiers, positions, expressions, checking, lemmas, and proof analysis.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `PCLCheckType`
- `ProverType`

### Macros And Constants

- `E_EXEC_DEFAULT`
- `OTTER_EXEC_DEFAULT`
- `PCL_PROOFCHECK`
- `SPASS_EXEC_DEFAULT`

### Globals

- None found in the source scan.

### Exported Functions

- `ClauseSet_p PCLGenerateCheck(PCLProt_p prot, PCLStep_p step)`
- `PCLCheckType PCLStepCheck(PCLProt_p prot, PCLStep_p step, ProverType prover, char* executable, long time_limit)`
- `long PCLCollectPreconds(PCLProt_p prot, PCLStep_p step, ClauseSet_p set)`
- `long PCLNegSkolemizeClause(PCLProt_p prot, PCLStep_p step, ClauseSet_p set)`
- `long PCLProtCheck(PCLProt_p prot, ProverType prover, char* executable, long time_limit, long* unchecked)`

## Implementation Notes

### Internal Functions

- `clause_print_dfg`
- `clause_print_otter`
- `clause_set_print_dfg`
- `clause_set_print_otter`
- `eqn_print_dfg`
- `eqn_print_otter`
- `pcl_run_prover`
- `pcl_verify_eprover`
- `pcl_verify_otter`
- `pcl_verify_spass`
- `sig_print_dfg`

### Source-Level Behavior

- `pcl_run_prover`: Execute command and scan the output for success. If found, return true, else return false;
- `pcl_verify_eprover`: Run E on the problem, return true if a proof is found.
- `eqn_print_otter`: Print a literal in Otter format.
- `clause_print_otter`: Print a clause in Otter format.
- `clause_set_print_otter`: Print a set of clauses in Otter format (with prolog-variables).
- `pcl_verify_otter`: Run Otter on the problem, return true if a proof is found.
- `sig_print_dfg`: Collect function symbols from set and print them in DFG syntax.
- `eqn_print_dfg`: Print an equation in DFG syntax.
- `clause_print_dfg`: Print a clause in dfg format.
- `clause_set_print_dfg`: Print a set of clauses in DFG format.
- `pcl_verify_spass`: Run SPASS on the problem, return true if a proof is found.
- `PCLCollectPreconds`: Collect copies of all clauses quoted in the justification of step in set. Return number of clauses.
- `PCLNegSkolemizeClause`: Add the clauses resulting from negating and skolemizing step->clause to set. The implementation is not very efficient, but that should not matter for our application.
- `PCLGenerateCheck`: Generate a clause set that is unsatisfiable if the clause in step is a logical conclusion of the precondition. For initial steps, return NULL.
- `PCLStepCheck`: Check the validity of a single PCL step. Return true if it checks ok, false otherwise. At the moment, just print the generated problem.
- `PCLProtCheck`: Check all steps in a PCL listing. Return number of successful steps.

### Dependencies

- `"pcl_proofcheck.h"`
- `<cio_tempfile.h>`
- `<pcl_protocol.h>`

### Compile-Time Conditions

- `PCL_PROOFCHECK`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Term sharing and term-bank insertion are semantic constraints, not just memory optimizations.
- Clause/literal mutation affects indexing, derivation, and proof reconstruction; preserve update ordering.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `PCL2/pcl_proofcheck.h`, `PCL2/pcl_proofcheck.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `PCL2` covering 2 source file(s), about 898 lines, 7 scanned public declarations, 11 scanned internal function definitions, and 16 structured function-comment blocks.
- Proof checker. Failure behavior and inference validation are compatibility targets for proof tooling.
- Proof-object code. Preserve identifier handling, step structure, protocol syntax, and proof-checking side effects.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Proof output/checking code is externally consumed; preserve identifiers, step ordering, and formatting details.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Rust Port Status

- Initial proofcheck support is ported as `src/pcl2/proofcheck.rs`, including `PCLCheckType`/`ProverType` equivalents, clausal precondition collection through full PCL references, copied parent-clause insertion into a check `ClauseSet`, negated skolemized unit generation for the target clause, check-problem construction, E/TPTP problem rendering and invocation, Otter problem rendering and invocation, SPASS/DFG problem rendering and invocation, output-level progress/prover-trace/failed-problem rendering through explicit output APIs, split-step unchecked classification, assumption classification for steps without clausal preconditions, and protocol-level checked/unchecked counting.

### Change Later

- `pcl_run_prover` constructs shell command strings and passes them to `popen`, including caller-provided executable text, then scans fixed 180-byte output lines for a success substring. Rust ports external prover execution with argument-vector process spawning and whole-output success scanning instead of shell-string execution; if compatibility with C-style executable strings containing shell syntax is needed later, add it as an explicit shell mode rather than as the default.
- `PCLCollectPreconds` only copies clausal parents and prints a warning for full first-order formula parents, so proofcheck silently weakens checks involving FOF parents. Rust preserves the clausal-only generated problem shape and should revisit formula-to-clause handling when full formula proof objects are integrated.
- `PCLNegSkolemizeClause` negates each literal of the skolemized target into a separate unit clause, which is correct for checking clause implication but loses any source metadata on those generated units. Rust mirrors the unit-generation shape and marks them as hypotheses; a later proofcheck problem owner should decide whether generated check clauses need explicit provenance.
- `PCLStepCheck` returns `CheckNotImplemented` for split clauses, and `PCLProtCheck` reports that as "assuming true" while incrementing `unchecked` rather than `res`. Rust keeps split clauses and prover variants without a C verifier (`NoProver`, `Setheo`) in the unchecked bucket; only genuine unimplemented inference checks should use that path.
- The C Otter and SPASS printers force LOP-oriented global output assumptions (`OutputFormat == LOPFormat`, `!EqnUseInfix`) and use ad hoc `$T`/`$F` and `spass_hack` encodings. Rust ports these as explicit renderers rather than hidden process-global state; revisit the legacy truth-literal and dummy-symbol encodings only after external-prover compatibility tests cover them.
- `sig_print_dfg` emits only symbols that occur in the generated check problem, plus dummy `spass_hack` and `spass_pred_dummy` declarations. This produces compact DFG but can hide signature declarations that are present but unused in the local check slice; keep this C behavior for compatibility and revisit only if SPASS integration needs richer declarations.
- `PCLStepCheck` and `PCLProtCheck` print progress, prover output traces, and failed generated problems through `GlobalOut` based on `OutputLevel`. Rust now exposes equivalent rendering through explicit output-aware APIs while keeping the core check APIs side-effect-light; when proofcheck is connected to a user-facing command path, route those wrappers through the executable/session output owner.
<!-- END MANUAL REVIEW: c_source_docs -->
