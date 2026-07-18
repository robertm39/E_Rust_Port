# Comment-prefix ownership and compatibility decision

## Status

Completed for Bead `E_Rust_Port-j76.2.32`. The supported executable already
matches the pinned default WSL/Linux C build. No Rust runtime or CLI change is
needed, and the vendored C checkout remains unchanged.

## Ownership finding

The default C build deliberately has two spellings for the same conceptual
comment prefix:

- `COMCHAR` is `"%%"` because it is normally embedded in a `printf` format
  string and renders as one percent sign;
- `COMCHARRAW` is `"%"` for already-rendered strings; and
- `TSTPOUTFD` is an important exception: it passes `COMCHAR` directly to
  `WriteStr`, so hard-timeout descriptor output contains two percent signs.

Rust represents those actual output spellings as `DEFAULT_COMCHAR_RAW` and
`DEFAULT_COMCHAR_DIRECT`. Ordinary `eprover` stream output uses the first;
only the low-level status helper and signal-owned hard-timeout path use the
second. The existing unit regressions pin both `% SZS status Theorem` and
`%% SZS status ResourceOut`.

Flattening the two constants would be a compatibility regression. The live
deduction-server investigation in
[`../2026-07-17-044-deduction-server-run-framing/FINDINGS.md`](../2026-07-17-044-deduction-server-run-framing/FINDINGS.md)
also demonstrated that upstream code can accidentally use the printf-escaped
macro in a direct-string context. That evidence is why ownership is audited
rather than inferred from the macro name alone.

## Retained executable evidence

The source audit revalidates three independent unchanged-C/Rust production
references:

- 18/18 format-option cases, including 34 non-null C/Rust status lines that
  all start with one `%`;
- 15/15 proof-output cases, including proof frames, graphs, training output,
  and proof statistics; and
- 11/11 reporting, strategy, and limit cases, including resource-limit status
  behavior.

All 44 executable cases are byte-exact where the retained matrices define
exact comparison. The audit also bounds `DEFAULT_COMCHAR_DIRECT` to
`basics/defines.rs` and `inout/signals.rs`.

The retained [`owner-audit.json`](owner-audit.json) passes 18/18 checks and has
SHA-256 `4160EA4E94977E4483188D8995463938C2590887BDA2FC2373637F9BAFB80D8E`.

## Compatibility decision

The drop-in target is the pinned upstream default build. C's alternative `#`
prefix is a configure-time `--unix-comments` build, not a runtime option and
not a currently supported reference target. Rust therefore does not add a
speculative CLI flag or Cargo feature. If an alternate C build becomes a
supported target, it should receive an explicit compatibility mode plus its
own full executable reference matrix; the current raw/direct ownership split
must remain intact within the default mode.

## Reproduction

```powershell
C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-18-106-comment-prefix-ownership\audit_comment_prefixes.py `
  --expected experiments\2026-07-18-106-comment-prefix-ownership\owner-audit.json

cargo test --locked --all-features `
  basics::defines::tests::tstp_status_helpers_preserve_formatted_and_direct_comment_prefixes
```
