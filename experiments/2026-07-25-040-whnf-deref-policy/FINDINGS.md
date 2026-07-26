# Higher-order WHNF dereference policy

## Status

Accepted for Beads `E_Rust_Port-j76.4.1277` and
`E_Rust_Port-j76.4.1288`. Rust now preserves C's problem-specific weak-head
dereference behavior in substitution normalization and optional term-bank
insertion without restoring C's unused signature parameter or hiding the owner
bank behind a term pointer. The vendored C checkout remains unchanged.

## Falsified batch assumption

Experiments 038 and 039 classified detailed `Change Later` records for routing,
but the two records retained by the earlier formula-owner audit exposed a
semantic gap:

- C `SubstNormTerm` selects `WHNF_deref` for higher-order problems and
  `TermDerefAlways` otherwise. Rust's only normalization path always used the
  first-order dereferencer.
- C `TBInsertOpt` selects `WHNF_deref` for higher-order `DEREF_ALWAYS`.
  Rust deliberately had not yet made `insert_opt` own that policy.

The final compatibility matrix did not prove these branches because no prior
case distinguished a beta-reduced root from its unreduced application.
Classification and broad compatibility evidence were therefore insufficient;
both records were reopened and implemented.

## Implementation

`Substitution::norm_term_with_bank` accepts the term bank and problem type
explicitly. First-order calls immediately reuse the measured borrowed-cursor
fast path; `NotInitialized` also stays on that path because C selects WHNF only
for an exact higher-order problem. Higher-order calls use a reusable owning scratch stack because
`WHNF_deref` can return rebuilt shared terms whose owners must outlive their
pending descendants. It retains C's reversed pushes and left-to-right
fresh-variable binding order.

The bank-aware policy is carried through `Eqn`, `EqnList`, `Clause`, formula
collection, equality resolution, ordered/equality factoring, and plain,
ordered, indexed, and simultaneous paramodulation normalization paths.
Fallible normalization is transactional: the substitution, equation, and
equation-list layers remove bindings created by a failed call, and inference
wrappers retain their outer backtracking boundaries around copying and
normalization.

`TermBank::insert_opt` now computes C's dereference limit before selecting the
root policy, calls `whnf_deref` only for higher-order `DerefType::Always`, and
recurses with the unchanged `CONVERT_DEREF` equivalent. Rust retains its
existing safety repair for a dereferenced unshared ground term by inserting a
shared copy; unchanged C asserts that this expansion is already shared.

## Regression boundaries

The substitution regression applies a bound function variable to a discarded
free-variable argument. With first-order dereferencing, the unreduced argument
would be visited and freshened; with C's higher-order WHNF policy, beta
reduction returns the rigid lambda matrix and the substitution stays empty.
The same regression first runs with `NotInitialized` and requires the ordinary
dereference result, pinning the exact branch condition.

The optional-insertion regression applies a bound function variable to a DB
argument under higher-order `DEREF_ALWAYS`. The result must be the existing
shared rigid constant, not a retained phony application. The pre-existing
first-order optional-insertion regression continues to require the unreduced
application, proving that the new branch is problem-specific.

[`audit_whnf_deref_policy.py`](audit_whnf_deref_policy.py) pins 13 C-source,
Rust-source, production-routing, regression, safety, and error-cleanup checks
over nine implementation files. [`audit-reference.json`](audit-reference.json)
pins source digest
`e575ccfe38ea03da7283b4bfb9b35b29151604b3567c6c5dd49249989c8aa609`.

## Validation

On ephemeral Ubuntu 24.04 worker `e-rust-codex-260726-191044-7524` with Rust
1.97.1, exact code snapshot
`2c0fee77927f5188f026b95e27984351df19725f9296e1b2fbc007ae699aca38`
and release `eprover` SHA-256
`8706e23a757691b9374001605527fff28c758a2a00fbff61a81d5eb4f851f3b3`
pass:

- `cargo fmt --all -- --check`;
- strict all-target/all-feature pedantic Clippy;
- the two focused higher-order beta/WHNF regressions;
- the complete all-target/all-feature suite: 4,416 library and 11 integration
  tests, 4,427 total; and
- the native optimized build of every Rust binary;
- compile-only Windows GNU x64 all-target/all-feature tests and every release
  binary; and
- PE32+ inspection of `eprover.exe`, SHA-256
  `03d3e4ed29c2646fe2f026542c7f1b1af660eb275ac69866c1422c388d23f7c0`.

The exact optimized binaries also pass both maintained native compatibility
matrices:

- all 50 main-prover cases have zero unexpected differences and the one
  declared `sledgehammer.p` presentation difference; and
- all 216 support-tool cases have zero unexpected differences and 15 declared
  differences.

[`validation-reference.json`](validation-reference.json) retains the compact
snapshot, binary, test, cross-compile, and matrix record.

The documentation coverage, `Change Later` wording, local-link, and
manual-regeneration validators pass across the indexed C-source Markdown
corpus. The C checkout is clean.

Reproduce the source audit locally:

```powershell
.\.venv\Scripts\python.exe experiments/2026-07-25-040-whnf-deref-policy/audit_whnf_deref_policy.py `
  --repo . `
  --expected experiments/2026-07-25-040-whnf-deref-policy/audit-reference.json
```
