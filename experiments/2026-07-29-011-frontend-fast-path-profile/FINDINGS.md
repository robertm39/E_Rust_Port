# Frontend fast-path profile

## Result

Adopt the THF owner-routing short circuit.

The frozen current-code profile found two distinct frontend regimes:

- CNF, FOF, and TFF are clausification-dominated at useful sizes; and
- THF was parser-dominated because its unconditional represented-owner route
  still ran three first-order `$distinct` parser probes for every formula.

One early return removes work that cannot affect the THF routing decision.
Held-out 10,000-formula THF syntax time fell from 1.54 to 0.04 seconds
(`97.4%`), full CNF frontend time fell from 1.77 to 0.27 seconds (`84.7%`),
and DHAT allocation fell from 6.230 GB to 8.742 MB (`99.86%`). Exact TSTP CNF
and ancestry output, compatibility, downstream solve behavior, and all project
gates remained unchanged.

## Frozen setup

The preregistration and controller were committed before measurement as
`19c95bbc`. The baseline executable SHA-256 was
`95d99eb37324b0162c59931d1a28048a594d397c79c4755f47e0014c8f65fc2e`;
the candidate was
`d34912867aad36a621cd83fb9ed19c774e4f184d9be3f70fd3cd91da2556aa8a`.
The comparison reference was E commit
`17026b1bfe61aaf223cfaae54947c8d2679c31a0`.

All prover work ran sequentially on the same Ubuntu 24.04 US-IAD Linode with
Rust 1.97.1, GCC 13.3.0, and Valgrind 3.22.0. The baseline source snapshot ID
was `9507818625a02e9d2e71389ff8513d29739bedaeff1033be849a517e2cdf74b6`;
the comprehensive candidate snapshot ID was
`f61cbb54d77487f14560515d858e33c800118e74c56a4aa7cdc0ec756a0b3d7d`.

The deterministic 12-file corpus manifest ID was
`77306fde75087a7c1e0d6b72a2cc91ffe6c3de253b314b43726701e2a35b121e`.
It contains 1,000, 10,000, and 50,000 formula strata for CNF, FOF, TFF, and
THF. Every corpus parsed in Umlaut and the assigned FOL or HO E reference.
Five timed samples were collected per implementation, dialect, size, and
frontend mode after warm-up.

## Baseline phase profile

The table reports 50,000-formula medians. Phase values are differences between
startup, syntax-only, CNF/no-preprocessing, and full-CNF endpoints. GNU time's
10 ms reporting resolution is material only in the smallest stratum.

| Implementation | Dialect | Full CNF | Parse | Clausify | Preprocess | Full-CNF peak RSS |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| Umlaut | CNF | 0.96 s | 0.34 s | 0.45 s | 0.17 s | 149,084 KiB |
| Umlaut | FOF | 1.41 s | 0.24 s | 1.06 s | 0.11 s | 214,660 KiB |
| Umlaut | TFF | 1.44 s | 0.26 s | 1.07 s | 0.11 s | 215,408 KiB |
| Umlaut | THF | 9.41 s | 7.88 s | 1.44 s | 0.09 s | 236,032 KiB |
| E | CNF | 0.46 s | 0.19 s | 0.19 s | 0.08 s | 90,788 KiB |
| E | FOF | 4.57 s | 0.13 s | 4.44 s | 0.00 s | 145,204 KiB |
| E | TFF | 4.75 s | 0.14 s | 4.51 s | 0.10 s | 145,608 KiB |
| E-HO | THF | 39.18 s | 0.15 s | 42.94 s | 0.00 s | 165,712 KiB |

Negative reference phase differences from independent process medians were
reported raw and clamped only for fractions, as preregistered. They explain why
the E-HO component estimates can sum above the full endpoint.

The timing rule selected THF parsing: it was 88.2% of the 1,000-formula
frontend, 87.5% of held-out 10,000-formula frontend, and 83.7% of the
50,000-formula frontend. FOF/TFF clausification was separately dominant, but
THF parsing had the largest eligible absolute cost.

## Allocation and instruction diagnosis

Held-out baseline DHAT measurements were:

| Endpoint | Total bytes / blocks | Peak live bytes / blocks | End live bytes / blocks | Max RSS under DHAT |
| --- | ---: | ---: | ---: | ---: |
| Syntax | 6,229,794,685 / 32,830,386 | 5,799,045 / 35,691 | 1,901,438 / 23,949 | 58,448 KiB |
| CNF, no preprocessing | 6,289,494,980 / 33,096,131 | 47,493,301 / 290,567 | 33,240,449 / 277,215 | 224,076 KiB |
| Full CNF | 6,289,182,973 / 33,091,493 | 46,924,008 / 286,443 | 32,926,487 / 273,093 | 222,092 KiB |

Callgrind recorded 26,112,669,034 instructions for syntax-only. Its inclusive
profile attributed 97.28% to
`should_parse_tstp_formula_as_represented_owner`, below the normal input-parser
call chain.

