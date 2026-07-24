# Experiment 297: Reject single-entry variable-bank lookups

## Status

Rejected performance candidate for Bead `E_Rust_Port-j76.5.3`; accepted
Experiment 293 production source is restored byte-for-byte.

## Question

Can `VarBank::get_fresh_var` keep its ordered-map representations while
performing only one lookup for the per-type cursor and one lookup for the
per-type variable stack?

## Candidate

The accepted existing-variable path performs:

1. `v_counts.get()` followed by `v_counts.insert()`;
2. `varstacks.entry()` to read the length followed by `varstacks.get()` to
   fetch the selected variable.

The candidate destructures the already mutably borrowed `VarBankCell`, retains
one mutable `BTreeMap::entry` for each map, reads and advances the cursor
through its retained entry, and selects the variable through the retained
stack entry. It preserves the ordered maps, paged variable table, allocation
condition, fresh-code chronology, shadow synchronization, and return values.

Experiment 218 also reused the stack entry, but bundled it with a
representation change from the cursor `BTreeMap` to a dense `Vec`. Its native
rejection therefore did not decide this isolated representation-preserving
boundary.

## Validation

- All 17 focused variable-bank tests pass with default and all features.
- Strict all-feature library pedantic Clippy and formatting pass.
- Native and WSL fingerprints both record exactly `features=["default"]`.
- Three parent and eight candidate proof runs all exit zero, emit empty
  stderr, and produce the same 378-byte stdout with SHA-256
  `b50bfb29a2fa3728792a422be2bd83a54c436fa1536c79b8d95c1c1f8c7f427d`.
- Every Callgrind, warmup, and measured native process proves and exits zero.

## Deterministic result

Exact default-feature LUSK6 Callgrind improves from accepted Experiment 293's
`8,718,487,029` instructions to `8,694,012,980`:

- delta: `-24,474,049`;
- improvement: `-0.280714%`;
- hypothetical Rust/C ratio: `1.654628`, versus `1.659286`.

The intended `VarBank::get_fresh_var` owner falls from `54,637,167` to
`40,472,090` instructions, saving `14,165,077` or `25.928%`. Substitution
normalization remains exactly `444,445,091` instructions; neighboring major
owners are stable within ordinary whole-binary layout movement.

Raw profile:

```text
.artifacts/experiments/2026-07-24-024-single-varbank-entry/callgrind-single-varbank-entry.out
```

## Native result

Each 64-pair block has four separate alternating warmup pairs. All 256
measured processes prove and exit zero. Positive percentages mean the
candidate is slower.

| Sample | Wall mean | CPU mean | Paired wall mean | Paired CPU mean |
| --- | ---: | ---: | ---: | ---: |
| Block 1 | -0.929064% | -0.881295% | -0.686165% | -0.699597% |
| Block 2 | +0.354155% | +0.572907% | +0.428472% | +0.646647% |
| Combined 128 | -0.295499% | -0.164069% | -0.128846% | -0.026475% |
| Combined stable halves | -0.211719% | -0.036846% | -0.068284% | +0.094924% |

The small combined mean gains are driven by a few slower parent samples and
are contradicted by central tendency and win counts:

| Sample | Wall median | CPU median | Paired wall median | Paired CPU median | Wall wins | CPU wins/ties |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Block 1 | +1.077359% | +1.190476% | +0.404212% | 0.000000% | 29/64 | 27/64, 9 ties |
| Block 2 | +0.644729% | 0.000000% | +0.563777% | +1.204819% | 23/64 | 19/64, 8 ties |
| Combined 128 | +0.902998% | +1.190476% | +0.557301% | +1.119030% | 52/128 | 46/128, 17 ties |
| Combined stable halves | +1.174715% | +0.595238% | +0.861497% | +1.190476% | 25/64 | 22/64, 8 ties |

The candidate executable also grows from `8,928,256` to `8,946,688` bytes,
an increase of `18,432` bytes.

Tracked evidence is in `native-lusk-block1.csv` and
`native-lusk-block2.csv`; excluded warmups are retained in the ignored
experiment artifact directory.

## Decision

Reject. Reusing the ordered-map entries removes the intended deterministic
lookup work, but the independent native blocks do not reproduce a robust
throughput gain. The candidate loses most pairs, regresses every combined
median, grows the executable, and leaves the combined stable CPU mean
slightly worse.

Candidate production code is removed and `src/terms/termvars.rs` is restored
byte-for-byte to accepted Experiment 293. Compatibility matrices and full
repository gates are skipped after the production performance rejection.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=callgrind-single-varbank-entry.out \
  target-wsl-297-single-varbank-entry/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```

```powershell
& .\experiments\2026-07-22-214-move-termtree-insert-links\run-native.ps1 `
  -ParentExe .\target\native-293-fuse-always-deref-app-check\release\eprover.exe `
  -CandidateExe .\target\native-297-single-varbank-entry\release\eprover.exe `
  -Pairs 64 `
  -OutputCsv .\experiments\2026-07-24-024-single-varbank-entry\native-lusk-block1.csv
```
