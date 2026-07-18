# Classic KBO Integration

## Scope

This experiment reconciles the migrated ordering gap around classic KBO, the
generic ordering dispatcher, proof-control ownership, and the listed RPO
surface. It uses the isolated upstream C reference at commit
`17026b1bfe61aaf223cfaae54947c8d2679c31a0`.

The source audit checks that C and Rust both:

- dereference before classic-KBO term weights and comparison;
- delay the whole-term variable condition until a possible strict result;
- dispatch classic KBO without requiring a mutable term bank, while reserving
  bank-backed dispatch for KBO6 and LPO4;
- preserve explicit classic KBO during higher-order proof-control setup; and
- keep RPO unavailable because upstream C itself contains three explicit
  `RPO not yet implemented!` assertion sites.

## Results

All 8 source/dispatch checks passed. Both executable comparisons were exact:

| Case | Result | Exit | Stdout SHA-256 |
| --- | --- | ---: | --- |
| FOL `--term-ordering=KBO` | exact | 0 | `901a3e88278126a4a02f3d7bd93527354189fcc2b46dfdc06fc0f937a71e30f9` |
| THF explicit classic KBO, optimized release surface | exact | 0 | `03b0577f032d430eca95b1efe043900820314778787ac73f6b5b3f21e7231703` |

The FOL case pins ordinary classic-KBO proof search. The THF case pins the
observable optimized-C behavior: the debug-only C assertion against classic
KBO in higher-order problem mode is compiled out, so an explicit user choice
continues through the legacy recursive comparison instead of being silently
changed to KBO6. Rust deliberately matches that release executable behavior.

The permanent unit regressions additionally prove that `DerefOnce` reaches a
bound term before classic comparison and that proof-control initialization
retains an explicitly selected classic-KBO OCB for a higher-order problem.

## Reproduction

Build the Rust release binary, then run:

```powershell
python experiments\2026-07-17-070-classic-kbo-integration\compare_kbo.py `
  --rust-exe target\release\eprover.exe `
  --c-exe /home/rober/.cache/e-rust-port/bin/17026b1bfe61aaf223cfaae54947c8d2679c31a0/fol/eprover `
  --c-ho-exe /home/rober/.cache/e-rust-port/src/17026b1bfe61aaf223cfaae54947c8d2679c31a0-ho-20260717b/PROVER/eprover-ho `
  --distro Ubuntu-24.04 `
  --output experiments\2026-07-17-070-classic-kbo-integration\results.json
```

## Compatibility decision

Classic KBO, KBO6/LPO4 bank routing, and proof-control ordering ownership are
complete for the represented production paths. The remaining RPO wording in
the migrated gap was stale: no RPO algorithm exists in the upstream source, so
implementing one would be a post-compatibility extension rather than port work.
The production banked-callback and owner-bank claims are independently covered
by experiments 053, 055, and 060.
