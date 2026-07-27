<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# PCL2 / pcl_protocol

## Source Files

- [PCL2/pcl_protocol.h](../../../eprover/PCL2/pcl_protocol.h)
- [PCL2/pcl_protocol.c](../../../eprover/PCL2/pcl_protocol.c)

## Purpose

Lists of PCL steps the GNU Lesser General Public License. <1> Sat Apr 1 22:17:54 GMT 2000 New

Within the source tree, this unit belongs to `PCL2`. PCL protocol and proof-object support: proof steps, mini-protocols, identifiers, positions, expressions, checking, lemmas, and proof analysis.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `PCLProtCell`
- `PCLProt_p`

### Macros And Constants

- `PCLProtCellAlloc()`
- `PCLProtCellFree(junk)`
- `PCLProtInsertStep(prot, step)`
- `PCLProtPrint(out, prot, format)`
- `PCLProtPrintProofClauses(out, prot, format)`
- `PCLProtStepNo(prot)`
- `PCLStepCollectPreconds(prot, step, tree)`
- `PCL_PROTOCOL`

### Globals

- None found in the source scan.

### Exported Functions

- `(prot),\ false, \ (format)) bool PCLStepHasFOFParent(PCLProt_p prot, PCLStep_p step)`
- `PCLExprCollectPreconds((prot), (step)->just, (tree)) PCLStep_p PCLExprGetQuotedArg(PCLProt_p prot, PCLExpr_p expr, int arg)`
- `PCLProtPrintPropClauses((out), (prot), PCLIsProofStep, format) void PCLProtPrintExamples(FILE* out, PCLProt_p prot)`
- `PCLProt_p PCLProtAlloc(void)`
- `PCLStep_p PCLProtFindStep(PCLProt_p prot, PCLId_p id)`
- `bool PCLProtDeleteStep(PCLProt_p prot, PCLStep_p step)`
- `bool PCLProtMarkProofClauses(PCLProt_p prot)`
- `long PCLProtCollectPropSteps(PCLProt_p prot, PCLStepProperties props, PStack_p steps)`
- `long PCLProtCountProp(PCLProt_p prot, PCLStepProperties props)`
- `long PCLProtParse(Scanner_p in, PCLProt_p prot)`
- `long PCLProtStripFOF(PCLProt_p prot)`
- `void PCLExprCollectPreconds(PCLProt_p prot, PCLExpr_p expr, PTree_p *tree)`
- `void PCLProtDelProp(PCLProt_p prot, PCLStepProperties props)`
- `void PCLProtFree(PCLProt_p junk)`
- `void PCLProtPrintExtra(FILE* out, PCLProt_p prot, bool data, OutputFormatType format)`
- `void PCLProtPrintPropClauses(FILE* out, PCLProt_p prot, PCLStepProperties prop, OutputFormatType format)`
- `void PCLProtResetTreeData(PCLProt_p prot, bool just_weights)`
- `void PCLProtSerialize(PCLProt_p prot)`
- `void PCLProtSetProp(PCLProt_p prot, PCLStepProperties props)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `PCLProtAlloc`: Return an initialized PCL protocol data structure.
- `PCLProtFree`: Free a PCL protocol
- `PCLProtExtractStep`: (Try to) take a step out of the protocol. Return true if it exists, false otherwise.
- `PCLProtDeleteStep`: Delete a step from a protocol. Return true if the step existed in the protocol, false otherwise. In the second case, the step is _not_ freed.
- `PCLProtFindStep`: Given a PCL-Identifier, find the matching step in prot.
- `PCLProtSerialize`: Ensure that prot->in_order is up to date
- `PCLProtParse`: Parse a PCL listing into prot. Return number of steps parsed.
- `PCLProtPrintExtra`: Print a PCL protocol.
- `PCLStepHasFOFParent`: Return true if one of the parents of step is a FOF step, false otherwise.
- `PCLProtStripFOF`: Remove all FOF steps from protocol. Make steps referencing a FOF step into initials and rewrite the justification accordingly. Expensive if there are FOF steps, reasonably cheap otherwise...
- `PCLProtResetTreeData`: Reset the tree data counters in all steps in the protocol.
- `PCLExprCollectPreconds`: Collect all PCL steps referenced in expr into tree.
- `PCLExprGetQuotedArg`: If the designated arg is a quote expression, retrieve and return the quoted step. Otherwise return NULL.
- `PCLProtMarkProofClauses`: Mark all proof steps in protocol with PCLIsProofStep. Return true if protocol describes a proof (i.e. contains the empty clause). otherwise.
- `PCLProtSetProp`: Set props in all clauses in the protocol.
- `PCLProtDelProp`: Set props in all clauses in the protocol.
- `PCLProtCountProp`: Return the number of steps with all properties in props set.
- `PCLProtCollectPropSteps`: Push all steps in prot with properties props set onto stack.
- `PCLProtPrintPropClauses`: Print all steps with prop set.
- `PCLProtPrintExamples`: Print all PCL steps that are marked as examples in example format.

### Dependencies

- `"pcl_protocol.h"`
- `<pcl_steps.h>`

### Compile-Time Conditions

- `PCL_PROTOCOL`

## E Reference Notes

- Use the ownership model visible in this unit's allocation/free helpers and exported APIs as evidence; preserve it only where correctness, supported compatibility, or measured performance requires it.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions reveal performance assumptions; measure allocation granularity and reuse before choosing Umlaut's representation.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Audit where clause/literal mutation order affects indexing, derivation, proof reconstruction, or deterministic behavior before changing it.
- Parser routines usually advance scanner state and may report fatal errors; preserve supported input behavior or document and test an intentional divergence.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22; updated for safe protocol storage and traversal semantics on 2026-07-17.

Source files reviewed: `PCL2/pcl_protocol.h`, `PCL2/pcl_protocol.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `PCL2` covering 2 source file(s), about 869 lines, 21 scanned public declarations, 0 scanned internal function definitions, and 20 structured function-comment blocks.
- Main PCL protocol representation and I/O. Textual proof compatibility depends on identifier and step syntax.
- Proof-object code. Preserve identifier handling, step structure, protocol syntax, and proof-checking side effects.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Parser functions usually consume input and report fatal diagnostics on mismatch; exact token flow matters for compatibility.
- Proof output/checking code is externally consumed; preserve identifiers, step ordering, and formatting details.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Rust Port Status

