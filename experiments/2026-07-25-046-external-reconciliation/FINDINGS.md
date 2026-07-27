# External and CSSCPA reconciliation

## Status

Accepted for the six remaining open `external` records under Beads
`E_Rust_Port-j76.4`. Direct source review found two coupled allocation gaps in
the Rust `CSSCPA_filter` path. `CsscpaState` now retains a signature-synchronized
tautology scratch bank, matching C's two-bank state without introducing shared
mutable signature ownership, and file scanners retain the original input
`Vec<u8>` allocation behind `Arc<Vec<u8>>` rather than copying it into an
`Arc<[u8]>`. The other four records are preserved parser, status, and numeric
output-level compatibility contracts. The vendored C checkout remains
unchanged.

## Review decisions

| Records | Decision |
|---|---|
| 745 | Close the measured allocation gap. C retains `terms` and `tmp_terms` over one signature. Rust now retains both banks and synchronizes the scratch signature when its symbol/type cardinality changes, using the same policy already exercised by `ProofState::tmp_terms_mut`. Repeated tautology checks reuse their banked work terms. |
| 748 | Preserve the exact six-value `ClauseStatusType` surface. `requested` remains a state-reporting value emitted only by the loop's `state:` command; clause processing never returns it. |
| 749, 754 | Preserve C's two distinct numeric quirks in drop-in mode. Loop input consumes any positive integer but only `0` or `1` changes state. The CLI rejects only values above one, so a negative level remains truthy for direct trace branches but fails the `OUTPRINT(1)` threshold. Existing focused tests pin both boundaries. |
| 755 | Retain the narrow historical bridge. The wrapper selects TSTP mode as C does, but an `input_clause(...)` at the clause boundary is parsed temporarily as old TPTP and the original scanner mode is restored. Core-loop and wrapper tests cover it. |
| 759 | Retain eager scanner ownership for now, but remove the avoidable second full-size allocation and record a scaled C/Rust benchmark. C streaming remains more memory-efficient. Replacing the shared scanner with incremental I/O would be a broad parser redesign, not an isolated CSSCPA fix. |

## Implementation

`CsscpaState::clause_is_tautology` formerly cloned the complete live signature
and allocated a new `TermBank` for every clause. The large repeated-tautology
workload showed that this dominated the Rust tool. A persistent scratch bank
retains hash-consed work terms; a regression proves that the second identical
tautology adds no nodes and that parsing new symbols synchronizes the scratch
signature before use.

`InputStream::from_file_content` formerly converted its input vector into
`Arc<[u8]>`. That conversion must allocate slice storage alongside the Arc
header and copy the bytes, temporarily retaining both full buffers. The stream
now stores `Arc<Vec<u8>>`, so construction keeps the original vector allocation
and stream clones still share input bytes with independent cursor state. A
one-mebibyte pointer-identity regression pins that ownership property.

## Benchmark

[`benchmark_large_csscpa.py`](benchmark_large_csscpa.py) generates a fixed
42-byte tautology command one, 100,000, or 500,000 times and runs optimized C,
the prior Rust snapshot, and the candidate in rotated order for three
repetitions. It records controller wall time and GNU `time` peak RSS. The
accepted run used ephemeral Ubuntu worker
`e-rust-codex-260726-191044-7524`, Linux 6.8.0 x86-64, with these exact binary
hashes:

- C: `cc72b8c75f9b9ce4d85dcc3e4c8e55fa79840a0a7dee258924f9ca87e8acade9`;
- Rust baseline: `aa4d50f0eb5653d9fd978e400ebfd5e9a653b4ac605704b8dd6303389c6b1f51`;
- Rust candidate: `7f2eabc02d654d36571673ff6373378bd40ef400108258edc700777df1067af3`.

| Commands | Input | C median | Candidate median | Baseline median | Candidate / baseline | Rust / C | Candidate RSS | Baseline RSS |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 1 | 42 B | 1.415 ms | 1.422 ms | 1.411 ms | 1.007 | 1.004 | 3,200 KiB | 3,072 KiB |
| 100,000 | 4.2 MB | 0.229 s | 0.307 s | 1.746 s | 0.176 | 1.339 | 8,428 KiB | 11,008 KiB |
| 500,000 | 21 MB | 1.135 s | 1.506 s | 8.718 s | 0.173 | 1.326 | 27,900 KiB | 43,776 KiB |

At 21 MB the candidate is 82.7% faster than its exact baseline and uses 15,876
KiB less peak RSS. C's scanner remains stream-backed at 2,816 KiB, while Rust
retains one input-sized buffer and peaks 25,084 KiB above C. The resulting
1.326 Rust/C runtime ratio is acceptable for this auxiliary path given the
large baseline repair and the deliberate one-copy ownership boundary; this
decision does not claim memory parity. [`benchmark-reference.json`](benchmark-reference.json)
pins every accepted median, fixture digest, and available raw sample.

Reproduce on Linux:

```bash
python3 experiments/2026-07-25-046-external-reconciliation/benchmark_large_csscpa.py \
  --c-bin /path/to/c/CSSCPA_filter \
  --rust-bin /path/to/candidate/CSSCPA_filter \
  --rust-baseline-bin /path/to/baseline/CSSCPA_filter \
  --commands 1,100000,500000 \
  --repetitions 3
```

## Audit

[`audit_external_reconciliation.py`](audit_external_reconciliation.py) pins the
exact six migrated identities and content hashes, checks all seven grouped
source/implementation/benchmark contracts, and retains the earlier exact
CSSCPA and full-port compatibility evidence. Its source digest covers the
three unchanged C units and nine Rust, status, experiment, and evidence files.
The audit is independent of issue status, so it remains reproducible after
closure.

## Validation

On the same ephemeral worker with Rust 1.97.1, the exact benchmarked code
passes:

- `cargo fmt --all -- --check`;
- all 4,418 library and 11 integration tests, 4,429 total;
- strict all-target/all-feature pedantic Clippy;
- the native optimized build of every Rust binary;
- compile-only Windows GNU x64 all-target/all-feature tests and every release
  binary; and
- PE32+ inspection of `eprover.exe`.

The native `eprover` SHA-256 is
`0d629df5e716b07fa13d26922da11407c7a2c865471ddd81bd9193bb52085ea9`;
the compile-only Windows GNU x64 executable SHA-256 is
`90aacc2c9f31874129a5a290cc380e796992e2fb5acf63e3e12be340005d9004`.
Against unchanged cached FOL/HO C references, the optimized Rust binaries pass
all 50 main-prover cases with zero unexpected differences and one declared
presentation difference, plus all 216 support-tool cases with zero unexpected
differences and 15 declared differences. The main and tool report SHA-256
digests are respectively
`2bff8f2d1299b8a3e5cf910270af33533bb5af731e2c272efcd50ae6965dc8c3`
and
`419fe460c193cdb2c6f00398514aa9885d174fd93bd1842c5e3c8df11ba22dc4`.

The six-record source audit, Python syntax check, C-source coverage,
documentation wording/link/manual-regeneration checks, and `git diff --check`
pass. The local C checkout is clean.
[`validation-reference.json`](validation-reference.json) pins the source,
binary, matrix-report, cross-compile, and test evidence. No Rust or C toolchain
ran on the local Windows host.

Reproduce the source audit locally:

```powershell
.\.venv\Scripts\python.exe experiments/2026-07-25-046-external-reconciliation/audit_external_reconciliation.py `
  --repo . `
  --expected experiments/2026-07-25-046-external-reconciliation/audit-reference.json
```
