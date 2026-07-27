# E Rust port completion

## Status

Complete. The canonical migrated namespace `E_Rust_Port-j76` contains 2,176
records: the root plus 2,175 descendants. Every record is closed. Its seven
direct children cover explicit Pending work, known gaps, summarized
post-compatibility reviews, detailed C-source reviews, the compatibility and
performance milestone, the strict Clippy restoration, and the unsafe-Rust
policy decision.

The only non-closed Beads record in the repository is
`E_Rust_Port-c4w`, an independent Windows/Codex sandbox-latency maintenance
task. It is not a child of the port namespace and does not own a source,
feature, compatibility, performance, test, or quality-policy gap.

## Final compatibility state

- The 50-case maintained main-prover matrix has zero unexpected differences.
- The 216-case support-tool matrix has zero unexpected differences.
- Native main-prover aggregate wall time is `1.0801753448x` C, within the
  project's `1.10x` comparable-performance threshold.
- The final executable-source lifecycle passes 4,430 Rust tests, Rustfmt,
  strict all-target/all-feature pedantic Clippy, native and Windows GNU x64
  builds, clean FOL and higher-order C builds, and all compatibility and
  behavior benchmarks.
- The vendored `eprover/` checkout remains unchanged.

## Final backlog state

The five nested migration owners contain the following direct child counts,
all closed:

| Owner | Children |
| --- | ---: |
| explicit Pending | 47 |
| known gaps | 140 |
| summarized revisits | 649 |
| detailed Change Later reviews | 1,327 |
| compatibility/performance | 5 |

The detailed review count reflects the actual migrated database, including the
two later detailed records beyond the original 1,325-item summary text.

## Audit

[`audit_port_completion.py`](audit_port_completion.py) independently reads the
live Beads database and asserts:

- 2,176/2,176 namespace records are closed;
- the root has exactly the seven intended direct children;
- each nested owner has the expected child count and zero non-closed records;
- the compatibility and detailed-review epics carry their final evidence;
- the final lifecycle has 50 main and 216 tool cases with zero unexpected
  differences and 4,430 passing tests; and
- the status ledger and final CLAUSES audit agree with the closure state.

The reference digests every namespace identity, status, and close reason plus
the final source/evidence ledger. Issue status outside `E_Rust_Port-j76` is
deliberately excluded.

Reproduce locally:

```powershell
.\.venv\Scripts\python.exe experiments/2026-07-25-055-port-completion/audit_port_completion.py `
  --repo . `
  --expected experiments/2026-07-25-055-port-completion/audit-reference.json
```
