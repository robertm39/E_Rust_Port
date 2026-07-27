# Experiment 329: Compact term arguments

## Status

Complete and accepted for Bead `E_Rust_Port-j76.5.5`. This closes the
maintained main-prover performance target.

## Question

Can the final per-term `RefCell`, which owns the inline-or-heap argument
representation, become one private single-threaded `UnsafeCell` boundary while
preserving the existing safe clone/copy/set surface, contracted internal slice
access, exact behavior, and improved performance?

## Baseline

- Accepted production: Experiment 328 at commit `e8f8f948`.
- Matched accepted LUSK6 work: `7,138,890,703` instructions on this worker.
- The latest comprehensive aggregate was `1.1027921183x` C.
- `TermArgs` is 24 bytes, but `RefCell<TermArgs>` is 32 bytes. This kept every
  64-bit `TermCell` at 144 bytes and placed dynamic borrow bookkeeping on all
  argument reads and writes.
- All prior metadata fields already used measured compact representations, so
  argument storage was the last per-node dynamic borrow boundary.

## Candidate

`TermArguments(UnsafeCell<TermArgs>)` retains the existing empty, unary,
binary, and heap representations in 24 bytes. Safe accessors only:

- copy arity;
- clone one or all argument owners; or
- replace an argument slot.

No safe API exposes a reference into the cell. `Term` is `Rc`-backed and
therefore single-threaded. Private borrowed-slice access is unsafe: each of the
28 call sites retains its owner graph and explicitly prohibits structural
mutation until the slice dies. Mutable slice access is limited to freshly
allocated, unshared copies that are distinct from any borrowed source.

The 64-bit layout regression pins a 24-byte `TermArguments`, a 136-byte
`TermCell`, the inline/heap representation, opaque debug behavior, cloned-owner
lifetimes, and rejection of mutable access after sharing.

## Setup and exact commands

Focused measurement used dedicated Ubuntu 24.04 worker
`e-rust-codex-260726-163244-77ac` with Rust 1.97.1 and Callgrind 3.22.0. The
accepted parent archive SHA-256 was
`E6AD73460438B9E1BF01478F5969444CEB5B5C429F8BF5A5AC0AD444DFDC1EB8`.
The exact candidate source hashes were:

```text
c335defbc2246b1e82d1c1c08b0a08b2f51d6be8eeb7dacb2cb0879b081ab4eb  src/clauses/pdtrees.rs
10a0dd6ea26e53a56f0730f3fc0b49bcb65001f1f8def1855c2e8480060e3086  src/clauses/rewrite.rs
8daa07e572c622c208d3c9fa70770e8ab357dea43d2dbea0192f9883cc5dd963  src/heuristics/diversityweight.rs
06c7cd35dcb2de1986bf6f20e6ff5669d4c1e0265552c590213c072f627c9abc  src/orderings/cto_kbolin.rs
0892077ef720e2964dcc3ce99bff6a1b9d4d6d07aa10e9dd4386e7b28797f227  src/terms/termbanks.rs
715249565de4e6dca26f9563859ba838cbf78cb51745aa55422b5e02de198538  src/terms/termcellstore.rs
44e538b621e28b10ff171314a4697ac5ca545aaff2167799cb63dbc9853b10f4  src/terms/termfunc.rs
e1050c6aae6e9cecfa8c50d420a2d51d8d1cdcd9b8d6dabb2debd8fadf16e6e3  src/terms/termtrees.rs
8cd2787cff0f83e27d8664d1290b8cc374303873107ae4ea9e16bbf05ccecbb7  src/terms/termtypes.rs
```

The focused lifecycle was:

```powershell
git archive --format=tar --output=accepted-source.tar e8f8f948 `
  src/clauses/pdtrees.rs `
  src/clauses/rewrite.rs `
  src/heuristics/diversityweight.rs `
  src/orderings/cto_kbolin.rs `
  src/terms/termbanks.rs `
  src/terms/termcellstore.rs `
  src/terms/termfunc.rs `
  src/terms/termtrees.rs `
  src/terms/termtypes.rs
