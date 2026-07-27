<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# PCL2 / pcl_miniclauses

## Source Files

- [PCL2/pcl_miniclauses.h](../../../eprover/PCL2/pcl_miniclauses.h)
- [PCL2/pcl_miniclauses.c](../../../eprover/PCL2/pcl_miniclauses.c)

## Purpose

Maximal compact representation for clauses, to be used in compact pcl listings (and possibly wherever elese needed). Adaptded from can_clausestore.h. the GNU Lesser General Public License.

Within the source tree, this unit belongs to `PCL2`. PCL protocol and proof-object support: proof steps, mini-protocols, identifiers, positions, expressions, checking, lemmas, and proof analysis.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `MiniClauseCell`
- `MiniClause_p`

### Macros And Constants

- `MiniClauseCellAlloc()`
- `MiniClauseCellFree(junk)`
- `PCL_MINICLAUSES`

### Globals

- None found in the source scan.

### Exported Functions

- `(MiniClauseCell*)SizeMalloc(sizeof(MiniClauseCell)) SizeFree(junk, sizeof(MiniClauseCell)) void MiniClauseFree(MiniClause_p clause)`
- `Clause_p MiniClauseToClause(MiniClause_p clause, TB_p bank)`
- `Clause_p UnMinifyClause(MiniClause_p clause, TB_p bank)`
- `MiniClause_p ClauseToMiniClause(Clause_p clause)`
- `MiniClause_p MinifyClause(Clause_p clause)`
- `void MiniClausePCLPrint(FILE* out, MiniClause_p compact, TB_p bank)`
- `void MiniClausePrint(FILE* out, MiniClause_p compact, TB_p bank, bool full_terms)`
- `void MiniClauseTSTPCorePrint(FILE* out, MiniClause_p compact, TB_p bank)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `MiniClauseFree`: Release the memory taken by a compact clause.
- `MiniClauseAddTerms`: Add the terms in term_clause to clause.
- `ClauseToMiniClause`: Generate a compact clause represention from the normal one.
- `MiniClauseToClause`: Given a compact clause, create the corresponding normal clause.
- `MinifyClause`: As ClauseToMiniClause(), but destroy original.
- `UnMinifyClause`: As MiniClauseToClause(), but destroy original.
- `MiniClausePrint`: Print a compact clause. Not the best way, but the easiest!
- `MiniClausePCLPrint`: Print the clause in PCL format, i.e. as a literal list.
- `MiniClauseTSTPCorePrint`: Print the core clause in TSTP format, i.e. as a literal list.

### Dependencies

- `"pcl_miniclauses.h"`
- `<ccl_clauses.h>`

### Compile-Time Conditions

- `PCL_MINICLAUSES`

## E Reference Notes

- Use the ownership model visible in this unit's allocation/free helpers and exported APIs as evidence; preserve it only where correctness, supported compatibility, or measured performance requires it.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions reveal performance assumptions; measure allocation granularity and reuse before choosing Umlaut's representation.
- Determine which term-sharing and term-bank properties are semantic constraints and which are replaceable memory optimizations.
- Audit where clause/literal mutation order affects indexing, derivation, proof reconstruction, or deterministic behavior before changing it.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22; updated for compact ownership, count width, and rendering equivalence on 2026-07-17.

Source files reviewed: `PCL2/pcl_miniclauses.h`, `PCL2/pcl_miniclauses.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `PCL2` covering 2 source file(s), about 378 lines, 10 scanned public declarations, 0 scanned internal function definitions, and 9 structured function-comment blocks.
- Maximal compact representation for clauses, to be used in compact pcl listings (and possibly wherever elese needed). Adaptded from can_clausestore.h. the GNU Lesser General Public License.
- Proof-object code. Preserve identifier handling, step structure, protocol syntax, and proof-checking side effects.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Parser functions usually consume input and report fatal diagnostics on mismatch; exact token flow matters for compatibility.
- Proof output/checking code is externally consumed; preserve identifiers, step ordering, and formatting details.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Rust Port Status

- Initial Rust support is in `src/pcl2/miniclauses.rs`, covering compact literal snapshots, conversion from and back to ordinary clauses, owned minify/unminify wrappers, explicit `MiniClausePrint` LOP/TPTP/TSTP rendering, and PCL/TSTP-core rendering through temporary rebuilt clauses.
- Rust stores one vector of owned `MiniLiteral` values instead of C's separately allocated `short` sign array and borrowed two-`Term_p` array. Each cloned `Term` is a shared identity handle, so the snapshot preserves the exact banked term cells after the source clause is freed while preventing dangling pointers and double frees.
- The vector length is the authoritative `usize` literal count. It cannot disagree with storage or truncate at C's signed-`short` boundary; focused coverage retains 32,768 literals. Emulating C's overflow would only recreate invalid allocation/loop bounds and has no valid serialized effect.
- Mini clauses capture only literal signs and term pairs. Clause-level properties are not preserved because the C `properties` field is commented out and `MiniClauseToClause` creates a fresh `ClauseAlloc` result. Rust regression coverage requires reconstruction to return `CPIgnoreProps`/unknown role even when the source was a negated conjecture.
- The Rust printer methods intentionally rebuild a full `Clause` before calling the already-ported clause printers, matching the C implementation's simple temporary-clause strategy. Output format, problem type, and equation-print options are explicit call arguments; repeated LOP/TPTP/LOP rendering proves the selection cannot leak process-global state.

### Change Later

- `MiniClauseCell.literal_no` is a C `short` even though it is assigned from `ClauseLiteralNumber`; very large clauses can truncate or overflow. Rust intentionally retains the full safe count; any attempt to reproduce the invalid boundary remains tracked by `E_Rust_Port-j76.4.943`.
- The compact representation drops role/source/indexing metadata exactly like C. Whether a future proof format should retain it remains tracked by `E_Rust_Port-j76.4.944`.
- C borrows raw `Term_p` pointers and relies on the term bank outliving every mini clause. Rust keeps term cells alive with shared handles while the owning mini protocol supplies the rendering bank; alternative lifetime modeling remains tracked by `E_Rust_Port-j76.4.945`.
- `MiniClausePrint`, `MiniClausePCLPrint`, and `MiniClauseTSTPCorePrint` rebuild a complete `Clause` just to print it. Rust retains that compatibility path; a measured direct compact printer remains tracked by `E_Rust_Port-j76.4.946`.
- `MiniClausePrint` reaches C's process-global output format and problem type. Rust's explicit controls preserve per-call behavior without leakage; any global-state emulation remains tracked by `E_Rust_Port-j76.4.947`.
- `MiniClauseAddTerms` duplicates most of `ClauseToMiniClause`, is not declared in the header, and has no callers in the vendored tree. Rust factors the literal-copy path instead of exposing dead surface; reconsideration remains tracked by `E_Rust_Port-j76.4.948`.
<!-- END MANUAL REVIEW: c_source_docs -->
