# classify_problem parser and merged boundaries

## Status

Completed for Bead `E_Rust_Port-j76.1.35` with an observable negative-timeout
parity fix, expanded real-input dispatcher and real-child comparison cases,
and evidence-backed ownership/process decisions. The vendored C source remained
unchanged.

## Real-input dispatcher coverage

`classify_problem.c` delegates real input to the same
`FormulaAndClauseSetParse` surface used by other executables. Rust likewise
delegates to the shared represented-owner dispatcher used by `eprover`; it does
not maintain a classifier-specific grammar.

The permanent matrix now covers the main dispatcher families and feeds each
result through real classification rather than syntax-only parsing:

- raw LOP clauses;
- legacy TPTP `input_formula(...)` and `input_clause(...)` records;
- a first-order TSTP mix of TFF type declarations, FOF, TCF, and CNF owners;
- a typed FOF term-position `$let` owner through represented CNF;
- raw THF application input and higher-order class selection; and
- an include selector that keeps one included formula, drops another, and
  combines the selected owner with a local formula.

Focused executable tests pin the LOP/legacy/mixed dispatcher routes, while the
existing lower-level classifier tests retain represented formula ownership,
typed FOOL CNF, THF dialect threading, selected includes, preprocessing, and
equation-definition boundaries.

Exact coverage of every term/formula spelling accepted by C remains shared
parser work, not a reason to fork another parser inside `classify_problem`.
Existing durable items already track the relevant shared surfaces, including
`E_Rust_Port-j76.2.105`/`.107` for full term/clause-set parser parity and the
`ccl_formulafunc` review items `E_Rust_Port-j76.4.235` through `.4.239`; the
later eground and epatternize executable items reuse the same owner. This slice
resolves the classifier-specific routing and comparison requirement while
preserving that single parser ownership.

## C merged-classification process contract

After parsing and SInE filtering, C's `ClausifyAndClassifyWTimeout`:

1. creates a pipe and default classification limits;
2. forks the already-mutated proof state;
3. sets the child soft `RLIMIT_CPU` to the requested `int` timeout;
4. preprocesses/clausifies the inherited formula owners and computes the CNF
   class;
5. writes exactly `SPEC_STRING_MEM` (22) bytes, including the terminating NUL;
6. lets the parent perform one blocking fixed-width read; and
7. substitutes 21 hyphens after any short read before collecting the child.

The parent does not run its own wall timer. CPU exhaustion or a child failure
closes the pipe and causes the short-read fallback.

## Portable Rust child contract

Native Windows has no `fork()` or POSIX `RLIMIT_CPU`, and stable standard Rust
does not expose a portable inherited-address-space process primitive. The Rust
executable therefore re-execs itself in a hidden mode:

- stdin is buffered once and the exact bytes are piped to the child;
- a file input is reopened under the same working directory/environment;
- parse format, free-number/free-object policy, and SInE configuration are
  serialized explicitly;
- the child repeats parse and SInE, performs the same fixed merged CNF
  computation, and writes the 22-byte NUL-terminated class buffer; and
- the parent applies the same short-buffer fallback, with an elapsed-time kill
  as the portable timeout guard.

For immutable files/includes and buffered stdin—the executable's normal batch
contract—the child sees the same logical state and produces the same bytes.
The permanent `merged-positive-cnf` and `merged-positive-fool` cases exercise
the actual optimized re-exec path, including represented FOOL parsing in both
parent and child. Native probes produced the expected 36-character concatenated
raw/CNF class strings.

Two differences are intentionally outside the portable contract: changing an
input/include between the parent parse and child reopen, and spending wall time
blocked without consuming CPU. Reproducing those would require a Unix-only
forked implementation and a separate native-Windows policy; neither changes
ordinary completed classifications. The zero-timeout case pins the common
21-hyphen fallback instead.

## Negative timeout sentinel fix

`-1` is the only value that disables the merged branch in `classify_problem.c`.
C passes every other `int` to `SetSoftRlimit`'s unsigned `rlim_t` parameter. On
the normal Linux reference, whose hard CPU limit is unlimited, `-2` converts to
a huge allowable limit and the child completes normally. Rust previously used
`cnf_timeout <= 0` and incorrectly returned hyphens for `-2`; it now reserves
the immediate fallback for exactly zero and treats other negative values as an
effectively unbounded elapsed timeout.

If a host imposes a finite hard CPU limit, C can reduce that huge converted
value, treat `RLimReduced` as failure, print `softrlimit call failed.`, and fall
back. That is host resource policy rather than input semantics. The permanent
`merged-negative-unbounded` case pins the normal archived-reference environment
and the focused regression pins the corrected branch boundary.

## Reference availability and performance

This sandbox has no visible WSL distribution, compiler, or archived C tools,
so the expanded cases cannot run against C in this session. They remain in the
permanent matrix for the normal user-context reference environment. Positive
merged cases run a child in both implementations; the Rust child performs a
second small parse instead of C's copy-on-write fork. The compatibility cases
complete well within two seconds, and the only runtime behavior change is that
non-sentinel negative values now perform the required classification instead
of returning immediately, so a separate throughput benchmark is not warranted.

## Validation

- `cargo test --locked --lib --quiet -- --test-threads=1`: 4,135 passed
- `cargo test --locked --bins --quiet -- --test-threads=1`: all binary targets passed
- `eprover_schedule`, `e_stratpar`, and `executable_inventory` integration
  suites: 4, 1, and 1 passed
- `tools/e-interop/test_e_interop.py`: 30 passed
- focused `prover::classify_problem::tests`: 36 passed
- `cargo check --locked --all-targets`: passed
- `cargo clippy --locked --all-targets -- -D warnings`: passed
- `cargo build --locked --release --bin classify_problem`: passed
- `cargo fmt --all -- --check`: passed
- C-source documentation coverage: 492 source files across 266 unit docs
- Change Later wording and local-link checks: 269 Markdown files each
- documentation regeneration: preserved manual sections in 268 files
- optimized native dispatcher probes: LOP, old TPTP, mixed TSTP, FOOL, and THF
  inputs classified successfully
- optimized native merged probes: FOOL positive and `-2` unbounded cases
  produced complete raw/CNF class strings
