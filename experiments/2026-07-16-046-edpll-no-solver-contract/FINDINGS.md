# edpll no-solver compatibility contract

## Status

Completed for Bead `E_Rust_Port-j76.1.38`. The drop-in port intentionally
preserves the reference executable's incomplete parser/state-shell behavior.
Implementing a real SAT/DPLL result mode would be a new product feature rather
than completion of the referenced C behavior. The vendored C source remained
unchanged.

## Executable call-path audit

After parsing all clauses, `PROVER/edpll.c` performs exactly one DPLL action:

```c
dpllstate = DPLLStateAlloc(form);
```

It does not call `DPLLAssignVar`, unit propagation, branching, retraction, or a
solve loop. The state is immediately freed in the normal non-`FAST_EXIT` build,
the output stream is closed, and the process returns zero. The help text itself
labels the program `Not completed yet!`; `--dimacs` merely sets an otherwise
unused global.

## Low-level DPLL audit

`PROPOSITIONAL/cpr_dpll.c` is also incomplete:

- `DPLLStateAlloc` populates assignment/deactivation/unit/open-atom containers;
- `DPLLStateFree` releases them;
- `deactivate_clauses` and `shorten_clauses` both return zero without mutation;
- `DPLLAssignVar` therefore records an assignment but reports false; and
- `DPLLRetractLastAss` is declared in the header but has no definition.

There is no missing executable call to an otherwise completed solver. A real
driver would first require defining algorithm semantics absent from the C
reference, including propagation, conflict handling, retraction, branching,
model/result rendering, and the advertised DIMACS mode.

## Compatibility decision

The compatibility executable remains a clause parser and `DPLLState`
constructor. Rust already mirrors the implemented low-level state allocation
and the current assignment stub result. It must not infer a SAT/UNSAT result,
invoke the repository's other SAT backends, or make `--dimacs` observable,
because each choice would change reference stdout and potentially exit status.

The permanent comparison matrix now includes contradictory positive and
negative unit clauses:

```text
p.
<- p.
```

The exact contract is success exit 0, empty stderr, and trace-only stdout:

```text
New clause: p<-....accepted
New clause: <-p....accepted
```

No satisfiability line is printed despite the obvious contradiction. The Rust
unit regression and `run_native.py` pin all four properties. When a real driver
is desired after drop-in compatibility, it should be exposed deliberately as a
new mode or executable and tracked by the existing post-compatibility
`Change Later` items rather than silently changing `edpll`.

## Reference availability

This desktop session has no installed WSL distribution, cached C executable,
or native POSIX C toolchain, so the newly added contradictory-unit case was not
rerun against C. The conclusion rests on the complete executable call-path and
low-level implementation audit, while experiment 045 records real byte-equal C
and Rust LOP trace output. The new case is now permanent and will run in the
normal comparison command when a reference is restored:

```powershell
.\e-interop.ps1 compare-tools -RustBinDir .\target\release -Tool edpll
```

## Validation

- `cargo test --locked --lib prover::edpll::tests --quiet -- --test-threads=1`:
  23 passed;
- `cargo test --locked --lib --quiet -- --test-threads=1`: 4,141 passed;
- all binary targets passed under `cargo test --locked --bins`;
- integration targets `eprover_schedule`, `e_stratpar`, and
  `executable_inventory`: 4, 1, and 1 passed;
- `cargo check --locked --all-targets`: passed;
- `cargo clippy --locked --all-targets -- -D warnings`: passed;
- `cargo fmt --all -- --check`: passed;
- `cargo build --locked --release --bin edpll`: passed;
- bundled-Python `unittest` discovery under `tools/e-interop`: 32 passed;
- experiment 045's historical 14-case optimized matrix: passed;
- contradictory-unit optimized executable probe: passed;
- C-source documentation coverage: 492 sources / 266 unit docs;
- Change Later and local-link checks: 269 Markdown files each; and
- regeneration preservation: 268 Markdown files.
