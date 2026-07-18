# Literal-selection and inference-option surface

## Status

Completed for Bead `E_Rust_Port-j76.2.46`. All 144 literal-selection strategy
names now have exact executable behavior against the unchanged first-order C
reference on the discriminating mixed-clause fixture. The CLI-to-
`HeuristicParmsCell` bridge and the advertised heuristic, generation,
paramodulation, splitting, disequality-decomposition, and completeness controls
also match their C executable surfaces.

## Defect and correction

The first exhaustive run matched 138/144 selectors. The six mismatches were the
standard `MinInfpos` family:

- `SelectMinInfpos`;
- `PSelectMinInfpos`;
- `HSelectMinInfpos`;
- `GSelectMinInfpos`;
- `SelectMinInfposNoTypePred`; and
- `PSelectMinInfposNoTypePred`.

C evaluates these selectors with `TermStandardWeight`, whose variable and
function-symbol weights are 1 and 2. Rust's shared standard-family helper had
instead passed 1 and 1 to the weighted implementation. On the retained fixture,
that tie selected a negative predicate rather than the pure-variable
disequality, suppressing C's one equality-resolution inference.

Both immutable- and mutable-term-bank standard helpers now pass
`DEFAULT_VWEIGHT` and `DEFAULT_FWEIGHT`. The separate `Min2Infpos` family keeps
its C-specific 2/1 tuple. Two focused regressions use a clause where the old
unit function weight and the standard function weight choose different
literals, covering all six affected names through both dispatch paths.

## Exhaustive selector comparison

[`compare_literal_selection.py`](compare_literal_selection.py) extracts the
advertised name arrays from C and Rust, asserts exact order, and runs every name
through the two optimized executables. Runs use deterministic FIFO clause
selection, no preprocessing, one processed clause, PCL proof output, and the
mixed predicate/equality fixture in [`selection.p`](selection.p).

The retained report is 30,999 bytes with SHA-256
`E04B4DEA15A0DFD51EC385EC0939DC3DD1A01609F7FA57D735723098E034976F`.
It records:

- 144/144 names in exact C table order;
- 144/144 exact normalized execution summaries;
- three distinct C behavior groups preserved; and
- empty stderr for every C and Rust process.

## Option-bridge comparison

[`compare_option_bridge.py`](compare_option_bridge.py) adds nine executable
cases. Eight compare complete `--print-strategy` output byte-for-byte after line
ending normalization, covering selection limits and inheritance, an expert
heuristic, `--no-generation` precedence, equality-factoring and negative-unit
paramodulation toggles, condensing, forward simplification, raw split-class
bitmasks, split method/aggression/definition reuse, disequality-decomposition,
and all four simultaneous-paramodulation modes. The ninth confirms that
`--assume-incompleteness` produces C's `GaveUp` status and exit behavior.

All 9/9 cases are exact. [`audit_option_surface.py`](audit_option_surface.py)
additionally pins 22/22 static C-owner, Rust bridge/consumer, and permanent-test
contracts.

## Reproduction

```powershell
C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-18-091-literal-selection-option-surface\audit_option_surface.py `
  --repo . `
  --output target\literal-selection-option-audit.json `
  --expected experiments\2026-07-18-091-literal-selection-option-surface\audit-reference.json

C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-18-091-literal-selection-option-surface\compare_literal_selection.py `
  --repo . `
  --c-exe /home/rober/.cache/e-rust-port/bin/17026b1bfe61aaf223cfaae54947c8d2679c31a0/fol/eprover `
  --rust-exe target\release\eprover.exe `
  --output target\literal-selection-surface.json `
  --expected experiments\2026-07-18-091-literal-selection-option-surface\comparison-reference.json

C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-18-091-literal-selection-option-surface\compare_option_bridge.py `
  --c-exe /home/rober/.cache/e-rust-port/bin/17026b1bfe61aaf223cfaae54947c8d2679c31a0/fol/eprover `
  --rust-exe target\release\eprover.exe `
  --output target\literal-selection-option-surface.json `
  --expected experiments\2026-07-18-091-literal-selection-option-surface\option-comparison-reference.json
```

## Validation

- selector execution: 144/144 exact;
- option-bridge execution: 9/9 exact;
- source/test audit: 22/22 contracts passed;
- two new standard-weight regressions and the retained `Min2Infpos` regression
  passed; and
- full suite, strict lint/format gates, documentation gates, optimized build,
  and vendored-C cleanliness are recorded in the completing commit.
