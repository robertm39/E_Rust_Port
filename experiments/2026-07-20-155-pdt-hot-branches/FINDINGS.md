# PD-tree frame and token hot branches

## Question

After making demodulator queries lazy, which bounded cursor change reduces the
new traversal and backtracking hotspot without undoing the compact safe state
machine or weakening higher-order token classification?

## Setup

- Parent source: commit `841c3593` (`Make PD-tree query traversal lazy`).
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Production parent profile: 13,412,948,963 instructions with the exact proof
  and 4,873 processed clauses.
- Separate line-enabled parent profile: 13,412,606,151 instructions. Debug
  information changed the total by only -342,812 (-0.0026%), but this profile
  is used for source attribution only.
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`, profiled at 5,254,361,329
  instructions.
- Native proof corpus: retained GEO/HEN/LUSK four-case corpus with proof
  objects enabled.
- Native resource corpus: `BOO020-1.p` and `SWV851-1.p` at 60 process-CPU
  seconds and a 2-GiB C data allowance.

Profiles are retained at
`.artifacts/experiments/2026-07-20-155-lazy-pdt-lines/`. Compatibility reports
are retained under `.artifacts/e-compare/`.

## Line attribution

The lazy cursor made `search_next_matching_occurrence_with_subst` the largest
exclusive Rust function. Its separate frame-restoration function retired
276,837,018 instructions in the production profile. The line build attributed
most of that work to vector operations around frame and processed-query
restoration.

The symbol branch invoked `prefix_token` 5,136,555 times. In the line profile,
that general classifier cost 171,946,069 instructions because every ordinary
first-order function also tested DB-variable, phony-application, and lambda
properties before returning its function code. C already has the global
problem type and its first-order terms cannot contain those higher-order cell
forms.

## Rejected merged frame

The first candidate moved each processed query term and direct-child expansion
count into its corresponding traversal frame. Variable bindings referred to
the owning frame, eliminating the parallel `query_steps` vector and one
push/pop stream. The focused substitution-cursor tests passed and the proof
was exact, but the larger monolithic frame regressed the production profile to
13,442,422,820 instructions, 29,473,857 above the parent (+0.2197%). The
source was reverted exactly. The rejected profile is
`rust-callgrind-merged-frame.out`.

## Accepted first-order token dispatch

The retained candidate captures the thread-local global problem mode once in
each search state. Explicit first-order searches use a compact classifier:
negative codes produce the same variable identity/type/weight token, while
all nonnegative codes are ordinary function tokens. Higher-order and
uninitialized modes continue to use the unchanged general classifier with DB,
application, and lambda handling. Debug assertions pin the first-order
precondition, and a direct regression checks that ordinary function and free-
variable tokens equal the general classifier's output.

The exact-proof profile falls to 13,328,560,605 instructions, 84,388,358 below
the parent (-0.6292%). The deterministic C/Rust ratio improves from 2.553 to
2.537. The out-of-line general `prefix_token` cost falls to 5,509,833
instructions. The specialized work is folded into the cursor, whose exclusive
cost rises from 1,532,906,560 to 1,604,192,052, while search initialization
rises from 63,168,408 to 75,451,154 for mode capture. The global result shows
that removing repeated higher-order property classification pays for both
cost shifts.

## Compatibility result

The final four-case proof report at
`.artifacts/e-compare/20260720-212159-627250/` has zero mismatches across
GEO288, HEN011, LUSK6, and LUSK6ext. The final resource report at
`.artifacts/e-compare/20260720-212421-530977/` has zero mismatches for BOO020
and SWV851. Thus first-order specialization preserves exact proof order and
the maintained resource boundary without changing higher-order dispatch.

The complete Rust suite passes 4,372 library tests plus every integration
target. Strict all-target/all-feature pedantic Clippy, formatting, and the
all-feature release build pass.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-fo-token.out \
  target-wsl-155-fo-token/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```

```powershell
cargo build --locked --release --bin eprover `
  --target-dir target\native-155-fo-token
& .\e-interop.ps1 compare `
  -Corpus .\.artifacts\e-corpus\diversity-scratch-139\proof `
  -RustExe .\target\native-155-fo-token\release\eprover.exe `
  -TimeoutSeconds 60 -MemoryLimitMb 2048
& .\e-interop.ps1 compare `
  -Corpus .\.artifacts\e-corpus\diversity-scratch-139\resource `
  -RustExe .\target\native-155-fo-token\release\eprover.exe `
  -TimeoutSeconds 60 -MemoryLimitMb 2048
```

## Decision

Reject the merged-frame layout and retain separate traversal and processed-
query vectors. Accept first-order token dispatch while preserving the complete
general classifier for every other problem mode. Keep the main parity issue
open: the synthetic one-second LUSK cutoff and the remaining 2.537
deterministic C/Rust instruction ratio still fail the project-wide acceptance
criteria.
