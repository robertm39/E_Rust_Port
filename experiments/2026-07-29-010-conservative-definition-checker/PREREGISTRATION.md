# Preregistration

## Question

Can Umlaut's existing positive-only validation gate certify used conservative
definitions by selecting an independent checker that supports them, without
weakening proof or mutation rejection?

## Candidate checker

The candidate is ProofGuard 1.0 at Git commit
`18fc573131648c9d1ed81e818f52f704c435033e`. The upstream checker:

1. requires an introduced definition to have the shape
   `![vars]:(fresh_pred(vars)<=>body)`;
2. rejects a previously seen principal predicate, repeated principal
   variables, a circular body, or body variables outside the principal
   arguments; and
3. invokes an external E process to check each theorem-preserving dependent
   inference.

The commit is fetched and used transiently on the Ubuntu runner. The upstream
repository has no license declaration at this revision, so the experiment
must not redistribute or package it. The checker is configured through
Umlaut's existing shell-free external-command interface.

ProofCheck 1.0 with executable SHA-256
`92bb5193a9d8b2857fb97d9bd9fb6f16f5bcb57d07e4307d7f087e403ff51c7e`
is the negative coverage control.

## Frozen cases

### Positive

- the committed minimized used-definition problem/proof from experiment
  `2026-07-29-009-tstp-input-leaf-provenance`;
- a fresh `PUZ008-2` proof emitted with
  `--split-clauses=7 --split-method=2 --split-aggressive
  --split-reuse-defs`.

### Adversarial

Starting from the minimized proof, generate exactly four mutations:

1. make the principal predicate occur in the original problem;
2. put the principal predicate in its own defining body;
3. change the defining body while leaving its dependent clauses unchanged;
4. replace the definition parent of the first dependent clause with an
   unrelated input parent.

The third mutation remains a conservative definition in isolation. It must
still be rejected because its unchanged dependent inference no longer
follows.

## Gates

The checker path advances only if:

1. the pinned ProofGuard repository revision and remote URL match exactly and
   its upstream test suite passes on Ubuntu 24.04 with the runner's pinned E;
2. ProofCheck self-certifies all 117 bundled tests;
3. ProofCheck reports `Unknown` on both valid used-definition proofs, preserving
   the reproduced coverage boundary;
4. ProofGuard reports `VerifiedGood` on both valid proofs;
5. `tools/validation/validate_tptp_solution.py`, without
   `--allow-coverage-gap`, accepts both valid proofs when ProofGuard is the
   configured external command;
6. ProofGuard reports `VerifiedBad` for all four mutations and the
   positive-only gate rejects all four;
7. no checker source or binary is committed, packaged, or linked into Umlaut;
8. controller tests, the full proof-output suite, compatibility gates, strict
   Clippy, cross-platform builds, and resource gates pass.

Any `Unknown`/`Timeout` from ProofGuard on a positive case, any accepted
mutation, any changed prover proof semantics, or any licensing-boundary
violation blocks adoption.
