# Official CASC LTB specification corpus

## Status

Accepted for Bead `E_Rust_Port-j76.1.46`. The runner can execute concrete
problems, and its loose batch grammar is now pinned against three authentic
official CASC samples rather than only synthetic strings. The accepted current
forms match the source-level C parser contract; the older incompatible spelling
is retained as an exact rejection regression. The vendored C source remained
unchanged.

## Official reference data

The official CASC design pages link these batch files directly:

- CASC-28 JJT: `BatchSampleLTBJJT.txt`, 2,850 upstream bytes,
  SHA-256 `e85f9ccff5b281b6adf96fa3d4f4467c849a67e5d32432e80deeb0b45e661083`;
- CASC-J11 VBT: `BatchSpec.VBT.txt`, 29,751 upstream bytes,
  SHA-256 `14f505ac10d1782187bb20f8a82bb97bd73d8f47e07c728aeba1910d037bd295`;
  and
- the sample served through the CASC-J8 `BatchSampleLTBHLL` link, 1,410
  upstream bytes, SHA-256
  `36013f79ea453cbc3bed7230f1a8728e9d3e2e2603f2f449144f3f03b7466f96`.

The files are checked in byte-for-byte under `tests/fixtures/ltb/official/`.
`PROVENANCE.md` records direct official URLs, sizes, hashes, and the LF-stable
repository attribute that preserves those bytes across platforms.

## Accepted current grammar

The CASC-28 JJT and CASC-J11 VBT files use the current runner's
`division.category.training_data` spelling. They also exercise the actual
competition layout that motivated the deliberately loose parser:

- `% SZS start/end` section markers are comments to the scanner rather than
  grammar terminals;
- `execution.order unordered` is accepted;
- required proof output and zero per-problem limits are parsed;
- overall limits are 450 and 3,000 seconds;
- abstract problem filenames retain `*` version globbing;
- destination names have no required extension; and
- trailing `% starexec-dependency` lines remain comments through EOF.

Permanent unit regressions parse all 10 nonconsecutive JJT problem/output pairs
and all 100 sequential VBT pairs. They pin categories, training archives,
ordering, output requirements, limits, empty include sets, every abstract
source/destination name, no parse notices, and final scanner EOF.

## Historical spelling boundary

The smaller file served through the older HLL sample link currently identifies
`LTB.HOL`, has two batches and shared includes, and uses
`division.category.training_directory`. Current
`PROVER/e_ltb_runner.c` contains that exact accepted-id call only as a comment
and actively calls `AcceptDottedId(..., "division.category.training_data")`.
The current C executable therefore rejects the official older sample at the
header before reaching either batch.

Rust already had the same current-C boundary. A permanent regression now feeds
the unmodified sample and requires a syntax error naming both the expected
`training_data` field and the observed `training_directory` token. Widening the
runner to accept the older spelling would be a product extension, not drop-in
parity with the vendored source.

## Reference environment

This Windows host has no installed WSL distribution and no C compiler, so a
live C binary could not be rerun. Accepted field order and the historical
rejection are instead read directly from `e_ltb_runner.c` and
`cco_batch_spec.c`; the Rust fixtures execute as native parser regressions. The
official sample bytes and hashes make a later live C rerun reproducible without
relying on search results or reconstructed syntax.

## Performance decision

Parsing the complete 712-line VBT file finishes below the test timer's 0.01
second display resolution. This slice adds compile-time fixture inclusion and
test-only assertions, with no production path change; a runtime benchmark is
not warranted.

## Validation

- official CASC-28 JJT parse: 10 exact problem/output pairs passed;
- official CASC-J11 VBT parse: 100 exact problem/output pairs passed;
- official older HOL spelling rejection: exact error class/tokens passed;
- full library suite: 4,168 passed;
- all binary targets passed;
- integration targets `eprover_schedule`, `e_stratpar`,
  `executable_inventory`, and `e_ltb_variant_worker`: 4, 1, 1, and 1 passed;
- `cargo check --locked --all-targets`: passed;
- `cargo clippy --locked --all-targets -- -D warnings`: passed;
- `cargo fmt --all -- --check`: passed;
- bundled-Python `tools/e-interop` discovery: 32 passed;
- `git diff --check`: passed;
- C-source documentation coverage: 492 sources / 266 unit docs;
- Change Later wording and local-link checks: 269 Markdown files each; and
- regeneration preservation: 268 Markdown files.
