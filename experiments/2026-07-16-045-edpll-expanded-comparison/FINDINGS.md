# edpll expanded executable comparison

## Status

Completed for Bead `E_Rust_Port-j76.1.37`. This slice turns the remaining
deterministic `edpll` diagnostic and resource-option surfaces into permanent
support-tool cases, fixes clause-trace timing, and records the narrow host
failure boundaries that cannot be represented as stable byte-for-byte input
workloads. The vendored C source remained unchanged.

## Archived C reference evidence

The surviving real C/Rust report is:

`.artifacts/e-compare/20260711-080549-170729-tools/tool-comparison.json`

It used upstream commit `17026b1bfe61aaf223cfaae54947c8d2679c31a0`. Its
four `edpll` cases prove byte-for-byte equality for help, version, the small
LOP `--dimacs` stream, and the old-TPTP `--tptp-in` input-clause stream. All
four also have equal exit status zero.

## Clause-trace timing fix

C's `DPLLFormulaParseLOP()` prints each completed clause directly to
`GlobalOut`. Rust previously accumulated all trace text for one input file and
returned it only after parsing the whole file. If a later clause was malformed,
the error discarded already-completed trace text.

`DpllFormula::parse_lop_with_trace` now delivers each completed trace line to a
caller sink. The original `parse_lop` API remains as a collecting adapter, while
`edpll` uses the streaming API. The exact regression input `p.\nq(f(a).\n`
therefore returns syntax exit 3 while retaining:

```text
New clause: p<-....accepted
```

The change adds one short per-clause trace allocation and does not alter clause
storage, normalization, or the intentionally absent solver phase.

## Expanded permanent matrix

The support-tool matrix now contains 14 cases: help, version, and 12 functional
cases. In addition to the two archived streams it covers:

- a trailing non-clause token that C intentionally ignores;
- a two-clause output file with exact generated contents;
- a malformed later term, malformed equation, and empty procedural tail;
- successful hard/soft/memory option processing;
- both hard/soft equality orders and their historical diagnostic typos; and
- missing input and missing output-parent failures in isolated workdirs.

The filesystem cases retain relative paths, actions, line breaks, channels,
and exit code. The shared harness canonicalizes only the complete known
POSIX/Windows system-error suffix, and the failed output case also proves that
no destination artifact appears.

## Resource-limit contract

Source review of `PROVER/edpll.c` and `BASICS/clb_os_wrapper.c` established:

- `--memory-limit=Auto` computes 80 percent of detected physical MB, converts
  it to bytes, then misleadingly prints that byte value with an `MB` label;
- equal or inverted hard/soft limits use distinct messages based on which
  option was processed last, with the source's `softtime`/`hardtime` typos;
- `SetMemoryLimit()` invokes `RLIMIT_DATA` twice when `RLIMIT_AS` exists, labels
  the second warning `RLIMIT_AS`, masks failed `RLIMIT_DATA` warnings, and emits
  both labels when the identical limit must be reduced; and
- CPU/core setup uses direct POSIX `getrlimit`/`setrlimit` calls before parsing,
  even though this executable constructs a DPLL state and never searches.

Rust now pins the exact Auto text, both validation paths, paired reduced-limit
warnings, and masked failed-limit behavior. The successful resource case uses
`--cpu-limit=30 --soft-cpu-limit=20 --memory-limit=0`, exercises configuration
without making the tiny parser race a one-second budget, and exits zero.

Direct POSIX CPU/core syscall failures require host fault injection or changing
the caller's resource ceiling and are not portable input/output workloads. On
Windows, enforcing a job-object CPU limit would terminate with
`STATUS_QUOTA_EXCEEDED` before a C-shaped signal diagnostic could be written.
Because the referenced tool has no solver phase, Rust preserves the parsed
global CPU state but keeps actual CPU/core enforcement as a documented host
policy boundary. This is an evidence-backed compatibility decision, not a
claim that forced syscall failures were observed as equal.

## Broken-pipe boundary

C normally inherits POSIX `SIGPIPE`; native Windows reports a write error, and
the comparison harness's capture pipe keeps its reader open. Consequently no
single real-pipe case has a portable exit/channel contract. Rust retains the
deterministic compatibility surface through the existing injected flush-failure
regression, which pins C's `OutClose()` wording exactly. Real broken-pipe
termination remains platform policy rather than a normalized byte comparison.

## Current reference availability

This desktop session has no installed WSL distribution, no visible cached C
executable, and no native POSIX C toolchain. The 10 newly added cases therefore
could not be run against the archived ELF binary. This is an environment
limitation, not an assertion that unobserved outputs matched.

When WSL is restored, the complete differential command is:

```powershell
cargo build --locked --release --bins
.\e-interop.ps1 build-reference
.\e-interop.ps1 compare-tools -RustBinDir .\target\release -Tool edpll
```

## Native verification

`run_native.py` materializes the permanent cases with the same harness helpers.
All 14 optimized Rust cases returned the expected status: seven exit 0, three
syntax cases exit 3, two option-order cases exit 5, and two filesystem cases
exit 6. The successful output file existed with exact contents, the failed
output path was absent, and the malformed-later-term case retained its exact
completed trace prefix.

Validation:

- `cargo test --locked --lib prover::edpll::tests --quiet -- --test-threads=1`:
  22 passed;
- `cargo test --locked --lib --quiet -- --test-threads=1`: 4,140 passed;
- all binary targets passed under `cargo test --locked --bins`;
- integration targets `eprover_schedule`, `e_stratpar`, and
  `executable_inventory`: 4, 1, and 1 passed;
- `cargo check --locked --all-targets`: passed;
- `cargo clippy --locked --all-targets -- -D warnings`: passed;
- `cargo fmt --all -- --check`: passed;
- `cargo build --locked --release --bin edpll`: passed;
- bundled-Python `unittest` discovery under `tools/e-interop`: 32 passed;
- isolated optimized executable matrix: 14 expected outcomes passed;
- C-source documentation coverage: 492 sources / 266 unit docs;
- Change Later and local-link checks: 269 Markdown files each; and
- regeneration preservation: 268 Markdown files.
