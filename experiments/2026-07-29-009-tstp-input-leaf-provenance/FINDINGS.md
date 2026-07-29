# Findings

## Result

The source-leaf and proof-publication repairs passed. The final Ubuntu run
produced five external `VerifiedGood` results, seven `VerifiedBad` source-leaf
mutations, a rejected truncated proof, and a deterministic process kill during
proof rendering that published neither a success status nor an SZS proof-block
marker.

Two definition-dependent proofs remained fail-closed `Unknown`. A minimized
synthetic proof established that this is a ProofCheck 1.0 coverage boundary:
the checker accepts the same fresh conservative definition when it is unused,
but reports `unspecified non-conservative rule` once the refutation depends on
it. Bead `E_Rust_Port-9jt.2.10` tracks definition elimination or a supporting
independent checker.

## Root causes and repairs

### Source clauses

Umlaut parsed `cnf` input directly into its normalized clause
representation. When the proof graph later archived that clause as a formula,
printing renamed variables and reordered literals but still cited the original
`file(path,name)` source. ProofCheck correctly rejected the mismatch.

The parser now retains a token-faithful TSTP body in `ClauseInfo`. Only an
archived, parentless clause source leaf uses it; derived clauses continue to
print their represented terms and explicit ancestry. This preserves source
variable names, literal signs, and literal order without changing search
terms.

### Generated definitions

Definitional CNF previously printed generated formulas as role `plain` with
the underspecified source `introduced(definition)`. Umlaut now emits role
`definition` and
`introduced(definition,[new_symbols(definition,[symbol])],[])`, following the
official TPTP
[derivation](https://tptp.org/UserDocs/QuickGuide/Derivations.html) and
[new-symbol](https://tptp.org/UserDocs/TPTPLanguage/NewSymbolNames.html)
conventions.

Internal Boolean literal encoding can represent a conservative definition as
`~P <=> ~Q` or `~P <=> Q`. The proof renderer now prints the equivalent
principal-symbol form `P <=> Q` or `P <=> ~Q`, so the new symbol occurs
positively and exactly once on the left. This is a presentation-only
normalization; search and descendants are unchanged.

ProofCheck 1.0 nevertheless cannot certify a used conservative definition.
The preserved `used-definition-proof.s` receives:

```text
% SZS status Unknown : proof relies on an unverifiable introduced(definition) step 'test_definition' -- unspecified non-conservative rule
```

Encoding the step as `inference(...,[status(esa)],...)` did not solve the
coverage gap: ProofCheck explicitly limits equisatisfiable inference support
to `skolemize` and `clausify`. The positive-only validation gate therefore
continues to return coverage-gap exit code 2 for both definition-dependent
cases.

### Interrupted output

The prover previously wrote a success status before serializing the proof
object. An outer kill could therefore leave a successful claim followed by an
unterminated `CNFRefutation`.

Success proof objects are now rendered completely into memory before the
status is published. A debug-only, opt-in fault injection delayed the first
proof node during rendering; the experiment killed that process after one
second. Its stdout was empty, while stderr contained the fault-injection
marker. No success status and no proof-block marker escaped. Separately,
removing a valid proof's end marker caused the independent validation gate to
reject it as an unterminated output block.

## Final matrix

| Case | External result | Gate exit | Prover wall (s) | Proof bytes |
|---|---:|---:|---:|---:|
| Minimized plain source | `VerifiedGood` | 0 | 0.002594 | 1,400 |
| Minimized negated-conjecture source | `VerifiedGood` | 0 | 0.002590 | 1,624 |
| Minimized used definition | `Unknown` | 2 | 0.002864 | 2,804 |
| `COL003-19` | `VerifiedGood` | 0 | 0.033920 | 3,506 |
| `SYN846-1` | `VerifiedGood` | 0 | 0.584291 | 278,128 |
| `PUZ008-2` static split | `Unknown` | 2 | 0.006116 | 14,100 |
| `GRP667-4` baseline | `VerifiedGood` | 0 | 14.771768 | 21,346 |

All three cited source leaves in the plain fixture and all four in the
negated-conjecture fixture were mutated independently. Every mutation received
`VerifiedBad`. The truncated proof received validation verdict `error` with
the reason `unterminated SZS output block CNFRefutation from line 11`.

ProofCheck self-certification ran 117 tests: 117 passed and zero failed.

## Full project validation

Ubuntu 24.04 comprehensive run `260729-125548-56eb` completed with both
`VALIDATION_COMPLETE` and `SUCCESS`:

- native Rust formatting, all-target/all-feature tests, pedantic Clippy, and
  release builds passed;
- the independent solution-validation controller ran 37 tests successfully,
  with only its opt-in external-Z3 probe skipped;
- Windows GNU x64 test and release targets cross-compiled successfully;
- the 50-case main compatibility matrix had zero unexpected mismatches and 29
  hash-pinned expected differences;
- the 216-case support-tool matrix had zero mismatches and 16 expected
  differences;
- the 10-case timing matrix had zero behavior mismatches and an aggregate
  Rust/C wall-time ratio of `1.076819`; and
- Callgrind smoke runs completed with `9,609,690` Rust and `7,591,871` C
  instructions.

The first comprehensive run exposed 13 compatibility contracts changed by the
repair. An audit of every changed normalized line found that 11 cases differed
only in file-cited source leaves. The other two combined the same source-leaf
change with their already accepted checker-complete Skolem records. Complete
reference/candidate output pairs are now SHA-256-pinned, and the 46-test
compatibility-controller suite requires every proof-output contract to retain
both digests.

## Reproduction

Run on an Ubuntu 24.04 Linode:

```text
python3 experiments/2026-07-29-009-tstp-input-leaf-provenance/run_experiment.py \
  --repo-root /opt/e-rust-port/source \
  --artifact-root /opt/e-rust-port/tstp-repair/results-v7 \
  --umlaut /opt/e-rust-port/source/target/release/umlaut \
  --debug-umlaut /opt/e-rust-port/source/target/debug/umlaut \
  --proofcheck /opt/e-rust-port/tstp-repair/proofcheck-linux-x86_64/proofcheck \
  --held-out-root /opt/e-rust-port/tstp-repair
```

The ignored local evidence archive is
`.artifacts/tstp-repair/results-v7.tar.gz`.

| Artifact | SHA-256 |
|---|---|
| Result archive | `84739e45cfad7523b4d397bd71086b5798e1cd2b066927878ec5399212577a9b` |
| `report.json` | `8417f7277e51db620ecda682e9280c7efff7fb7a131ae924c3700852ad1d31c6` |
| Controller | `18387e14a8e17d536e190c2bff1dadb50a686ad7121913bb3a7ed61fc6b8fdd6` |
| Release Umlaut | `89699f48724aacb6f851ee33f63a5709ff25f8d5000c8c82d2daf051cec4f92f` |
| Debug Umlaut | `149e1b41eb22cc59ba32b6633c45fef2df85fd5a06d7396bc282d7ce483c5585` |
| ProofCheck 1.0 | `92bb5193a9d8b2857fb97d9bd9fb6f16f5bcb57d07e4307d7f087e403ff51c7e` |
