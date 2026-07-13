# SWC078 selected-clause divergence

## Question

Why did the Rust `SWC078-1.p` proof diverge from C shortly after
presaturation even though both provers generated the same simultaneous
paramodulation children?

## Reproduction

Build and capture the Rust selected-clause trace from the repository root:

```powershell
cargo build --release --locked --bin eprover
$env:TPTP=(Resolve-Path 'eprover/EXAMPLE_PROBLEMS/TPTP').Path
.\target\release\eprover.exe --auto --output-level=1 --cpu-limit=60 `
  --memory-limit=2048 --detsort-rw --detsort-new `
  .\eprover\EXAMPLE_PROBLEMS\TPTP\SWC078-1.p
```

`compare_selected.py RUST_TRACE C_TRACE` normalizes the Rust LOP and C TSTP
selected-clause lines. The C runtime probes use the canonical cached reference:

```sh
gdb -q -batch -x trace-c-clause553.gdb \
  /home/rober/.cache/e-rust-port/bin/17026b1bfe61aaf223cfaae54947c8d2679c31a0/fol/eprover
gdb -q -batch -x trace-c-simparamod-children.gdb \
  /home/rober/.cache/e-rust-port/bin/17026b1bfe61aaf223cfaae54947c8d2679c31a0/fol/eprover
```

Raw generated traces are under `.artifacts/` with the `swc078-` prefix. The
captured C reference is `.artifacts/swc078-c-level1-tstp.txt`; the post-fix
Rust trace is `.artifacts/swc078-rust-fixed-level1-utf8.txt`.

## Findings

- Before the fix, selected clauses agreed only through ordinal 174. Rust then
  retained the `app/cons` sibling of a simultaneous-paramodulation result while
  C selected a later `cons/cons` child.
- `trace-c-simparamod-children.gdb` confirmed that C generated both siblings.
  The difference was contraction: C recognized the first sibling as a variant
  of the second, while Rust did not.
- C `eqn_list_rec_subsume` tries one side pairing for an unoriented literal,
  recursively matches all remaining literals, and then backtracks and retries
  the swapped pairing if the later recursion fails. Rust's former call to
  `Eqn::subsume` committed to the first locally successful pairing, so a later
  literal conflict could not reopen that choice.
- The focused regression uses a positive equality whose direct variable
  binding conflicts with a later negative equality. It failed before the fix
  and passes only when the positive equality is retried in swapped orientation.
  Both ordinary and mutable-term-bank clause subsumption are covered.

## Resolution

The recursive Rust clause matcher now keeps literal orientation as a
clause-level choice point. It restores the substitution after a failed
recursive suffix and retries the swapped candidate sides for an unoriented
pattern literal, matching C's control flow.

The post-fix trace agrees with C through 6,367 selected clauses instead of 174.
The next divergence is ordinal 6,368, where C selects clause `i_0_18945` and
Rust does not retain an equivalent selected clause. This later divergence is a
separate investigation; the current fix does not claim complete SWC078 proof
parity.

The full 50-case comparison at
`.artifacts/e-compare/20260712-200555-272981/` reports eight mismatches. On
`SWC078-1.p`, C proves the problem in 9.24 seconds while Rust reaches its CPU
limit and reports `ResourceOut` after 56.21 wall-clock seconds. Correcting the
early contraction therefore removes a fast but C-incompatible Rust search path
and exposes the remaining search/performance defect. The changed contractions
also re-expose a normalized proof-output difference on `LUSK6ext.lop`; the
existing reverse-insertion PDTree leaf surrogate is not sufficient under the
new clause population. The other mismatches are the established
`BOO020-1.p`, `CSR036+2.p`, `HEN011-2.p`, `sledgehammer.p`, and synthetic
one-second `LUSK6.lop` cases, plus marginal `GEO288+1.p` timing out in this run.

The five-run native benchmark at
`.artifacts/e-compare/20260712-202243-108195-benchmark/` measures a `3.373x`
aggregate Rust/C median wall-time ratio. `BOO020-1.p` is excluded because its
outcome differs. All nine behavior-matching cases exceed the required `1.10x`
threshold; `LUSK6.lop` measures `3.424x` and `LUSK6ext.lop` measures `2.997x`.
The semantic correction does not resolve the broader performance defect.

## Falsification And Limits

- Tracing generation ruled out simultaneous-paramodulation child creation as
  the first cause: both implementations generated the same two siblings.
- Inspecting rewrite links and derivation parents ruled out forward rewriting
  of the original C clause as the source of its `cons/cons` shape.
- The focused regression checks the exact recursive backtracking requirement,
  rather than merely pinning the large SWC078 trace.
- The remaining ordinal-6,368 divergence means normalized final proof output
  can still differ, and the canonical 60-second search now reaches
  `ResourceOut`. Full differential and benchmark results are recorded in
  `docs/rust-port-status.md` after validation.
