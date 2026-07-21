# Compact PD-tree traversal frame

## Question

Does narrowing the two-state PD-tree traversal-step field reduce cursor frame
movement enough to improve production performance without weakening the proof
or resource boundaries?

## Setup

- Parent source: commit `cc302148` (`Stream non-binding PD-tree candidates`).
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Parent profile: 13,122,494,580 instructions with the exact proof and 4,873
  processed clauses.
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`, profiled at 5,254,361,329
  instructions.
- Native proof corpus: GEO288, HEN011, LUSK6, and LUSK6ext with proof objects.
- Native resource corpus: BOO020 and SWV851 at 60 process-CPU seconds and a
  2-GiB C data allowance.

The profile is retained at
`.artifacts/experiments/2026-07-20-157-compact-pdt-frame/rust-callgrind-compact-frame.out`.
Compatibility reports are retained under `.artifacts/e-compare/`.

## Attribution and candidate

The retained line profile attributes most of
`pop_subst_cursor_frame` to vector element movement and term ownership work.
`PdtTraversalFrame::next_step` only takes values zero through two, but its
`usize` representation makes the complete 64-bit frame 48 bytes. Changing only
that field to `u8` makes the frame 40 bytes without narrowing node indices,
binding positions, variable-child links, or terminal positions. A layout
regression confirmed the 40-byte size, and all 40 PD-tree tests passed.

## Performance result

The candidate preserves the exact LUSK proof at 13,092,110,614 instructions,
30,383,966 below the parent (-0.2315%). The shared lazy cursor falls from
1,602,754,924 to 1,576,798,425 exclusive instructions (-25,956,499), and
`pop_subst_cursor_frame` falls from 284,175,947 to 279,148,494 (-5,027,453).
The C/Rust ratio would improve from 2.497 to about 2.491.

## Compatibility result

The first standard proof run at
`.artifacts/e-compare/20260720-232151-587188/` reached the HEN011 cutoff during
a globally slow interval. The accepted parent binary also reached the cutoff
in the valid focused control at
`.artifacts/e-compare/20260720-232524-047525/`. At a diagnostic 120-second
limit, both candidates produced the exact HEN proof: compact-frame report
`.artifacts/e-compare/20260720-232731-100650/` and parent report
`.artifacts/e-compare/20260720-233010-509903/`. A compact-frame repeat completed
in 51.95 seconds at `.artifacts/e-compare/20260720-233244-952366/`, and the
final ordinary-limit four-case proof report
`.artifacts/e-compare/20260720-233445-692459/` had zero mismatches.

The resource boundary rejects the candidate. Report
`.artifacts/e-compare/20260720-233637-713791/` records a BOO020 allocator abort
at 52.48 seconds while C returns normalized `ResourceOut`; SWV851 remains
exact. The accepted parent binary returns exact normalized `ResourceOut` on
the same focused BOO input and load in
`.artifacts/e-compare/20260720-234127-165111/`. The smaller frame therefore
advances BOO search far enough to exhaust the maintained 2-GiB allowance before
the CPU cutoff, even though its local traversal storage is smaller.

## Decision

Reject the compact traversal frame. Restore `next_step: usize` and remove the
layout regression exactly. The deterministic 0.2315% LUSK improvement is not
worth reopening the BOO allocator boundary; future cursor work must either
reduce the late-search live set as well as instruction count or leave BOO work
rate unchanged at the cutoff.
