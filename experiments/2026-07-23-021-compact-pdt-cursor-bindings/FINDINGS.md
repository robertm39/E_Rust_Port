# Experiment 259: Compact PD-tree cursor bindings

## Status

Rejected in Experiment 259 for Bead `E_Rust_Port-j76.5.3`.

## Hypothesis

The accepted first-order PD-tree cursor costs 1,581,288,798 exclusive
instructions. Each speculative variable binding stores a `usize` variable-child
index and a `usize` processed-query index, making the record 16 bytes on the
maintained 64-bit targets. The variable-child arena is already addressed by
packed `u32` links, and the query-step index is operation-local.

Store both indices as checked `u32` values, reducing the binding record to
eight bytes. Preserve binding order, reverse repeated-variable lookup,
substitution construction, frame binding positions, query traversal, and all
tree/search representations.

This is distinct from Experiment 181, which packed two hot traversal-frame
positions and regressed; the candidate changes only the separate binding
vector whose records are scanned and moved as a unit.

## Baseline

Accepted Experiment 245:

- Rust instructions: 9,898,434,766
- C instructions: 5,254,361,329
- Rust/C ratio: 1.883851
- accepted first-order cursor: 1,581,288,798 exclusive instructions

## Candidate

Store both binding indices as checked `u32` values. Add an explicit 8-byte
layout regression and convert to `usize` only at variable-edge and query-step
access boundaries.

## Validation

- All 41 focused PD-tree tests pass, including binding order, repeated
  variables, live substitution construction, deletion/reuse, constraints, and
  higher-order paths.
- The layout regression confirms an 8-byte binding record.
- Strict library pedantic Clippy, formatting, and `git diff --check` pass.
- The exact LUSK6 profile proves `Unsatisfiable` and exits zero.

## Measurement

The candidate retires 9,967,460,870 instructions, 69,026,104 above the
9,898,434,766-instruction parent. This is a 0.697344% whole-prover regression,
and the hypothetical Rust/C ratio worsens from 1.883851 to 1.896988.

The intended first-order cursor itself falls from 1,581,288,798 to
1,570,265,878 exclusive instructions:

- delta: -11,022,920;
- cursor-local improvement: -0.697085%;
- work outside the cursor rises by 80,049,024 instructions.

The smaller record therefore improves the direct owner, but its checked
conversion and changed optimized layout produce a much larger whole-program
loss. The exact global gate governs acceptance.

## Result

Reject. Restore full-width binding indices and remove the candidate layout
test. Native timing and compatibility/resource matrices are skipped after the
deterministic whole-program regression. Accepted Experiment 245 remains the
baseline at 9,898,434,766 instructions, or 1.883851 times C.

The raw candidate profile is preserved at:

```text
.artifacts/experiments/2026-07-23-021-compact-pdt-cursor-bindings/rust-callgrind-compact-pdt-cursor-bindings.out
```

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-compact-pdt-cursor-bindings.out \
  target-wsl-259-compact-pdt-cursor-bindings/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```
