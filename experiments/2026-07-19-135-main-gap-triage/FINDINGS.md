# Main proof-search gap triage

## Question

Are the remaining maintained-matrix differences stable implementation gaps, or
do GEO288 and `LUSK6ext` switch outcomes or proof ancestry across fresh
processes of the same committed production binary?

## Setup

- Rust commit: `3909f192`.
- Rust binary: `target/compact-final/release/eprover.exe`, SHA-256
  `C740DCA46C90BA2FBECB6270D3D1BA06B2D69473759705788577D7BC193C8333`.
- C reference: unchanged archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`.
- Shared arguments: `--auto --silent --cpu-limit=60 --memory-limit=2048
  --detsort-rw --detsort-new --proof-object=1`.

The ignored focused corpus is staged from the unchanged upstream fixtures with:

```powershell
& experiments\2026-07-19-135-main-gap-triage\stage_corpus.ps1
```

Each focused C/Rust comparison is then run from WSL with:

```text
python3 tools/e-interop/e_interop.py compare --repo-root . \
  --rust-windows target/compact-final/release/eprover.exe \
  --corpus .artifacts/e-corpus/main-gap-triage-135 \
  --timeout 60 --memory-limit-mb 2048
```

## Results

Three fresh comparisons of the committed candidate are retained at:

- `.artifacts/e-compare/20260719-085513-182347/`;
- `.artifacts/e-compare/20260719-085656-793806/`; and
- `.artifacts/e-compare/20260719-085837-813060/`.

All three runs are identical in behavior. GEO288 reaches `ResourceOut`/8 after
60.86-60.94 seconds of harness wall time. `LUSK6ext` proves in 9.06-9.62
seconds with exact normalized C output every time. The committed candidate's
current `LUSK6ext` proof is therefore stable in this sample; the earlier
proof-text switching remains a cross-layout observation, not a reproduced
same-binary instability.

The same committed candidate and pre-slice `9aac4a20` binary were then given a
90-second native headroom limit:

| Binary | CPU | Wall | Sampled peak RSS | Outcome |
| --- | ---: | ---: | ---: | --- |
| `9aac4a20` baseline | 62.593750 s | 64.531690 s | 326,664 KiB | Theorem |
| committed 4,096-page candidate | 64.140625 s | 64.710732 s | 313,984 KiB | Theorem |

The resource slice saves 12,680 KiB on this run but costs 2.47% CPU. The
focused baseline report at `.artifacts/e-compare/20260719-091231-332968/`
still proves GEO288 at the maintained limit, so the current difference is not
only a host-load artifact. Raw measurements are in
[`geo288-baseline-90.csv`](geo288-baseline-90.csv) and
[`geo288-current-90.csv`](geo288-current-90.csv).

Several bounded representation ablations were rejected:

| Ablation | CPU | Peak RSS | 60-second focused result |
| --- | ---: | ---: | --- |
| 65,536-header pages | 62.390625 s | 312,908 KiB | GEO `ResourceOut`; LUSK6ext exact |
| 65,536 pages plus inline derivation | 61.796875 s | 312,864 KiB | GEO `ResourceOut`; LUSK6ext exact |
| retained outer-page binary | not reprofiled | not reprofiled | GEO `ResourceOut`; LUSK6ext proof-text difference |

The page and derivation results are single diagnostic samples, not stable
percentage claims. Neither closes the maintained cutoff, inline derivations
give back the 32-byte clause-header saving needed by BOO020/SWV851, and the
outer-page binary reintroduces the proof-order symptom. Reports are
`.artifacts/e-compare/20260719-091045-959361/`,
`.artifacts/e-compare/20260719-091841-142139/`, and
`.artifacts/e-compare/20260719-092256-821521/`; the small raw CSVs are retained
beside this note.

The retained LUSK6 Callgrind profile attributes 9.83% of instructions to the
inlined `PdTree::record_search_init` path. A final ablation removed independent
`debug_assert_eq!` recomputation from `prefix_query_metadata`. It was rejected:
production release builds disable debug assertions, the independent
classification test already owns that validation, and the apparent timing
movement was noise or binary-layout variance rather than removed production
work. Both the exploratory-layout and restored-layout variants still timed out
at 60 seconds; the restored variant also timed out at 61 seconds. No source
change from this experiment is retained.

## Falsification checks

- Repeating the same committed binary three times separates stable current
  behavior from earlier cross-layout proof switching.
- A fresh pre-slice run under the same WSL harness proves both focused cases,
  so the GEO difference is candidate-visible even though its absolute timing
  is cutoff sensitive.
- Giving every native ablation 90 seconds confirms that all follow the same
  theorem path and emit the same 17,402-byte output; the 60-second reports then
  test the actual compatibility boundary separately.
- `LUSK6ext` stays exact for the committed, 64K-page, inline-derivation, and
  metadata-check variants. The preserved outer-page binary is the sole fresh
  proof-text mismatch.
- The staging script runs from the repository root and copies only the two
  unchanged upstream fixtures into an ignored artifact directory.
- Every exploratory source change was restored; `git diff` contains no
  production change from this experiment.

## Conclusion

The committed resource representation introduces a small measurable GEO288
CPU cost while substantially lowering RSS, and GEO288 is too close to the
maintained cutoff for single full-proof timings to govern another layout
change. Larger pages and inline derivations cannot be accepted because they do
not close the compatibility result and weaken the resource proof. The final
candidate's `LUSK6ext` proof is stable across three fresh processes.

Continue with a deterministic instruction- or fixed-prefix profile of a real
hot owner shared by GEO288, HEN011, and LUSK6. Do not classify GEO288 as an
expected difference, and do not change the resource layout merely to exchange
GEO or LUSK proof behavior at a noisy boundary.
