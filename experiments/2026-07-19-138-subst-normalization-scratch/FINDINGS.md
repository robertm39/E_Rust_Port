# Reusable substitution-normalization scratch

## Question

Can `Substitution::norm_term` retain its traversal vector across the four
normalization calls made by each simultaneous paramodulant, matching the
reusable-stack behavior expected by the C proof-search owner?

## Setup

- Parent commit: `dc1ed749`.
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`.
- Windows candidate: `target/subst-norm-scratch/release/eprover.exe`, SHA-256
  `7433A64CA989F20BBF166682196066F11AF12CA6B00D562BD9AB7CBA225DFD24`.
- Deterministic profile: unchanged LUSK6 fixture under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Maintained comparison: native Windows Rust versus archived WSL C with the
  standard 50-case harness.

The experiment-137 profile contains 17,671,438,618 instructions.
`Substitution::norm_term` owns 1,305,410,418 instructions, or 7.39% of the
profile. Its 488,212 calls allocate and destroy a fresh traversal `Vec<Term>`;
Callgrind attributes 191,522,902 instructions to 465,252 vector-growth calls,
plus allocation, deallocation, and vector-drop work.

## Source comparison

C creates one constructor-local substitution and calls `SubstNormTerm()` four
times while building a simultaneous paramodulant. Its local traversal stack is
reused within that owner. Rust created a new traversal vector inside each
`norm_term()` call even though all four calls share one `Substitution`.

`Substitution` now owns an empty normalization scratch vector. `norm_term()`
moves that vector into a local owner, performs the existing left-to-right
stack traversal, and moves the empty vector back afterward. Term visitation,
dereferencing, fresh-variable allocation, binding order, and backtracking are
unchanged. The move-out form also avoids aliasing the scratch field while
`add_binding()` mutates the substitution.

## Deterministic result

The reusable-stack profile contains 17,485,717,540 instructions, a reduction
of 185,721,078 instructions or 1.05%. The exact proof and relevant call counts
are unchanged. `Substitution::norm_term` falls from 1,305,410,418 to
1,046,266,605 instructions, a 19.85% subtree reduction. Vector-growth calls
fall from 465,252 to 102,237 because only the first normalization call for a
constructor-local substitution must establish capacity.

The retained profile is
`.artifacts/experiments/2026-07-19-138-subst-normalization-scratch/callgrind-current.out`.
Together with experiments 136 and 137, deterministic LUSK6 instructions have
fallen 12.13% from the retained 19,899,749,157-instruction baseline.

## Rejected ground-subtree ablation

A follow-up skipped traversal when a dereferenced shared term carried the
term bank's exact `TP_IS_GROUND` bit. The proof and tests remained correct, but
the added property check raised the profile to 17,514,323,549 instructions,
0.16% above reusable-stack-only. The ablation is retained at
`.artifacts/experiments/2026-07-19-138-subst-normalization-scratch/callgrind-ground-prune.out`
and is not present in production source.

## Compatibility result

The focused LUSK6 report at
`.artifacts/e-compare/20260719-123832-759042/` is exact. The final maintained
report is `.artifacts/e-compare/20260719-123904-352020/`: 50 cases, one
unexpected mismatch, and the declared `sledgehammer.p` proof-text difference.

- BOO020 and SWV851 remain exact `ResourceOut`/8 cases.
- GEO288 proves with exact output in 10.57 seconds.
- HEN011 proves with exact output in 56.26 seconds.
- LUSK6 and `LUSK6ext` prove with exact output in 2.94 and 7.23 seconds.
- The synthetic 16 MiB memory-limit case remains exact.
- The sole unexpected case remains synthetic one-second LUSK6: C proves in
  0.38 seconds, while Rust reaches `ResourceOut` at 1.12 seconds.

The single native timing sample is diagnostic, not a percentage claim; the
Callgrind reduction is the deterministic performance evidence.

## Falsification checks

- All nine substitution tests pass, covering normalization order, fresh
  variable marking, binding, backtracking, renaming, and higher-order helpers.
- `cargo check --locked --all-targets --all-features` passes for both the
  reusable-stack candidate and the rejected ground-pruning ablation.
- Callgrind records the identical proof and relevant call counts.
- The full matrix exercises first-order, higher-order, proof-documentation,
  resource, stdin, syntax, and small-memory behavior.
- The ground-prune ablation was rejected from instruction counts despite
  semantic correctness.
- The vendored C checkout is not modified.

## Decision

Accept reusable substitution-normalization capacity. It removes 1.05% of
deterministic LUSK6 instructions with no search-order change and preserves the
one-unexpected-case maintained matrix. Reject per-node ground-bit checks.
Continue with the remaining rewrite, PD-tree, evaluation, and term-bank
owners; the one-second LUSK6 acceptance criterion remains open.
