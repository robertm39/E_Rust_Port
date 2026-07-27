# Initial evaluation-order trace

## Question

Why do the Rust and C provers first choose different tied initial clauses for
`ALL_RULES.p`, `LUSK6ext.p`, and `SWC078.p` despite matching normalized status
and proof validity?

The working hypotheses are that the ports differ in one of these places:

1. axiom-list order before the `Uniq` reweight;
2. `Uniq` priority or heuristic values;
3. global evaluation-counter allocation order; or
4. active-HCB evaluation order after the `Uniq` traversal.

## Setup and commands

Run from the repository root. The C command uses the cached, unmodified FOL
reference binary recorded by comparison report
`.artifacts/e-compare/20260712-000443-172305/comparison.json`.

```powershell
wsl.exe -d Ubuntu-24.04 -- bash -lc "gdb -q -batch -x /mnt/c/Users/rober/OneDrive/Desktop/E_Rust_Port/experiments/2026-07-12-001-initial-eval-order/trace-c.gdb /home/rober/.cache/e-rust-port/bin/17026b1bfe61aaf223cfaae54947c8d2679c31a0/fol/eprover" > experiments/2026-07-12-001-initial-eval-order/c-trace.txt
```

## Results

- C and Rust call `HCBClauseEvaluate` on the same ten source axioms in the
  same `Uniq` order, then on the same active-heuristic copies in the same
  `Uniq`-index order. All 35 captured evaluation cells match in priority,
  floating heuristic value, and global evaluation counter. The raw C tuples
  are in `c-eval-tuples.txt`.
- The OutputLevel-6 traces in `c-level6.txt` and `rust-level6.txt` have a
  common evaluation sequence of 17 out of 17 clauses. This falsified the four
  initial queue/evaluation hypotheses for `ALL_RULES.p`.
- The final C graph trace in `c-final-graph.txt` showed separate selected
  commutativity/associativity quote nodes. C `DerivStackExtractOptParents`
  applies `ClauseDerivFindFirst` to explicit opcode arguments, but its
  `DCACRes` loop appends `sig->ac_axioms` directly. The same physical selected
  clause is therefore collapsed to its original axiom when used as a rewrite
  parent and retained when injected as an AC-resolution parent.
- Rust applied dummy-quote collapse uniformly to both edge kinds. It also
  skipped `DocClauseQuote`'s pre-gate `CPInputFormula` clearing in output-free
  initialization/reset paths, making retained selected nodes print as axioms.
- The fix adds an explicit AC-parent resolution mode, preserves direct-parent
  display remapping through the collapsed edge, and performs the input-marker
  transition even when quote output is suppressed. `rust-fixed.stdout` now
  matches the C `ALL_RULES.p` proof after path normalization.
- Full comparison report
  `.artifacts/e-compare/20260712-021021-202531/` has 6 mismatches, down from 7.
  `ALL_RULES.p` is exact; the remaining cases are unchanged.
- `c-proof.dot` and `rust-proof.dot` are retained negative probes: using
  `--proof-object=2` still selects list output rather than DOT output, so the
  extension reflects the original exploratory assumption rather than content.

## Falsification checks

- Compared every C/Rust evaluation allocation rather than inferring order from
  the final proof.
- Compared OutputLevel-6 evaluated-clause sequences and final proof-search
  statistics; both matched before the proof graph was extracted.
- Verified the C optimized binary's `EvalCell` and final ordered-derivation
  layouts from disassembly before reading raw fields in GDB.
- Added a Rust regression where an AC axiom is a dummy quote. The graph must
  retain that selected node and then follow its own direct quote parent.
- Added output-free initialization/reset assertions for `CPInputFormula`.
- Reran the complete 50-case C/Rust suite to check for unrelated regressions.

## Conclusion and limits

`ALL_RULES.p` differed in proof extraction, not search. C has an observable,
context-dependent parent-resolution rule: direct parents are quote-collapsed,
but AC parents expanded from the signature are not. Rust now preserves it.

This conclusion does not explain the remaining output mismatches.
`LUSK6ext.lop` and `SWC078-1.p` have genuinely different proof-search paths;
`sledgehammer.p` retains allocator-dependent same-sort binder ordering. The
behavior/resource mismatches are also independent of this proof-graph fix.
