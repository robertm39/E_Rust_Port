# GEO288 Search Divergence

## Question

Why does the C reference prove `GEO288+1.p` in a few seconds while the Rust
port reaches its 60-second CPU limit, and what causes the first selected-clause
divergence?

## Setup

- C reference: `/home/rober/.cache/e-rust-port/bin/17026b1bfe61aaf223cfaae54947c8d2679c31a0/fol/eprover`
- Rust executable: `target/release/eprover`
- Problem: `eprover/EXAMPLE_PROBLEMS/TPTP/GEO288+1.p`
- Shared arguments: `--auto --output-level=1 --cpu-limit=60 --memory-limit=2048 --detsort-rw --detsort-new`

Primary commands, run from the repository root:

```bash
wsl -d Ubuntu-24.04 -- bash experiments/2026-07-12-007-geo288-search/run-traces.sh
wsl -d Ubuntu-24.04 -- gdb -q -batch \
  -x experiments/2026-07-12-007-geo288-search/trace-c-hcb-calls.gdb \
  /home/rober/.cache/e-rust-port/bin/17026b1bfe61aaf223cfaae54947c8d2679c31a0/fol/eprover
wsl -d Ubuntu-24.04 -- gdb -q -batch \
  -x experiments/2026-07-12-007-geo288-search/capture-c-hidden-clause.gdb \
  /home/rober/.cache/e-rust-port/bin/17026b1bfe61aaf223cfaae54947c8d2679c31a0/fol/eprover
```

The bounded Rust HCB/generation/admission traces used temporary `eprintln!`
instrumentation that was removed before validation. The reusable C GDB and
trace-analysis scripts remain in this folder.

## Results

The initial run had these outcomes:

- C: theorem, about 3.48 seconds wall time, 154240 KiB peak RSS.
- Rust: resource out, about 60.24 seconds wall time, 323872 KiB peak RSS.
- Selected clauses matched structurally through ordinal 311.

The first 800 HCB calls matched exactly. Around the first visible divergence,
C selected clauses as follows:

```text
call 1241: clause 3610, current_eval=4, select_count=8
call 1242: clause 3656, current_eval=4, select_count=9
call 1243: clause 3608, current_eval=4, select_count=10
```

Rust instead selected sibling clause 3609 at call 1242. C clause 3656 is a
plain paramodulant with derivation `spm(3610, 298)`. Rust generated the same
clause with the same runtime ID, structure, and parents, but removed it during
generated-clause admission because both `ForwardModifyClause` and the direct
triviality test classified it as trivial.

Clause 3656 has 5 positive and 11 negative literals. Its complementary
`ron(X2,X4)` and `ron(X2,X5)` pairs share exact term pointers and term-bank
entry numbers in C. Nevertheless, a direct GDB call to C `ClauseIsTrivial`
returned false. At 16 literals, C crosses `EQN_LIST_LONG_LIMIT` and uses
`EqnLongListIsTrivial`; that routine inherits `PStackBinSearch`'s off-by-one
updates and can skip shared syntax keys. Rust had used a correct `BTreeSet`
intersection and therefore did not preserve this false negative.

Rust now ports the exact sorted-stack and `PStackBinSearch` state machine. A
focused 16-literal regression proves that the pairwise short-list algorithm
finds the complement while the C-compatible long-list algorithm misses it.

After the correction:

- C: theorem, about 2.64 seconds wall time, 154240 KiB peak RSS.
- Rust: resource out, about 58.18 seconds wall time, 325780 KiB peak RSS.
- Selected clauses match structurally through ordinal 561.
- The next visible mismatch is ordinal 562; C has one additional hidden
  selection between visible ordinals 558 and 559.

The full 50-case interop comparison at
`.artifacts/e-compare/20260713-022515-636229/` reported 5 mismatches, down from
6 in `.artifacts/e-compare/20260712-230220-753122/`. `LUSK6ext.lop` gained
normalized-output parity; the remaining mismatches were `BOO020-1.p`,
`GEO288+1.p`, `HEN011-2.p`, `sledgehammer.p`, and the synthetic CPU-limit
case.

The five-run benchmark at
`.artifacts/e-compare/20260713-024046-594917-benchmark/` reported an aggregate
Rust/C wall-time ratio of 3.500x, compared with 3.377x in the prior benchmark
at `.artifacts/e-compare/20260712-231637-524521-benchmark/`. The ratio increase
does not identify a workload regression: Rust's median wall time decreased on
every behavior-matching case. In particular, `LUSK6.lop` decreased from 3.36
to 2.73 seconds and `LUSK6ext.lop` from 7.65 to 6.31 seconds. C medians also
decreased, and the aggregate includes several sub-10-millisecond cases whose
Rust/C ratios are sensitive to process-startup noise.

## Raw Artifacts

Generated artifacts are under the ignored directory
`.artifacts/experiments/2026-07-12-007-geo288-search/`:

- `c-trace.txt` and `rust-trace.txt`: latest full selected-clause traces.
- `c-hcb-calls.txt` and `rust-hcb-calls.txt`: bounded selector call histories.
- `c-hidden-clause-3656.txt`: C clause 3656 and derivation capture.
- `rust-parent-3610-generation.txt`: Rust clauses generated from parent 3610.
- `rust-clause-3656-admission.txt`: Rust admission decision for clause 3656.

## Falsification Checks

- Compared initial HCB evaluations for sibling clauses 3608 through 3610; C
  and Rust weights, priorities, and FIFO counts matched.
- Compared the first 800 HCB calls exactly before extending the trace to the
  first mismatch.
- Captured C clause 3656 at selection and confirmed its parent derivation.
- Captured Rust generation before admission and confirmed the same clause ID,
  literals, and parent IDs.
- Called C `ClauseIsTrivial` directly and inspected cached polarity counts,
  literal pointers, and term entry numbers, ruling out stale counts and term
  sharing differences.
- Re-ran the end-to-end trace after the code change; the former ordinal-312
  divergence disappeared and the matching prefix increased to 561 clauses.
- Ran 4,041 unit tests and 3 schedule integration tests, the all-targets
  all-features Clippy check with warnings denied, locked release builds under
  WSL and Windows, all generated C-source documentation checks, the 50-case
  interop comparison, and the five-run benchmark.

## Conclusion And Limits

The first GEO288 search divergence was caused by Rust improving an accidental C
false negative in long-list tautology detection. Preserving C's search state
machine restores the compatibility-visible hidden selection and advances the
matching prefix by 250 clauses.

This experiment does not establish complete GEO288 parity. Rust still reaches
the resource limit, and the next hidden-selection discrepancy before visible
ordinal 559 must be investigated separately.
