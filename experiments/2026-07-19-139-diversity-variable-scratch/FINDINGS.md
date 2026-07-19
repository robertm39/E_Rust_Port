# Bounded diversity-variable scratch

## Question

Can the diversity-weight WFCB retain its variable traversal storage between
clause evaluations without changing the C-visible variable count, proof
search, or resource behavior?

## Setup

- Parent commit: `39e23fc2`.
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`.
- Deterministic fixture: unchanged `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Previous retained profile: 17,485,717,540 instructions.

The WFCB called the public diversity helper for every evaluation. That helper
created a fresh variable map and function-code vector even though one
`DiversityWeightParam` owner survives for the complete heuristic run.

## Source comparison

C's diversity computation counts distinct variables independently from its
function-code traversal. This distinction matters because term operations may
already have left `TP_OP_FLAG` on variable cells. Reusing the
`ClauseReturnFCodes()` marking walk for variable discovery therefore is not a
valid fusion.

Rust now gives `DiversityWeightParam` private variable-term and variable-ID
vectors. The WFCB-only path traverses non-ground children, records shared term
identity IDs, sorts and deduplicates them, then clears the vectors for reuse.
The public compatibility helper retains its previous `BTreeMap` and
`ClauseReturnFCodes` behavior. Scratch capacity above 1,024 elements is
dropped, so an exceptional clause cannot permanently enlarge every diversity
owner.

## Deterministic result

The retained candidate profile contains 17,232,497,185 instructions, a
reduction of 253,220,355 instructions or 1.45%. The proof and all 4,873
processed-clause calls are unchanged.

The new `diversity_weight_compute_reusing_scratch` subtree contains
750,457,011 instructions, down from 958,590,123 for the prior diversity
subtree: a 21.71% local reduction. The retained profile is
`.artifacts/experiments/2026-07-19-139-diversity-evaluation-scratch/callgrind-current.out`.
Together with experiments 136 through 138, this is a 13.40% reduction from
the retained 19,899,749,157-instruction LUSK6 baseline.

## Rejected ablations

The first combined traversal reused the function-code operation flag for
variables. It changed the search to 2,366 processed clauses because stale
variable flags caused under-counting. Its misleading 6,330,197,560-instruction
profile is retained as
`.artifacts/experiments/2026-07-19-139-diversity-evaluation-scratch/callgrind-rejected-combined-traversal.out`.

A second candidate also retained the complete function-subterm stack. It kept
the exact 4,873-clause trace and reduced the profile to 17,051,335,243
instructions, but its unbounded retained capacity exposed repeatable Windows
resource failures. It is retained at
`.artifacts/experiments/2026-07-19-139-diversity-evaluation-scratch/callgrind-rejected-unbounded-function-scratch.out`
and is not in production source.

## Compatibility and resource checks

Focused LUSK6 proof comparisons remained exact. Resource runs were deliberately
repeated because BOO020 and SWV851 sit close to both the CPU and Windows Job
Object limits:

- `.artifacts/e-compare/20260719-132445-442541/` recorded a BOO allocator
  abort in the first full candidate matrix;
- `.artifacts/e-compare/20260719-134232-903185/` reproduced exact BOO
  `ResourceOut`, while `.artifacts/e-compare/20260719-134840-000438/`
  reproduced the allocator race in a later full run;
- increasing the Windows whole-process allowance to 384 MiB and then 768 MiB
  merely let SWV consume the added headroom before another large allocation.
  Those allowance changes were rejected and fully reverted.

The repeated failures localized a separate whole-clause sort allocation,
investigated in experiment 140. They did not justify weakening the configured
memory limit or retaining the function traversal stack.

## Falsification checks

- A regression leaves `TP_OP_FLAG` on a shared variable, counts it twice in
  consecutive evaluations, and verifies retained capacity.
- Focused diversity tests and the all-target/all-feature check pass.
- The accepted Callgrind profile preserves the exact proof and processed
  clause count.
- Both faster but search-changing and faster but resource-unsafe candidates
  were rejected.
- The vendored C checkout remains unchanged.

## Decision

Accept only bounded variable scratch. It removes 1.45% of deterministic
instructions without changing search and prevents exceptional clauses from
pinning large traversal buffers. Keep function-code collection fresh, and
continue the independently identified clause-sort/resource owner in
experiment 140.
