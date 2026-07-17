# Full PCL-step ownership and shell-mode audit

## Status

Completed for Bead `E_Rust_Port-j76.2.132`. Rust's safe `PclStep`
representation preserves the observable ownership, rendering, and executable
shell-mode behavior of C without retaining unchecked union state or borrowed
back-pointers. The vendored C source remained unchanged.

## C behavior

`PCLStepCell` stores a borrowed `TB_p`, then an untagged union containing either
an independently allocated `Clause_p` or a term-bank-owned `TFormula_p`.
`PCLStepFree` chooses the active arm from property bits: it frees clausal
content, leaves formulas to term-bank garbage collection, and skips both for a
shell step. Printers use the stored bank to recover symbol and type context.

`SupportShellPCL` is process-global and defaults to false. The C tree contains
one executable assignment: `epclextract` sets it true. Other current PCL tools
therefore parse with the false default.

## Rust equivalence

`PclStepLogic` makes `Shell`, `Clause(Box<Clause>)`, and `Formula(Term)` mutually
exclusive. The box matches C's separate clause allocation and keeps the clause
address stable even when a `Vec<PclStep>` grows and relocates its step values.
RAII drops exactly the active owned variant; a formula's shared term handle
continues to use term-bank-managed sharing.

`PclProtocol` owns the `TermBank` that C lends to every step. Parsing and
printing receive that owner explicitly, so the same signature and shared terms
are available without a dangling `TB_p` if a movable Rust step outlives its
original container position.

`PclStepParseOptions::support_shell_pcl` replaces the global with a per-parse
choice. Executable call sites preserve C's effective matrix: `epclextract`
passes true, while `checkproof`, `epcllemma`, `epclanalyse`, `direct_examples`,
and `ekb_ginsert` pass false. A regression performs disabled, enabled, then
disabled shell parses in one process and confirms that the enabled parse does
not leak into either neighbor.

All previously recorded output quirks remain covered: the compiled property
bits rather than the stale `PCLType1` comment, the omitted `que` diagnostic,
the axiom/initial TSTP-role distinction, the wider full-step extra token set,
the missing formula TPTP period, and shell omission plus warning output.

## Performance

The representation introduces no allocation beyond C's separately allocated
clause object. Explicit bank and option parameters are pointer-sized/copy-sized
arguments and avoid global synchronization; parsing and rendering algorithms
are unchanged.

## Validation

- focused `pcl2::steps` tests cover 20 property, parsing, ownership, and output
  cases, including clause-address stability and shell-mode isolation;
- executable-source inspection confirms the C/Rust shell-mode matrix;
- formatting and strict Clippy pass;
- full all-target/all-feature tests pass; and
- source-document coverage, Change Later wording, links, and regeneration
  preservation pass.
