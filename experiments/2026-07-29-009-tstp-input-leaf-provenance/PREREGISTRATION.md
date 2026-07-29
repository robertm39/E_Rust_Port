# Preregistration

## Question

Can Umlaut emit faithful first-order TSTP ancestry for source clauses and
generated definitions without weakening independent proof validation or its
resource contract?

## Failure matrix

| Class | Minimized fixture | Held-out representative | Pre-repair failure |
|---|---|---|---|
| Plain CNF source | `fixtures/plain-source.p` | `COL003-19` | `VerifiedBad` source-body mismatch |
| Negated-conjecture CNF source | `fixtures/negated-source.p` | `SYN846-1` | `VerifiedBad` body-equivalence failure |
| Generated definition | `fixtures/definition.p` | `PUZ008-2` static split | `Unknown` non-conservative `introduced(definition)` |
| Interrupted proof block | deliberate truncation plus `GRP667-4` | `GRP667-4` baseline | unterminated `CNFRefutation` |

`PUZ008-2` baseline is the positive control. The pre-repair large-run evidence
is the final report from experiment `2026-07-29-008-avatar-restart-prototype`.

## Frozen implementation boundary

1. Preserve a token-faithful source body when parsing TSTP CNF input.
2. Use that body only for an archived source leaf that cites the corresponding
   file/name. Internal normalized descendants remain represented terms with
   explicit ancestry.
3. Emit generated definitions with role `definition` and a three-argument
   `introduced(definition,[new_symbols(definition,[...])],[])` record.
4. Render a complete success proof object before publishing its SZS success
   status, so a kill during expensive serialization cannot leave a successful
   claim with an open proof block.
5. Do not change inference semantics, search strategy, ProofCheck, or the
   validation gate.

The definition record follows the official TPTP derivation and new-symbol
conventions:

- <https://tptp.org/UserDocs/QuickGuide/Derivations.html>
- <https://tptp.org/UserDocs/TPTPLanguage/NewSymbolNames.html>

## Correctness gates

The change advances only if all gates pass:

1. The pinned ProofCheck bundle self-certifies all of its bundled tests.
2. Every minimized source and definition fixture receives `VerifiedGood`.
3. `COL003-19`, `SYN846-1`, `PUZ008-2` static split, and `GRP667-4` receive
   `VerifiedGood` with the repaired binary and an explicit proof-emission
   grace outside the prover CPU budget.
4. A mutation of each cited leaf body receives `VerifiedBad`.
5. A proof with its `CNFRefutation` end marker removed is rejected by Umlaut's
   independent validation gate.
6. A process killed during pre-status proof rendering emits neither an SZS
   success status nor a partial `CNFRefutation` block.
7. The full repository Rust, formatting, strict Clippy, compatibility,
   proof-output, timing, memory, and resource-limit runner passes.

Any remaining ProofCheck `Unknown` class must be named in `FINDINGS.md` and
tracked in Beads. A `VerifiedBad`, accepted mutation, incomplete output block,
or regression gate failure blocks completion.
