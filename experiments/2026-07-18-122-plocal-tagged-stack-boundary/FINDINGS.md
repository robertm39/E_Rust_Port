# PLocalTaggedStack compile-time layout and owner boundary

## Status

Completed for Bead `E_Rust_Port-j76.2.16`. Rust retains the complete portable
two-slot tagged-stack behavior without raw pointer manipulation. The vendored C
checkout remains unchanged.

## Dual C layout

The ordinary upstream `Makefile.vars` enables `-DTAGGED_POINTERS`. In that
branch, C stores a two-bit tag in the low aligned bits of each term pointer and
uses one pointer slot per entry. The portable branch stores the value and tag
in two separate pointer slots.

[`capture_c_layouts.py`](capture_c_layouts.py) compiles
[`probe_tagged_stack.c`](probe_tagged_stack.c) against the pinned unchanged
`BASICS.a` in both modes. Retained [`reference.json`](reference.json) proves
identical LIFO values/tags and the distinct growth shapes for a 40-entry
ensure from the 64-slot default:

| C mode | Slots per entry | Allocated slots | Allocated bytes |
| --- | ---: | ---: | ---: |
| `TAGGED_POINTERS` | 1 | 64 | 512 |
| portable | 2 | 256 | 2,048 |

The 256-slot portable result follows C's exact equality-triggered,
old-size-plus-space doubling rule, not a conventional reserve calculation.
Permanent Rust tests pin the same two-slot frame size and 256-slot accounting.

## Owners and performance decision

[`audit_tagged_stack_owners.py`](audit_tagged_stack_owners.py) retains the
complete mapping in [`owner-audit.json`](owner-audit.json). `cto_kbolin.c` is
the only C tagged-stack owner. The generic Rust `PLocalTaggedStack` has no
production owner, so its physical two-slot representation has no executable
runtime or memory cost.

Rust KBO6 directly uses safe typed `(Term, DerefType)` traversal frames for the
corresponding walkers. That active path contains no raw pointer conversions or
unsafe code. The earlier deterministic profile in
[`experiment 066`](../2026-07-16-066-main-eprover-profile/FINDINGS.md) measured
1,306,910 argument-helper calls from the iterative `mfy_vwb` walker and
accepted an argument-borrowing change that removed 0.87% of total instructions.
It did not isolate frame storage as material work.

Low-bit packing of owned `Rc` handles would require raw-pointer ownership
manipulation prohibited by the project unsafe policy. With no compatibility
stack owner and no focused profile demonstrating material frame cost, adding
that boundary is not justified. A future safe compact-frame design remains a
profiling-dependent optimization, not missing behavior.

## Reproduction

```powershell
C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-18-122-plocal-tagged-stack-boundary\capture_c_layouts.py `
  --output target\plocal-tagged-reference-check.json `
  --expected experiments\2026-07-18-122-plocal-tagged-stack-boundary\reference.json

C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-18-122-plocal-tagged-stack-boundary\audit_tagged_stack_owners.py `
  --output target\plocal-tagged-owner-audit-check.json `
  --expected experiments\2026-07-18-122-plocal-tagged-stack-boundary\owner-audit.json
```
