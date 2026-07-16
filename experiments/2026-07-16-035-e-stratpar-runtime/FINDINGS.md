# e_stratpar runtime compatibility

## Status

Completed for Bead `E_Rust_Port-j76.1.26`. The vendored C source remained
unchanged. Real satisfiable and unsatisfiable problem files now exercise the
native Rust executable, and a caller-level process-set regression pins the
same-poll output tie rule.

## Reference availability

This Windows host has no `gcc`, `clang`, or `make`, no installed WSL
distribution, and no prebuilt C `eprover` or `e_stratpar`. A direct C executable
run is therefore unavailable. The checked-in paired C sources are sufficient
to determine the current runtime boundary without guessing:

- `PROVER/e_stratpar.c:116-123` constructs each child with
  `-xAutoSchedN -tAutoSchedN --sine` and reads the PID line during
  `ECtrlCreateGeneric`;
- `PROVER/eprover.c:1615-1655` accepts only `LPO`, `LPOCopy`, `LPO4`,
  `LPO4Copy`, `KBO`, and `KBO6` for `-t`, then emits the same usage diagnostic
  Rust emits for every `AutoSchedN` value;
- `CONTROL/cco_proc_ctrl.c:217-239` treats EOF before a PID line as
  `Cannot read eprover PID line` with `OTHER_ERROR` (exit 11).

Thus a same-tree C build cannot reach the eight-child poll loop: its first
child rejects `-tAutoSched0`, writes no PID to captured stdout, and the wrapper
exits 11. This is a stale CASC-2017 command contract in the current source, not
a missing Rust search result.

## Native real-problem evidence

The integration test puts the just-built Rust `eprover` directory first on
`PATH` and runs `e_stratpar --cpu-limit=10` on both:

```tptp
cnf(false_axiom, axiom, ($false)).
cnf(unit_axiom, axiom, p(a)).
```

Both runs exit 11 with empty stdout and exactly these diagnostics:

```text
eprover: Option -t (--term-ordering) requires LPO, LPO4, KBO or KBO6 as an argument
e_stratpar: Cannot read eprover PID line
```

The optional second positional argument cannot select a corrected prover: C
accepts it but leaves `prover` fixed to `"eprover"`, and Rust preserves that
quirk. An older compatible `eprover` can still be selected by `PATH`; existing
injected-child tests cover the intended proof, no-proof, cleanup, and output
flush behavior without misrepresenting it as current same-tree runtime
evidence.

## Same-poll output ordering

`CONTROL/cco_proc_ctrl.c:477-524` calls `select`, scans descriptors from zero
through `maxfd`, and overwrites `res` for every theorem/unsatisfiable EOF. When
multiple proofs terminate in the same polling window, the highest ready file
descriptor wins. Failure messages from lower descriptors are emitted during
that same ascending scan.

Rust stores controls in a `BTreeMap`, scans ready descriptors in ascending
order, continues after successful EOF, and likewise overwrites the selected
descriptor. The new `e_stratpar` caller-level regression presents successful
descriptors 2 and 7 in one ready set and proves that only descriptor 7's output
is replayed. The lower-level controller regression continues to cover an
interleaved failure and its ordered `% No proof found by ...` message.

## Compatibility decision

The Rust port preserves the current C failure rather than silently deleting
`-tAutoSchedN` or accepting an ordering value the reference command-line parser
rejects. Restoring the intended eight-way same-tree prover run would require an
explicit compatibility decision spanning both executables and new reference
expectations. This slice closes the runtime-reference item by making the actual
current boundary and the otherwise unreachable poll tie behavior permanent and
visible.

## Performance decision

No benchmark is warranted. Production exits while creating the first child,
and the test-only poll injection changes no runtime code path beyond factoring
the existing poll call behind a monomorphized closure.

## Validation

- `cargo test --locked --lib --quiet`: 4,123 passed, including the
  simultaneous-proof caller regression
- `cargo test --locked --bins --quiet`: all binary targets passed
- `cargo test --locked --test e_stratpar --quiet`: 1 passed (two real problem
  files)
- `cargo test --locked --test eprover_schedule --quiet`: 4 passed
- `cargo test --locked --test executable_inventory --quiet`: 1 passed
- `cargo fmt --all -- --check`
- `cargo check --locked --all-targets`
- `cargo clippy --locked --all-targets -- -D warnings`
- C-source coverage: 492 source files covered by 266 unit docs
- Change Later wording: 269 Markdown files checked
- local Markdown links: 269 Markdown files checked
- regeneration preservation: manual sections preserved in 268 Markdown files
- `git diff --check`
