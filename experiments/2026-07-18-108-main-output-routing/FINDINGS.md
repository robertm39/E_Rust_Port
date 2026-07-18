# Main executable output-routing closure

## Status

Completed for Bead `E_Rust_Port-j76.2.30`. The default-feature Rust executable
routes the represented parse, preprocessing, proof-search, watchlist, result,
proof-object, statistics, and resource surfaces exactly like the pinned default
C build. No production change was needed, and the vendored C checkout remains
unchanged.

## C ownership model

The C executable intentionally has two normal output owners after `-o` opens
`GlobalOut`:

- raw/preprocessing classification, chosen configuration, search class, and
  related selection traces use `stdout`; and
- print-info lines, proof-state initialization, AC/watchlist status, result
  banners, SZS statuses, proof objects, statistics, and resource information
  use `GlobalOut`.

Rust models the first group through the explicit stdout side channel on
`ConfiguredOutput` and the second through its buffered configured-output
writer. The fresh matrix demonstrates that this is observable compatibility,
not merely source-shaped plumbing: the same 83-byte stdout stream is retained
across every successful exact case while each configured file receives its own
case-specific content.

Malformed input also matches the early-open boundary. Both implementations
create an empty configured file, leave stdout empty, emit the same 398-byte
scanner diagnostic on stderr, and exit with syntax status 3.

## Fresh configured-output matrix

[`compare_output_routing.py`](compare_output_routing.py) feeds fixtures through
stdin so proof sources have the same `<stdin>` spelling, gives C and Rust
separate output files, and compares exit status plus all three byte streams.
The default-feature executable is 8/8 byte-exact for:

- satisfiable and proof-found final reporting;
- combined AC recognition through CNF-only output;
- the default statistics surface;
- TSTP proof-object framing and content;
- inline-watchlist reduction and exhaustion;
- malformed-input diagnostics after configured-file creation; and
- initial print-info routing.

The resource-info case is exact after replacing only preprocessing/user/system/
total measured seconds and the target-dependent resident value. Its stdout,
stderr, result/status text, footer structure, and exit status remain strict.

The hard-timeout case is compared as a stable ownership projection because C
signal arrival and native-Windows cooperative polling can stop at different
phases. Both create the configured file, write exactly one doubled-percent
hard-timeout banner to that file, avoid the soft-timeout text, emit the same
fatal stderr diagnostic, and exit 8. Exact raw-descriptor/buffer ordering is
already covered by the dedicated global-output and signal reconciliations in
experiments 043 and 040.

The retained [`reference.json`](reference.json) has SHA-256
`6F46A0FE7163E2CEC0CCED2965D386CFEA877B5342E960E0DC2DFEFB2E66D372`.
The compared default-feature Rust release has SHA-256
`3B4674303926320C6BB62CADB56276A83E9FA7E4B271655618B60C1173AF519B`.

## Instrumentation-feature boundary

An initial comparison accidentally used `--all-features`. That binary adds the
intentional `measure-unification` timers and `print-index-stats` distribution/
DOT output to `--print-statistics`, corresponding to non-default C compile-time
instrumentation. The pinned C reference is a default build and Cargo declares
`default = []`, so the drop-in matrix correctly uses the default-feature Rust
release. All-feature builds remain covered by the repository test and Clippy
gates; their explicitly enabled instrumentation is not a GlobalOut ownership
defect.

## Scope decision

The legacy note said full routing awaited a complete proof pipeline. That
dependency is now satisfied by the represented formula/clause owner path,
watchlist/indexed saturation, result status mapping, proof-output closure, and
resource-limit closure. Broader parsing, higher-order inference, scheduler, and
performance work remains tracked by their own Beads and the ongoing executable
completion umbrella; it does not leave this output owner unresolved.

No performance benchmark is warranted because production code is unchanged.

## Reproduction

```powershell
cargo build --locked --release --bin eprover `
  --target-dir target\default-reference

C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-18-108-main-output-routing\compare_output_routing.py `
  --c-exe /home/rober/.cache/e-rust-port/bin/17026b1bfe61aaf223cfaae54947c8d2679c31a0/fol/eprover `
  --rust-exe target\default-reference\release\eprover.exe `
  --output target\main-output-routing-reference-check.json `
  --expected experiments\2026-07-18-108-main-output-routing\reference.json
```
