# Main-executable matrix baseline

## Status

Completed as the durable replacement for the stale comparison in Bead
`E_Rust_Port-j76.2.10`. A fresh run against the unchanged archived C commit
covers all 50 configured cases. Forty-five cases are exact, one has an exact
declared nonsemantic output difference, and four remain unexpected
resource/performance failures. The vendored C checkout remains unchanged.

## Declared compatibility decision

Both `sledgehammer.p` executables exit successfully with `Theorem`; only the
normalized proof text differs. Earlier controlled investigation found ten
same-sort binder permutations whose order follows allocator-address ties in C,
not a stable textual requirement. The main harness therefore declares exactly
`normalized_stdout` for this case. A missing difference or an added field still
fails the matrix.

## Unresolved failures

- `BOO020-1.p`: C returns `ResourceOut`/8 after its limit; Rust terminates from
  allocator failure with no SZS status and exit 9.
- `HEN011-2.p`: C proves `Unsatisfiable`; Rust reaches the 60-second CPU limit.
- `SWV851-1.p`: C returns `ResourceOut`/8 after its limit; Rust terminates from
  allocator failure with no SZS status and exit 9.
- `synthetic/cpu-limit-LUSK6.lop`: C proves `Unsatisfiable` below one second;
  Rust reaches the one-second CPU limit.

These remain compatibility failures and are transferred to a dedicated
proof-search resource/performance parity Bead. Experiment 128 rejected a
chunked clause store: although it removed the single 384 MiB contiguous growth
request, BOO020 peak working set rose by 73.87 MiB and the process still
terminated on a later allocation.

## Retained evidence

The complete volatile report is
`.artifacts/e-compare/20260719-025033-940384/comparison.json`.
[`reference.json`](reference.json) retains its stable case inventory, archived
commit, and exact difference fields. [`audit_main_matrix.py`](audit_main_matrix.py)
regenerates that projection and rejects inventory, archived-commit, or
difference-shape drift.

## Reproduction

```powershell
cargo build --locked --release --bin eprover --target-dir target\default-reference
.\e-interop.ps1 compare -RustExe .\target\default-reference\release\eprover.exe

C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-18-129-main-matrix-baseline\audit_main_matrix.py `
  --report .artifacts\e-compare\20260719-025033-940384\comparison.json `
  --output target\main-matrix-summary-check.json `
  --expected experiments\2026-07-18-129-main-matrix-baseline\reference.json
```