- Initial Rust support is in `src/pcl2/protocol.rs`, covering protocol-owned term-bank initialization, full-step insertion/find/extract/delete, deterministic serialization by C-shaped PCL-id comparison, positive-id protocol parsing, scanner-comment forwarding through explicit output-aware parsing, duplicate-id rejection, ordered PCL/TSTP/TPTP/LOP rendering, FOF-parent detection, FOF stripping with justification reset, tree-data reset, recursive full-expression precondition collection, direct quoted-argument lookup, proof-step marking, bulk property mutation, property counting/collection, property-filtered printing, and output-format-aware example printing.
- Rust stores steps in a sorted `Vec<PclStep>` and re-sorts on demand rather than a raw-pointer `PTree` plus cached `PStack`. The public operations still preserve sorted traversal and C-comparator duplicate detection.
- The vector is the canonical owner rather than a second raw-pointer cache. Binary search preserves C-comparator lookup/duplicate semantics, common increasing-id protocol input appends without shifting, extraction returns ownership directly, and Rust borrows prevent callers from retaining invalid step references across mutation. Boxed clause content remains address-stable if the vector relocates a step.
- Duplicate parse errors leave Rust's stored count equal to actual membership. C increments `number` before detecting the duplicate and then terminates through `Error`, so the inconsistent count is not observable to a continuing successful caller.
- Output-aware parsing is wired through `epclextract` when comment forwarding is enabled. Precondition collection rejects dangling identifiers before traversal, deduplicates them, and uses deterministic C-comparator PCL-id order; the current consumers only mark/query properties, so C's raw-pointer-tree extraction order has no output effect.
- FOF stripping intentionally resets a dependent clause's justification to `initial` without setting `PCLIsInitial`, matching C's property/justification split.

### Change Later

- `PCLProtInsertStep` increments `prot->number` before `PTreeObjStore` reports a duplicate. The parser immediately raises a fatal duplicate-id error, but the count mutation is observable before unwinding in C. Rust rejects duplicate insertion without changing the stored step count; revisit only if a nonfatal duplicate-insertion API becomes necessary.
- `PCLProtParse` echoes scanner comments to `GlobalOut` when `ignore_comments` is false. Rust exposes equivalent behavior through an explicit output-aware parse wrapper, and `epclextract` routes that wrapper through its configured output owner when comments are enabled; retain the explicit output boundary instead of reintroducing `GlobalOut`.
- `PCLExprCollectPreconds` stores the result of `PCLProtFindStep` without an explicit null check. Dangling quoted parents can therefore feed a null pointer into later proof-marking traversal. Rust reports a syntax diagnostic for dangling references instead of modeling the null crash surface.
- `PCLProtStripFOF` resets dependent clause justifications to `initial` when a FOF parent is removed, but it does not set `PCLIsInitial` on those steps. Rust preserves the justification-only reset; later proof analysis should decide whether property and justification should be kept consistent.
- C stores preconditions in pointer-ordered `PTree` nodes; Rust uses deterministic PCL-id ordering. This should be unobservable for current property marking, but revisit if future proof traversal emits parent-processing order.
- `PCLProtPrintExamples` passes the zero-based serialized stack index as the example id and renders clauses through the global `OutputFormat` via `PCLStepPrintExample`/`ClausePrint`. Rust preserves the zero-based index and exposes output-format-aware example printing explicitly; executable integration should pass the active output format rather than adding hidden global state.
<!-- END MANUAL REVIEW: c_source_docs -->
