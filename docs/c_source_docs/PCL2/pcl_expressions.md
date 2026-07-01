<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# PCL2 / pcl_expressions

## Source Files

- [PCL2/pcl_expressions.h](../../../eprover/PCL2/pcl_expressions.h)
- [PCL2/pcl_expressions.c](../../../eprover/PCL2/pcl_expressions.c)

## Purpose

PCL2 expressions and uexpressions. the GNU Lesser General Public License.

Within the source tree, this unit belongs to `PCL2`. PCL protocol and proof-object support: proof steps, mini-protocols, identifiers, positions, expressions, checking, lemmas, and proof analysis.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `PCLExprCell`
- `PCLExpr_p`
- `PCLOpcodes`

### Macros And Constants

- `PCLExprArg(expr,i)`
- `PCLExprArgInt(expr,i)`
- `PCLExprArgPos(expr,i)`
- `PCLExprCellAlloc()`
- `PCLExprCellFree(junk)`
- `PCLFullExprParse(in)`
- `PCLFullExprPrint(out, expr)`
- `PCLFullExprPrintTSTP(out, expr)`
- `PCLMiniExprParse(in)`
- `PCLMiniExprPrint(out, expr)`
- `PCLMiniExprPrintTSTP(out, expr)`
- `PCL_EXPRESSIONS`
- `PCL_OP_ACRESOLUTION_WEIGHT`
- `PCL_OP_CLAUSENORMALIZE_WEIGHT`
- `PCL_OP_CONDENSE_WEIGHT`
- `PCL_OP_CONTEXTSIMPLIFYREFLECT_WEIGHT`
- `PCL_OP_EFACTORING_WEIGHT`
- `PCL_OP_ERESOLUTION_WEIGHT`
- `PCL_OP_EVALGC_WEIGHT`
- `PCL_OP_INITIAL_WEIGHT`
- `PCL_OP_NOOP_WEIGHT`
- `PCL_OP_PARAMOD_WEIGHT`
- `PCL_OP_QUOTE_WEIGHT`
- `PCL_OP_REWRITE_WEIGHT`
- `PCL_OP_SIMPLIFYREFLECT_WEIGHT`
- `PCL_OP_SIM_PARAMOD_WEIGHT`
- `PCL_OP_SPLITCLAUSE_WEIGHT`
- `PCL_OP_UREWRITE_WEIGHT`
- `PCL_VAR_ARG`

### Globals

- None found in the source scan.

### Exported Functions

- `PCLExpr_p PCLExprAlloc(void)`
- `PCLExpr_p PCLExprParse(Scanner_p in, bool mini)`
- `bool PCLStepExtract(char* extra)`
- `void PCLExprFree(PCLExpr_p junk)`
- `void PCLExprPrint(FILE* out, PCLExpr_p expr, bool mini)`
- `void PCLExprPrintTSTP(FILE* out, PCLExpr_p expr, bool mini)`
- `void PCLMiniExprFree(PCLExpr_p junk)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `PCLExprAlloc`: Allocate an initialized PCL-expression-cell
- `PCLExprFree`: Free a PCL-expr-cell.
- `PCLMiniExprFree`: Free a PCL Mini-Expression.
- `PCLExprParse`: Parse a PCL-expression or Mini-expression
- `PCLExprPrint`: Print a PCL expression.
- `PCLExprPrintTSTP`: Print a PCL expression in TSTP format.
- `PCLStepExtract`: Given a PCL step "extra" string, return true if this should be the root of a proof tree for extraction. Implemented here, because it is used by both steps and ministeps.

### Dependencies

- `"pcl_expressions.h"`
- `<ccl_clauseinfo.h>`
- `<ccl_clausesets.h>`
- `<ccl_inferencedoc.h>`
- `<pcl_idents.h>`
- `<pcl_positions.h>`

### Compile-Time Conditions

- `PCL_EXPRESSIONS`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Clause/literal mutation affects indexing, derivation, and proof reconstruction; preserve update ordering.
- Parser routines usually advance scanner state and may report fatal errors; keep token-consumption behavior exact.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `PCL2/pcl_expressions.h`, `PCL2/pcl_expressions.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `PCL2` covering 2 source file(s), about 974 lines, 10 scanned public declarations, 0 scanned internal function definitions, and 7 structured function-comment blocks.
- PCL2 expressions and uexpressions. the GNU Lesser General Public License.
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

### Rust Port Status Notes

- `src/pcl2/expressions.rs` ports the core `PCLExprAlloc`, `PCLExprParse`, `PCLExprPrint`, `PCLExprPrintTSTP`, and `PCLStepExtract` behavior for quote, initial, and operator expression trees.
- The Rust representation stores expression arguments as typed child expressions plus optional `PCL2` positions instead of the C two-slot `PDArray` layout, while preserving the opcode discriminants, mini/full quote distinction, fixed-arity checks, variable-arity one-or-more operators, and PCL/TSTP rendering spellings.
- Initial source-info expressions reuse the ported `ClauseInfo` renderer; source strings are stored without the surrounding double quotes while the name field preserves the scanner literal, matching the C `DStrCopyCore`/`DStrCopy` split.

### Change Later

- `PCLExprParse` tests for `OpenBracket` before calling `PCL2PosParse`, but `PCL2PosParse` itself expects a positive integer as the current token and does not consume the opening bracket. Rust preserves this parser mismatch, so expression-level positions are still not accepted through this path. Revisit only after reference PCL traces clarify whether this is dead code, a C bug, or a syntax expectation hidden elsewhere.
- `PCLExprPrintTSTP` ignores stored argument/quote positions even though PCL rendering prints them. Rust mirrors that split; cleaned proof-object APIs should decide whether position metadata belongs in TSTP annotations or only in PCL syntax.
- `PCLOpURewrite` exists in the opcode enum and has a weight constant, but this unit neither parses nor prints it. Rust keeps the discriminant for compatibility and leaves behavior absent until a caller or source trace requires it.
- Variable-arity operators such as `cdclpropres` and `ar` still require at least one child because the C parser enters the argument parser whenever `arg_no` is `PCL_VAR_ARG` (`-1`). If empty argument lists should be legal, change that only after proof-output compatibility is established.
<!-- END MANUAL REVIEW: c_source_docs -->
