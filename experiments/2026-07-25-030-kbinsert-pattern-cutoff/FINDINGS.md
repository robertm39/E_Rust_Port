# Experiment 331: KB-insert pattern cutoff

## Status

Completed for Beads `E_Rust_Port-j76.3.162` and
`E_Rust_Port-j76.4.903`.

## Question

Does `parse_example_clause` preserve C `ParseExampleClause`'s silent `NULL`
result when representative pattern search exceeds
`PATTERN_SEARCH_BRANCHLIMIT`?

## Baseline

- `lit_list_rep_pattern` and `pattern_clause_compute` already reproduce C's
  zero-tries result when the initial choice count exceeds the branch limit.
- The standalone `epatternize` consumer already skips a clause when
  `PatternClauseResult::tries()` is zero.
- `parse_example_clause` retained an `Option<AnnoTerm>` return type but encoded
  and inserted every successfully parsed clause without checking `tries()`.

## Candidate

`parse_example_clause` returns `Ok(None)` immediately when the represented
pattern search returns exactly zero tries, matching C's truth test around
`PatternClauseCompute`. A direct two-equality TSTP regression exceeds the
three-choice branch limit and pins:

- successful consumption of the complete annotated clause;
- a `None` result; and
- no insertion into the destination term bank.

## Exact commands

Validation runs on an ephemeral native-Linux worker:

```powershell
.\linode-runner.ps1 up
.\linode-runner.ps1 sync
.\linode-runner.ps1 exec -- `
  "cd /opt/e-rust-port/source && bash experiments/2026-07-25-030-kbinsert-pattern-cutoff/remote_validate.sh"
.\linode-runner.ps1 down
```

## Results

- The first worker (`e-rust-codex-260726-175633-f1c3`) falsified the initial
  source layout at the formatting gate; the exact `cargo fmt` wrapping was
  applied locally before retrying.
- The second worker (`e-rust-codex-260726-175912-8797`) passed formatting and
  found a test-only import error at compilation; `IoFormat` was imported from
  its actual owner, `inout::scanner`.
- The final worker (`e-rust-codex-260726-180144-d915`, source snapshot
  `b3b338b7ea4ecf236a95f52ccd0b47976d65b71d963e38246a7e4307d452b499`)
  passed:
  - `cargo fmt --all -- --check`;
  - strict all-target/all-feature Clippy;
  - the exact cutoff regression (`1 passed`, `4401 filtered out`); and
  - the complete all-target/all-feature suite (`4,409` library tests plus
    `11` integration tests, `4,420 passed`, zero failures).
- All three workers and their firewalls were deleted after their respective
  runs.

## Falsification rule

Reject the candidate if the focused regression does not exercise a zero-tries
pattern result, if the scanner does not consume the clause exactly, if any
destination term is inserted, or if strict formatting, Clippy, or the complete
Rust test suite fails.

## Conclusion

The candidate is retained. Rust now preserves C `ParseExampleClause`'s silent
skip at the existing `Option` boundary when representative pattern search
returns zero tries. The regression also establishes that the parser consumes
the rejected clause and the destination term bank remains unchanged.

## Limits

- This slice changes only the already-existing C-compatible `Option` boundary;
  it does not change pattern search ordering or the branch-limit estimator.
- C was not modified.