.\linode-runner.ps1 up
try {
    .\linode-runner.ps1 sync
    .\linode-runner.ps1 exec -- bash `
      /opt/e-rust-port/source/experiments/2026-07-25-028-compact-term-arguments/remote_measure.sh `
      /opt/e-rust-port/source `
      /opt/e-rust-port/artifacts/experiment-329
    .\linode-runner.ps1 exec -- bash `
      /opt/e-rust-port/source/experiments/2026-07-25-028-compact-term-arguments/remote_repeat.sh `
      /opt/e-rust-port/source `
      /opt/e-rust-port/artifacts/experiment-329
    .\.venv\Scripts\python.exe -c `
      "import sys; sys.path.insert(0, r'tools/linode-runner'); import linode_runner as lr; state=lr.load_current(); state['remote_artifact_path']='/opt/e-rust-port/artifacts/experiment-329'; lr.save_current(state); print(lr.collect_artifacts(state))"
}
finally {
    .\linode-runner.ps1 down
}
```

The first two focused invocations exposed one Rustfmt difference, the
unneeded `Default` derive, and two strict-Clippy `borrow_deref_ref` findings.
Those were corrected before any measurement. Only the final source hashes
above entered either profile or native block.

Both exact profiles used:

```bash
valgrind --tool=callgrind \
  BINARY eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```

Fresh exact-source comprehensive validation used worker
`e-rust-codex-260726-165105-e907`:

```powershell
.\linode-runner.ps1 run
```

## Falsification criteria

- `TermArguments` must remain one three-word cell and `TermCell` must shrink
  from 144 to 136 bytes on 64-bit targets.
- No safe API may expose a reference into argument storage.
- Every unsafe shared borrow must retain all owners and prohibit overlapping
  argument mutation; every mutable borrow must target a fresh unshared term.
- Argument representation, lifetime, panic, term-bank, rewrite, PD-tree, KBO,
  traversal, hashing, and comparison tests must pass under strict Clippy.
- Parent/candidate proof output and program stderr must be byte-identical.
- Exact work and native timing must improve in both independent blocks and
  their stable halves.
- A positive focused result must pass the complete Linux/Windows/C,
  compatibility, resource, and benchmark lifecycle before acceptance.

## Results

Rustfmt, strict all-target/all-feature pedantic Clippy, and all focused modules
pass:

- 21 term-type tests;
- 47 term-function tests;
- 13 substitution tests;
- seven term-store tests;
- six term-tree tests;
- 125 term-bank tests;
- 21 KBO6 tests;
- 33 clause-rewrite tests;
- 44 PD-tree tests; and
- six diversity-weight tests.

Parent and candidate exit zero with empty program stderr and byte-identical
LUSK6 proof output, SHA-256
`b50bfb29a2fa3728792a422be2bd83a54c436fa1536c79b8d95c1c1f8c7f427d`.

| Build | SHA-256 prefix | Bytes | Callgrind instructions |
| --- | --- | ---: | ---: |
| Parent | `64f3f47532a1` | 8,236,584 | 7,138,890,703 |
| Candidate | `2a647280704d` | 8,181,520 | 6,911,634,000 |

Exact same-worker work falls by 227,256,703 instructions (`3.183362%`), and
the candidate executable shrinks by 55,064 bytes (`0.668530%`).

Two independent 64-pair blocks provide 128 alternating native pairs. Every
run has the exact proof hash. The candidate wins 123 wall and CPU pairs and 63
of the 64 combined stable-half pairs:

| Native improvement | Block 1 | Block 2 | All 128 | Combined stable 64 |
| --- | ---: | ---: | ---: | ---: |
| Paired mean wall | `4.129300%` | `3.779208%` | `3.954254%` | `3.923575%` |
| Paired mean CPU | `4.128687%` | `3.782612%` | `3.955650%` | `3.927353%` |
| Paired median wall | | | `3.836343%` | `3.658586%` |
| Paired median CPU | | | `3.839720%` | `3.657880%` |
| Candidate wins | 61 | 62 | 123 | 63 |

Fresh comprehensive run `.artifacts/linode/260726-165105-e907/` validates the
exact focused candidate binary SHA-256
`2a647280704d8a560b1542bcc66c2c2138cfd154b6860b5e1100a3828a6737a1`:

- 4,419 Rust tests pass: 4,408 library and 11 integration tests;
- Rustfmt, strict pedantic Clippy, and the native release build pass;
- every binary and test target cross-compiles for Windows GNU x64;
- clean pinned first-order and higher-order C references build and pass smoke
  checks;
- all 50 main cases have zero unexpected differences and one declared
  difference;
- all 216 support-tool cases have zero unexpected differences and 15 declared
  differences;
- all ten benchmark cases preserve behavior;
- smoke Callgrind records 9,559,737 Rust versus 7,590,630 C instructions; and
- the ten-case aggregate improves from `1.1027921183x` to
  `1.0801753448x` C.

`VALIDATION_COMPLETE` and `SUCCESS` both contain `ok`. Both measured workers
and firewalls were deleted after artifact collection. The documentation
coverage, Change Later wording, local-link, and manual-regeneration validators
also pass across the 269 indexed Markdown files.

## Conclusion

Accept. This is the concrete measured-performance case permitted by the
project's unsafe policy. It removes the last per-term dynamic borrow flag and
eight bytes from every 64-bit term node while keeping the public surface safe,
cloning owners before they escape, and containing every borrowed reference
behind an explicit owner/no-mutation contract.

The candidate improves exact LUSK6 work by 3.18% and native LUSK6 time by about
3.95% across independent blocks and stable halves. The maintained fresh-worker
aggregate is now `1.0801753448x` C, below the normal `1.10x` comparable-
performance threshold with exact behavior and no resource or compatibility
regression. Bead `E_Rust_Port-j76.5.5` and the compatibility/performance
milestone can close.

Raw evidence:

```text
.artifacts/linode/260726-163244-77ac/
.artifacts/linode/260726-165105-e907/
```

## Limits

- `TermArguments::shared` and `Term::arguments` remain unsafe internal APIs.
  Adding a caller requires an owner-liveness and no-overlapping-structural-
  mutation audit.
- `TermArguments::mutable` and `Term::arguments_mut` are valid only for
  unshared terms with no overlapping argument access; production callers
  currently use fresh distinct copies.
- `Term` deliberately remains `Rc`-backed and single-threaded. Replacing it
  with a cross-thread owner would invalidate the cell justification and
  require a synchronization design.
- The opaque debug representation deliberately avoids reading argument owners.
- Fresh-worker aggregate timing remains load-sensitive. Same-worker exact work
  and alternating native pairs are the causal evidence; the comprehensive run
  verifies that the maintained aggregate clears the project threshold.
