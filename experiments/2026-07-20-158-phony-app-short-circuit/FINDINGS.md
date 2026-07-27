# Phony-application predicate short-circuit

## Question

Does checking the reserved phony-application function code before the
de-Bruijn-variable property avoid a property load on ordinary first-order
terms and reduce dereference overhead?

## Setup

- Parent source: commit `892bd4db` (`Record rejected compact PD-tree frame`).
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Parent profile: 13,122,494,580 instructions with the exact proof and 4,873
  processed clauses.
- Candidate profile:
  `.artifacts/experiments/2026-07-20-158-phony-app-short-circuit/rust-callgrind-phony-app.out`.

## Attribution and candidate

`deref_step` costs 633,096,520 exclusive instructions in the accepted parent
and calls `Term::is_phony_app` on ordinary first-order dereference paths. The
original predicate checks the de-Bruijn-variable property before comparing the
function code with `SIG_PHONY_APP_CODE`. The candidate reversed the two
operands so ordinary symbols could reject after the function-code comparison.
A focused regression covered the necessary numeric-code collision: a term with
the reserved code and `TP_IS_DB_VAR` must not be classified as a phony
application.

## Result

The candidate preserves the exact proof but executes 13,222,891,620
instructions, 100,397,040 above the parent (+0.7651%). `deref_step` rises to
640,251,325 (+7,154,805), while the larger global loss is distributed through
allocator and term-processing code. The shared PD-tree cursor remains nearly
flat at 1,601,993,779 versus 1,602,754,924 in the parent. This is a decisive
deterministic regression rather than a local win obscured by cursor work.

## Decision

Reject the reordered predicate and remove its focused test with it, restoring
the accepted source exactly. Do not run the proof or resource matrices for a
candidate already falsified by the deterministic performance gate. Future
dereference work should reduce binding ownership or the number of dereference
steps rather than reorder this well-predicted semantic guard.
