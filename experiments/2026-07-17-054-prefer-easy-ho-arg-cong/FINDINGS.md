# PreferEasyHO ArgCong reconciliation

## Status

Completed for Bead `E_Rust_Port-j76.2.84`. The migrated gap is already closed
in production code and permanent regressions; no Rust implementation change was
needed, and the vendored C checkout remained unchanged.

## C contract

`PrioFunPreferEasyHO` starts at `PrioNormal`. When the process problem type is
`PROBLEM_HO`, it walks the clause derivation stack, masks each derivation code
through `DPOpGetOpCode`, and returns `PrioBest` as soon as the opcode is
`DOArgCong`. The raw stack walker skips argument slots according to the
derivation-code argument bits.

If no ArgCong entry is found, C computes several formula/non-pattern
preferences and then discards them because `prio = PrioPrefer ? PrioNormal :
PrioDefer` tests the constant rather than the computed value. The observable
non-ArgCong result is therefore `PrioNormal`.

## Rust reconciliation

Rust represents `DC_ARG_CONG` as `DO_ARG_CONG | ARG1_CNF | ARG_IS_HO` and stores
tagged derivation entries instead of C's interleaved raw `PStack`. Higher-order
ArgCong generation calls `set_ho_generation_proof_object` with `DC_ARG_CONG`, so
the generated clause contains the operation followed by its exact clause-parent
reference.

`prio_fun_prefer_easy_ho` checks the same process problem-type boundary and uses
`op_code(entry)` to compare the masked operation with `DO_ARG_CONG`. It returns
`PRIO_BEST` (`0`) only for that higher-order case and `PRIO_NORMAL` (`40`) for
the same derivation under unset/first-order modes and for non-ArgCong clauses.

This is the same semantic traversal as C without exposing raw derivation-stack
slot arithmetic through the Rust API.

## Focused evidence

- `prefer_easy_ho_detects_arg_cong_derivation_only_for_higher_order`: passed;
- both `proof_state_generate_new_clauses_higher_order_arg_cong_*` tests: passed;
- the generation tests pin `DC_ARG_CONG`, exact parent identity, inherited proof
  depth/size/SOS metadata, prefix application, shared fresh arguments, and the
  `MaxLits` filter; and
- the unchanged-production baseline from the immediately preceding slice is
  4,260 all-feature library tests plus every target, strict pedantic Clippy,
  release build, formatting, and all C-source documentation gates.

## Residual scope

C's discarded non-ArgCong preference calculation remains under
`E_Rust_Port-j76.3.567` and `.4.817`. The raw derivation-stack layout walk
remains under `.4.818`. Those are post-compatibility cleanup/representation
questions and do not block the represented ArgCong priority contract.
