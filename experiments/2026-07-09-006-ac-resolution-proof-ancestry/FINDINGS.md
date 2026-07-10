# AC Resolution Proof Ancestry

Date: 2026-07-09

## Question

Why did Rust's `ALL_RULES.p` proof omit C's AC-resolution step and later show either missing parents or invalid internal rewrite-parent ids after AC cleanup was restored?

## Setup

- C reference: `.artifacts/e-compare/20260709-224129-729562/mismatches/0001/reference.normalized`.
- Input: `eprover/EXAMPLE_PROBLEMS/SMOKETEST/ALL_RULES.p`.
- Rust release command:

```powershell
target\release\eprover.exe --auto --silent --cpu-limit=60 --memory-limit=2048 --detsort-rw --detsort-new --proof-object=1 eprover\EXAMPLE_PROBLEMS\SMOKETEST\ALL_RULES.p
```

- C source reviewed: `ClauseRemoveACResolved`, `ClausePushACResDerivation`, `ClauseArchive`, `ClauseArchiveCopy`, `ClauseComputeLINormalform`, derivation extraction, and rewrite-link handling.

## Findings

- C `ClauseRemoveACResolved` removes AC-trivial negative literals, emits `inf_ac_resolution`, and pushes `DCACRes` with the current `sig->ac_axioms` count. Rust originally performed only the literal mutation.
- Restoring `DCACRes` exposed a separate identity bug. C derivations and rewrite links retain clause pointers, while Rust compacted parents to visible clause id/source pairs. Archived, preprocessed, and requeued clauses can share those fields, so proof lookup could select the wrong body.
- Clause derivation references now include an opaque generation. Archive copies and requeued clauses receive fresh generations, while ordinary literal copies preserve the object's generation.
- Rewrite-demodulator handles now carry the same generation as the originating clause. This keeps cached term rewrite links tied to the exact archived or active demodulator that created them.
- Legacy generation-zero lookup now searches every proof set for an exact reference before falling back to a visible id. This prevents an older same-id clause in `ax_archive` from hiding an exact current clause in `axioms`.
- Proof-graph aliases preserve display remapping when literal-identical dummy quotes collapse onto an earlier node, and signature AC axiom references are remapped to visible proof ids.

## Falsification Checks

- Focused tests cover plain/documenting AC cleanup, forward modification with documentation, archive/requeue generation separation, exact-before-fallback lookup, generation-preserving demodulator conversion, AC-axiom display remapping, and demodulator display ids.
- The executable regression asserts the positive-predicate rewrite through `p_holds`, the three associativity/commutativity AC parents, the `ar` step, final equality resolution, and the absence of `c_0_-...` or `c_0_922337...` parents.
- A fresh fixture run retains C's preprocessing/search classes and `Theorem` result. Its proof reaches the same semantic chain as the archived C run: `p_holds` rewrite, identity rewrite, `b=c`, associativity rewrites, AC resolution, `c=a`, final rewrite, and equality resolution.
- The full Rust suite passes 3,994 library tests, every binary target, and all three schedule integration tests. Pedantic Clippy, formatting, generated C-source documentation coverage, Change Later wording, Markdown links, and manual-section regeneration checks also pass.

## Conclusion

The original proof divergence had two independent causes: missing `ClauseRemoveACResolved` proof side effects and loss of C pointer identity in compact clause/demodulator references. Restoring both yields valid, complete AC proof ancestry without exposing internal handles.

## Limits

- The normalized Rust and C proof texts still differ in some input/derived ordering, clause roles, and literal-variable ordering even though the semantic parent chain now agrees.
- The generation is process-local compatibility metadata, not a serialized proof identifier. A future unified proof-state arena should replace it with stable typed handles.
- The full 50-case C/Rust comparison still requires an available C runtime; this focused run does not update the archived mismatch count.
