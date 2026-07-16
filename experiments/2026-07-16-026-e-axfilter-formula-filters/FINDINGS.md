# Formula-aware `e_axfilter` GSinE/LambdaDef audit

## Status

Completed for Bead `E_Rust_Port-j76.1.17` as a source-backed compatibility
decision with permanent executable and comparison coverage. This host has no
installed WSL distribution, so the newly registered full-file cases could not
be run against a fresh C executable in this session.

## Question

Does the current owner model still require a temporary formula bridge before
`e_axfilter` can apply GSinE and LambdaDef to formulas and render the selected
formula identities exactly once in C stack order?

## C source evidence

`StructFOFSpecGetProblem` allocates non-owning clause and formula result stacks.
For `AFGSinE`, it calls `SelectAxioms` with the owned clause/formula-set stacks,
the shared-axiom boundary, and both result stacks. `SelectAxioms` builds the
mixed D-relation, discovers clause and formula seeds in set order, traverses
typed pointer entries, and appends the selected object pointers to the result
stacks. For `AFLambdaDefines`, `SelectDefinitions` scans formula sets in order,
ignores clause sets, and retains lambda definitions, conjectures, and
hypotheses.

`e_axfilter.c::filter_problem` consumes those borrowed stacks before the source
`StructFOFSpec` is freed. It writes type declarations, then the selected clause
stack, then the selected formula stack. `PStackFormulaPrintTSTP` prints formula
pointers in increasing stack-index order and appends one newline per formula.
No selected formula is cloned, moved to a new owner, or lowered to a clause.

## Rust ownership audit

The migrated pending note predated the current `StructFofSpec` implementation.
That owner now retains real `FormulaSet`s alongside its `ClauseSet`s for the
entire filtering call. `get_problem` constructs borrowed `PStack<&Clause>` and
`PStack<&WrappedFormula>` results:

- mixed GSinE D-relations store and return borrowed identities from those
  source owners;
- LambdaDef reads the `CP_IS_LAMBDA_DEF` property assigned to higher-order
  `definition` records and uses the shared conjecture/hypothesis predicates;
- selected stacks are rendered immediately while the source sets remain alive;
  and
- formula rendering preserves the parsed problem dialect and complete formula
  wrapper instead of using the proof-state clause-lowering path.

The stable-handle concern still documented for destructive `ProofStateSinE`
movement is therefore not a blocker for this non-moving executable path.

## New coverage

Two executable regressions exercise the complete parse/select/render path:

- a FOF GSinE case activates a related formula through the goal's symbol and
  excludes an unrelated formula owner; and
- a THF LambdaDef case prints the lambda definition and conjecture while
  excluding an ordinary axiom, retaining THF formula output.

The interop harness now also registers `tstp-gsine-formulas` and
`tstp-lambda-def-formulas`. Each compares both configured global output and the
complete generated problem file when the C reference environment is available.

## Performance decision

Production selection code was already complete and required no change. The new
work adds only tests and comparison metadata. Borrowed selected stacks retain
C's allocation profile more closely than a cloning bridge would, so no
performance benchmark is warranted.

## Validation

- focused formula GSinE executable regression passed
- focused LambdaDef executable regression passed
- focused `prover::e_axfilter::tests`: 22 passed
- interop harness tests passed: 26 passed
- Python bytecode compilation for the interop harness and tests
- `cargo fmt --all -- --check`
- `cargo test --locked --lib --quiet`: 4,108 passed
- `cargo test --locked --bins --quiet`: all binary targets passed
- `cargo test --locked --test eprover_schedule --quiet`: 3 passed
- `cargo test --locked --test executable_inventory --quiet`: 1 passed
- `cargo check --locked --all-targets`
- `cargo clippy --locked --all-targets -- -D warnings`
- C-source coverage: 492 source files covered by 266 unit docs
- Change Later wording: 269 Markdown files checked
- local Markdown links: 269 Markdown files checked
- regeneration preservation: manual sections preserved in 268 Markdown files
- `git diff --check`
