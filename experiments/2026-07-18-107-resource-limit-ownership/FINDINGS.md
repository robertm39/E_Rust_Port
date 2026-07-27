# Resource-limit ownership reconciliation

## Status

Completed for Bead `E_Rust_Port-j76.2.31`. A fresh pinned-C comparison found
and fixed one remaining executable mismatch: `eprover` recorded explicit and
automatic memory limits correctly but omitted C's verbose option-processing
diagnostics. The vendored C checkout remains unchanged.

## Fixed memory-limit diagnostic ownership

C emits memory-limit verbose text while processing each option, not later when
it applies the OS limit. That makes the output sensitive to option order and
keeps it visible before a later option error or an early `--help`/`--version`
action.

Rust now retains an ordered option-stderr stream alongside the parsed config.
It records:

- `Physical memory determined as N MB` before the automatic-limit line;
- `Memory limit set to M MB` for automatic and explicit limits;
- no line when `--memory-limit` appears before the option that enables
  verbosity; and
- the signed C display spelling for explicit negative values even though the
  stored `rlim_t`-shaped byte count is unsigned and wrapping.

The same ordered stream carries the existing LPO recursion warning, so adding
raw diagnostics does not reorder it relative to other option-time output.
Permanent regressions cover the config state, option-order split, negative
display, early version action, and output before a later CPU-limit validation
error.

## CPU-limit state and outcomes

The existing state path remains source-aligned:

- each hard/soft option updates its own limit and overwrites the schedule limit
  at that point in option order;
- a present soft limit owns the initial Linux `RLIMIT_CPU`, otherwise the hard
  limit does;
- soft expiry latches the cooperative stop, produces the single-percent user
  resource suffix, and exits with status 9; and
- hard expiry uses the direct doubled-percent banner, fatal diagnostic, and
  status 8.

The fresh comparison uses projections for both timeout cases because C's
`SIGXCPU` can arrive at different proof-search phases from run to run, while
native Windows deliberately polls the process-CPU deadline cooperatively.
Both projections are exact for the stable contract: banner/status ownership,
stderr, and exit status. The hard banner is the first emitted byte sequence,
preserving direct-descriptor output before pending buffered stdout.

The safe cross-platform decision remains unchanged. Linux builds install the
C-shaped signal trampoline and reset/rearm limits; native Windows does not use
a CPU Job Object because quota termination would preempt the required E output
and exit mapping. The retained live-delivery evidence is in
[`../2026-07-17-040-signal-delivery-reconciliation/FINDINGS.md`](../2026-07-17-040-signal-delivery-reconciliation/FINDINGS.md),
and raw output-descriptor ownership is in
[`../2026-07-17-043-global-output-fd-reconciliation/FINDINGS.md`](../2026-07-17-043-global-output-fd-reconciliation/FINDINGS.md).

## Memory and resource-footer platform boundary

The implementation retains C's Linux `RLIMIT_DATA` behavior, including the
duplicated branch labeled `RLIMIT_AS`, masked failed-`RLIMIT_DATA` warning, and
reduced-limit result. Native Windows owns a retained process-memory Job Object
limit when assignment succeeds. The OS mechanisms are platform adapters; the
parsed byte state and automatic delete-bad heuristic are shared.

The resource-info case is exact after replacing only the three measured CPU
second values and the target-dependent maximum-resident value. The complete
footer structure, historic `pages` label, parsing success/status, exit code,
and empty stderr match. Automatic memory diagnostics also match as a host
projection: both print the two lines in order and derive the displayed limit
by truncating 80 percent of their respective host physical-memory value.

## Fresh reference

[`compare_limits.py`](compare_limits.py) runs isolated unchanged C at commit
`17026b1bfe61aaf223cfaae54947c8d2679c31a0` and the Rust release. The retained
[`reference.json`](reference.json) contains:

- 3/3 byte-exact option-order and signed-memory diagnostic cases;
- one resource-footer case exact after host-value normalization;
- one automatic-memory case exact under its host-value projection; and
- exact soft- and hard-timeout outcome projections.

The reference has SHA-256
`F0AA30BD04C9FD0CD6191BF844D87398242CA35F4A7579CA7AB275EE1E4BB701`.
The compared Rust release had SHA-256
`FDF0DD716D099664B7AEFDDAA5A48E04C02CA390C5B940F4BBB7DDF68EF8B9F9`.

## Reproduction

```powershell
cargo build --locked --release --bin eprover --all-features

C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-18-107-resource-limit-ownership\compare_limits.py `
  --c-exe /home/rober/.cache/e-rust-port/bin/17026b1bfe61aaf223cfaae54947c8d2679c31a0/fol/eprover `
  --rust-exe target\release\eprover.exe `
  --output target\resource-limit-reference-check.json `
  --expected experiments\2026-07-18-107-resource-limit-ownership\reference.json
```
