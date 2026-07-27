# Automatic Term-ordering Selection State

## Status

Completed for Bead `E_Rust_Port-j76.2.65`. The automatic-ordering parameter
initialization, candidate enumeration, scoring, selection, OCB construction,
and production owner bridge are implemented and covered. No production Rust
change was required, and the vendored C checkout remained unchanged.

## Question

Does Rust preserve the actual C `che_to_autoselect` initialization and complete
`OrderNextOrdering` search order, and is any legacy automatic-ordering entry
point reachable in production without a Rust owner?

## Direct C state

[`build_debug_reference.sh`](build_debug_reference.sh) copies the isolated
`ENABLE_LFHO` C source at commit
`17026b1bfe61aaf223cfaae54947c8d2679c31a0`, changes only build flags to retain
debug symbols, and compiles the copy. It does not edit a C source file or use
the vendored checkout as a build directory.

[`collect_reference.gdb`](collect_reference.gdb) calls the compiled C
`init_oparms` and `OrderNextOrdering` functions directly.
[`collect_autoselect_state.py`](collect_autoselect_state.py) retains the result
in [`reference_state.json`](reference_state.json). The initialized fields are:

| Field | C value |
| --- | ---: |
| ordering type | `KBO6` (`3`) |
| weight generator | `WSelectMaximal` (`1`) |
| precedence generator | `PUnaryFirst` (`1`) |
| constant weight | `WConstNoSpecialWeight` (`-1`) |
| three axiom/conjecture modifiers | `0, 0, 0` |
| literal comparison | `LCNormal` (`1`) |
| higher-order kind | `LFHO_ORDER` (`0`) |
| DB/lambda weights | `10, 20` |
| forced KBO variable weight | `false` |

The all-wildcard mask produces exactly 1,972 candidate states. KBO traverses
all three constant-weight states for each of 19 precedence generators and 34
weight generators; LPO then traverses the 34 weight generators while the
KBO-only fields remain reset. Candidate 1,938 is the KBO-to-LPO transition,
candidate 1,971 is the last LPO state, and the unsuccessful successor wraps to
the initial KBO state.

The retained sequence has SHA-256
`ED297091EEE60F18B7ABE249CB6C672595F22173DA6CE9EB6382455B9E6C4F70` and
FNV-1a-64 `8C884832231FE663`. A fresh run from the dedicated build reproduced the
entire JSON byte-for-byte (file SHA-256
`E5C6E8BAB674BCD30076D4E0C31C4355BD4F832B29DB34BDBADBBCEC7FFACDF6`).

## Rust result

The permanent `instrumented_c_reference_ordering_search_state_matches`
regression generates the same 1,972 indexed states with Rust's public
`OrderNext*` helpers and matches the direct C FNV digest and final wrapped
state. Existing tests separately pin:

- `init_oparms`, all 13 `AutoOrderingMode` variants, and their diagnostic text;
- maximal-term marking plus C's exact ordering-evaluation penalty sum, including
  the mutable-bank higher-order path;
- strict lower-score replacement, seed normalization, and selected-parameter
  retention in `order_find_optimal`;
- concrete and optimized `to_select_ordering`, original predefined strings,
  and `rewrite_strong_rhs_inst` propagation; and
- LPO/KBO OCB allocation, generated/predefined precedence, generated/overridden
  weights, literal-comparison validation, and the upstream RPO assertion.

[`audit_autoselect.py`](audit_autoselect.py) records seven paired source and
regression checks in [`source-audit.json`](source-audit.json); all pass.

## Production reachability

All 13 C `generate_auto*ordering` functions are definition-only: no other C
translation unit calls them. The C command-line assignment for `OPTIMIZE_AX` is
commented out, and both C and Rust reject `-t Auto`/`-t Optimize`. Thus the
unconstrained optimizing branch, which can enumerate upstream's unimplemented
`POrientAxioms` candidate, is not a production executable path.

C has one production `TOSelectOrdering` caller, `ProofControlInit`. The earlier
owner reconciliation proves the Rust clause and clause/formula proof-control
initializers store the selected OCB at the same point, with representative FOL
and THF runs byte-exact against C. The broader executable term-ordering matrix
is 73/73 exact.

The C AutoCASC/AutoDev helpers initialize only part of a stack-local parameter
cell. Because those helpers are dormant and the remaining bytes are
indeterminate, there is no stable accidental value to reproduce. Rust's common
mode helper starts from a fully initialized cell and then preserves every
visible C assignment.

## Reproduction

```powershell
wsl -d Ubuntu-24.04 -- sh `
  /mnt/c/Users/rober/Code/E_Rust_Port/experiments/2026-07-17-073-autoselect-state/build_debug_reference.sh `
  /home/rober/.cache/e-rust-port/src/17026b1bfe61aaf223cfaae54947c8d2679c31a0-ho-20260717b `
  /home/rober/.cache/e-rust-port/src/17026b1bfe61aaf223cfaae54947c8d2679c31a0-ho-autoselect-debug-20260718e

python experiments\2026-07-17-073-autoselect-state\collect_autoselect_state.py `
  --c-exe /home/rober/.cache/e-rust-port/src/17026b1bfe61aaf223cfaae54947c8d2679c31a0-ho-autoselect-debug-20260718e/PROVER/eprover-ho `
  --expected experiments\2026-07-17-073-autoselect-state\reference_state.json `
  --output C:\tmp\autoselect-reference.json --quiet

python experiments\2026-07-17-073-autoselect-state\audit_autoselect.py `
  --output C:\tmp\autoselect-source-audit.json

cargo test --locked --all-features instrumented_c_reference_ordering_search_state_matches
```

## Compatibility decision

The direct candidate state, safe initialization policy, scoring and selection
order, concrete OCB construction, and sole production owner are covered. The
disabled upstream Optimize CLI and dormant legacy generators do not represent
missing drop-in behavior. This migrated item describes completed behavior
rather than remaining port work.