The source inspection matched the profile. Once the destination is a formula
set, the function's final match always returns `true` for `thf`. Before reaching
that match, however, it ran direct, negated, and parenthesized `$distinct`
compatibility probes. Those probes clone scanners and parse against detached
term banks. They exist for ambiguous first-order routing and cannot alter the
THF decision.

## Prototype and held-out result

The prototype returns `true` for `thf` immediately after the destination gate.
It changes neither the represented THF parser nor any formula, type, term,
clause, or ancestry owner. A focused regression covers print and CNF
destinations, ordinary application, lambda, and `$distinct` shapes with a
4,096-symbol live signature.

| Held-out 10,000 THF metric | Baseline | Candidate | Change |
| --- | ---: | ---: | ---: |
| Syntax wall | 1.54 s | 0.04 s | -97.40% |
| Syntax CPU | 1.53 s | 0.04 s | -97.39% |
| Syntax peak RSS | 11,448 KiB | 10,980 KiB | -4.09% |
| Full CNF wall | 1.77 s | 0.27 s | -84.75% |
| Full CNF CPU | 1.77 s | 0.26 s | -85.31% |
| Full CNF peak RSS | 54,816 KiB | 54,824 KiB | +0.01% |
| DHAT allocated bytes | 6,229,794,685 | 8,741,804 | -99.86% |
| DHAT allocated blocks | 32,830,386 | 250,289 | -99.24% |
| DHAT peak-live bytes | 5,799,045 | 5,766,164 | -0.57% |

At 50,000 formulas, syntax fell from 7.88 to 0.23 seconds and full CNF from
9.45 to 1.79 seconds. Every non-THF wall median was unchanged at GNU time's
resolution. All 36 dialect/size/mode groups completed with the same exit
behavior.

## Output, origins, and downstream effects

Two baseline executions of every 1,000-formula corpus produced byte-identical
TSTP output before profiling. The inventory included 1,000 CNF records for CNF,
2,000 for FOF, 2,515 for TFF, and 1,000 to 5,545 `inference(...)` records per
dialect. THF retains higher-order records rather than `cnf(...)` records.

The candidate then matched the baseline byte for byte on all four dialects at
both 1,000 and held-out 10,000 formulas: eight exact output hashes, zero
differences. Thus formula names, generated names, ordering, clausification, and
proof origins are unchanged.

The comprehensive candidate run reported:

- 50 main-executable cases: zero unexpected mismatches and 29 hash-pinned
  expected differences;
- 216 support-tool cases: zero unexpected mismatches and 16 expected
  differences; and
- 10 standard solve cases: zero behavior mismatches.

For direct downstream timing context, the immediately preceding clean
`260729-133617-70cc` benchmark had Rust/C aggregate wall ratio `1.078293`;
the candidate run measured `1.079722`, a 0.13% relative change. Every candidate
case was within 1.13% of its preceding Rust median; the long BOO020, LUSK6, and
LUSK6ext cases were 7.1%, 4.6%, and 4.5% faster respectively. Both runs had
identical status behavior, and the hash-pinned 50-case compatibility gate
accepted every benchmark case's output contract.

These results satisfy every preregistered adoption threshold.

## Full validation

The comprehensive Ubuntu run completed with both `VALIDATION_COMPLETE` and
`SUCCESS`:

- formatting passed;
- 4,482 all-feature library tests and every binary/integration test passed;
- strict all-target/all-feature Clippy passed;
- all Linux release binaries built;
- 42 independent validation-controller tests passed, with the opt-in external
  Z3 probe skipped;
- Windows GNU all-feature test targets and release binaries cross-compiled;
- both compatibility matrices and the ten-case benchmark passed; and
- Rust/C Callgrind smoke runs completed with 9,610,442 and 7,588,996
  instructions.

## Evidence

The ignored local archive is
`.artifacts/experiments/2026-07-29-011-frontend-fast-path-profile/evidence.tar.gz`.
It contains the generated manifest, all raw timing samples, exact output
artifacts, DHAT JSON, raw and annotated Callgrind profiles, candidate
comparison, and complete comprehensive validation directory.

| Artifact | SHA-256 |
| --- | --- |
| Evidence archive | `d39a11aa28d67091929b5b81312d423189b2ad70afbe5397b791828905d741af` |
| Baseline analysis | `4fd1bf4ea6a55d49a09f02b9ba4380f83e5c6c42437ca8497d0870914d8d5c21` |
| Baseline profile report | `10e579035dde8f5b3de0be8e0d1365acd655bfa558585b3439b44d357f86ab0c` |
| Candidate report | `33a123590c7fc1fb62f5bba1d58876a21dfd6e59bdef86af53a509a1e1aab577` |
| Candidate validation summary | `3b997cde23c76d6bbca129dbe1d4bc396e74ddfd75ac6a47884d77159f42f9a5` |
| Preceding benchmark | `61df3493aa04c8c5583ea33c02001ac640cc3e1805baf4c715bfdfde1681ae8b` |
| Candidate benchmark | `3cbf4e0f5172535f75da3f8a55561504f65f03cb19965b9b621e7f7e7497f023` |

