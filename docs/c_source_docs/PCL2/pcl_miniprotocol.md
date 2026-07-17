<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# PCL2 / pcl_miniprotocol

## Source Files

- [PCL2/pcl_miniprotocol.h](../../../eprover/PCL2/pcl_miniprotocol.h)
- [PCL2/pcl_miniprotocol.c](../../../eprover/PCL2/pcl_miniprotocol.c)

## Purpose

Lists of MiniPCL steps the GNU Lesser General Public License. <1> Thu Jul 11 17:37:03 MEST 2002 New (from pcl_rotocol.h

Within the source tree, this unit belongs to `PCL2`. PCL protocol and proof-object support: proof steps, mini-protocols, identifiers, positions, expressions, checking, lemmas, and proof analysis.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `PCLMiniProtCell`
- `PCLMiniProt_p`

### Macros And Constants

- `PCLMiniProtCellAlloc()`
- `PCLMiniProtCellFree(junk)`
- `PCL_MINIPROTOCOL`

### Globals

- None found in the source scan.

### Exported Functions

- `PCLMiniProt_p PCLMiniProtAlloc(void)`
- `PCLMiniStep_p PCLMiniProtExtractStep(PCLMiniProt_p prot, PCLMiniStep_p step)`
- `PCLMiniStep_p PCLMiniProtFindStep(PCLMiniProt_p prot, unsigned long id)`
- `bool PCLMiniProtDeleteStep(PCLMiniProt_p prot, PCLMiniStep_p step)`
- `bool PCLMiniProtInsertStep(PCLMiniProt_p prot, PCLMiniStep_p step)`
- `bool PCLMiniProtMarkProofClauses(PCLMiniProt_p prot, bool fast)`
- `long PCLMiniProtParse(Scanner_p in, PCLMiniProt_p prot)`
- `void PCLMiniExprCollectPreconds(PCLMiniProt_p prot, PCLExpr_p expr, PTree_p *tree)`
- `void PCLMiniProtDelClauseProp(PCLMiniProt_p prot, PCLStepProperties props)`
- `void PCLMiniProtFree(PCLMiniProt_p junk)`
- `void PCLMiniProtPrint(FILE* out, PCLMiniProt_p prot, OutputFormatType format)`
- `void PCLMiniProtPrintProofClauses(FILE* out, PCLMiniProt_p prot, OutputFormatType format)`
- `void PCLMiniProtSetClauseProp(PCLMiniProt_p prot, PCLStepProperties props)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `PCLMiniProtAlloc`: Return an initialized PCLMini protocol data structure.
- `PCLMiniProtFree`: Free a PCLMini protocol
- `PCLMiniProtInsertStep`: Insert a step into prot. Return true if it was not already in the protokol, otherwise false.
- `PCLMiniProtFindStep`: Given a PCLMini-Identifier, find the matching step in prot.
- `PCLMiniProtExtractStep`: Extract the step from the protokol and return it.
- `PCLMiniProtDeleteStep`: Delete a step from a protocol. Return true if the step existed in the protocol, false otherwise. In the second case, the step is _not_ freed.
- `PCLMiniProtParse`: Parse a PCLMini listing into prot. Return number of steps parsed.
- `PCLMiniProtPrint`: Print a PCLMini protocol.
- `PCLMiniExprCollectPreconds`: Collect all PCLMini steps referenced in expr into tree.
- `PCLMiniProtMarkProofClauses`: Mark all proof steps in protokoll with PCLIsProofStep. Return true if empty clause was encountered.
- `PCLMiniProtSetClauseProp`: Set a property in a PCLMiniStep protocol.
- `PCLMiniProtDelClauseProp`: Delete a property in a PCLMiniSteps protocol.
- `PCLMiniProtPrintProofClauses`: Print a PCLMini protocol.

### Dependencies

- `"pcl_miniprotocol.h"`
- `<pcl_ministeps.h>`

### Compile-Time Conditions

- `PCL_MINIPROTOCOL`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Clause/literal mutation affects indexing, derivation, and proof reconstruction; preserve update ordering.
- Parser routines usually advance scanner state and may report fatal errors; keep token-consumption behavior exact.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22; updated for storage, ownership, and traversal equivalence on 2026-07-17.

Source files reviewed: `PCL2/pcl_miniprotocol.h`, `PCL2/pcl_miniprotocol.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `PCL2` covering 2 source file(s), about 612 lines, 15 scanned public declarations, 0 scanned internal function definitions, and 13 structured function-comment blocks.
- Lists of MiniPCL steps the GNU Lesser General Public License. <1> Thu Jul 11 17:37:03 MEST 2002 New (from pcl_rotocol.h
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

- Initial Rust support is in `src/pcl2/miniprotocol.rs`, covering protocol-owned term-bank initialization, id-indexed mini-step storage, decimal-digit-token protocol parsing (including zero), scanner-comment forwarding through explicit output-aware parsing, duplicate-id rejection, ordered PCL/TSTP rendering, recursive proof-precondition collection, fast/slow proof-step marking, bulk property mutation, extraction/deletion by id, and proof-clause printing with the C newline behavior.
- Rust stores owned steps in a safe `Vec<Option<PclMiniStep>>` indexed by mini id instead of a `PDArray_p` of raw pointers. Both provide constant-time indexed access and retain `max_ident` as a historical high-water mark after extraction/deletion. Rust borrowing prevents aliases from surviving mutation, drops each live step exactly once, and lets a missing lookup return without C's incidental `PDArrayElementP` enlargement.
- C's initial one-slot `PDArray` grows in blocks of 500,000 pointer slots as soon as id 1 is accessed. Rust grows geometrically with inserted ids, avoiding that fixed first expansion while retaining amortized insertion and direct-index lookup. Dense step values also avoid C's separate allocation per protocol entry.
- Scanner-comment forwarding is owned explicitly by `parse_with_output`; `epclextract --fast-extract --forward-comments` routes through it and has executable-level coverage for comment ordering.

### Change Later

- `PCLMiniProtInsertStep` returns false only when C receives the exact pointer already stored; a distinct pointer with the same id violates an assertion. Rust ownership makes reinserting a still-stored object impossible, so direct insertion rejects an id collision without replacing the live step and protocol parsing reports the duplicate. Raw-pointer reinsertion remains tracked by `E_Rust_Port-j76.4.949`.
- `PCLMiniProtParse` enters its parse loop only when the current token is `PosInt`, whose scanner meaning is a sequence of decimal digits and therefore includes zero, even though `PCLMiniStepParse` itself calls signed `ParseInt`. Rust preserves that exact gate: zero is parsed and negative input is left unconsumed. The naming/free-time contradiction remains tracked by `E_Rust_Port-j76.4.950` and `E_Rust_Port-j76.4.957`.
- The C parser echoes token comments to `GlobalOut` when `ignore_comments` is false. Rust preserves the observable executable behavior through an explicit session output owner and avoids a process-global sink; any API reconsideration remains tracked by `E_Rust_Port-j76.4.951`.
- `PCLMiniProtPrint` prints each stored mini step without adding separators or newlines, so adjacent rendered steps concatenate. Rust preserves that exact string shape; cleanup remains tracked by `E_Rust_Port-j76.4.952`.
- Fast proof marking starts only from the contiguous suffix of extract-marked steps ending at `max_ident` and stops at the first gap or non-extract step. Rust preserves this shortcut; broader seeding remains tracked by `E_Rust_Port-j76.4.953` and executable-level `E_Rust_Port-j76.4.1133`.
- `PCLMiniExprCollectPreconds` stores raw step pointers in a `PTree`. Rust instead deduplicates and sorts by mini id. Proof marking sets the visited property before expanding preconditions and observes only the final property set plus an order-independent empty-clause boolean, so pointer order is not semantic; future order-sensitive traversal remains tracked by `E_Rust_Port-j76.4.954`.
<!-- END MANUAL REVIEW: c_source_docs -->
