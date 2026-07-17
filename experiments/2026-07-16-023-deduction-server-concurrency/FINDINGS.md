# Deduction-server parser, framing, and concurrency audit

## Status

Completed for Bead `E_Rust_Port-j76.1.14`. The interactive server accepts the
complete TSTP entry set represented by C `FormulaAndClauseSetParse`, preserves
the C TCP-string boundary of every output call, and serves accepted clients
concurrently with isolated parser, term, control, axiom, and runner state.

## Question

Can Rust match the C server's fork-per-client isolation and forked `RUN` wire
order without introducing process-wide parser races, and can the remaining
parser claim be closed with one server-level input-form matrix?

## C contract

Both `ADD` and `RUN` create a user-string scanner, force `TSTPFormat`, and call
`FormulaAndClauseSetParse`. The accepted entry families on this path are `cnf`,
`fof`, `tff`, `tcf`, `thf`, and `include`; watchlist clauses/formulas, type
declarations, and top-level `$distinct` are role/body variants of those forms.
Legacy `input_clause`/`input_formula` records are not accepted after the forced
TSTP format switch.

`run_command` forks, and the parent waits before returning its success status.
The child and parent therefore issue socket writes in this order:

1. `\n% Processing started for <job>\n`
2. the completed runner output, or the interactive `GaveUp` status
3. `\n% Processing finished for <job>\n\n`
4. `200 ok : success\n`

Each call to `print_to_outstream` maps directly to one `TCPStringSendX`; HELP,
LIST, and DOWNLOAD likewise send their body separately from their status, and
QUIT sends nothing. The executable's outer accept loop forks before returning
to `accept`, so multiple client sessions run simultaneously with copy-on-write
copies of all parser and interactive state.

## Rust implementation

`InteractiveCommandOutput` and `InteractiveDispatchResult` retain aggregate
text for existing text-mode/internal consumers plus byte offsets for the
original output-call frames. The TCP loop validates and sends each slice.
`InteractiveFrameWriter` records the batch processor's flush as the proof or
GaveUp boundary between the explicit RUN start and finish frames.

The executable listener now starts a detached worker for every accepted
`TcpStream`. The worker creates its term bank, structural control, interactive
axiom map, and concrete runner backend after it starts. The parser's implicit
`ProblemType` value is thread-local in production, which gives FO and HO
clients the isolation supplied by C's process fork while keeping same-thread
mixing rejection and avoiding synchronization in parser/inference lookups.

## Regression coverage

The server-level ADD matrix parses:

- ordinary and watchlist CNF records;
- FOF axioms and top-level `$distinct`;
- TFF type declarations and an ordinary typed formula;
- a TCF watchlist formula converted to the watchlist clause set;
- a selected include entry; and
- a separate typed THF problem.

The exact cardinalities, problem types, and typed-signature flags are pinned.
Existing batch-parser tests retain deeper selector, quantified-watchlist,
variable-map, legacy-rejection, and wrapper-specific coverage.

A real loopback regression starts two client workers, leaves both sessions open,
uploads the same axiom-set name with different FO and THF contents, downloads
both payloads, and quits both sessions. The second client must respond while the
first remains connected, and each download must return only that client's raw
input. A lower-level two-thread regression independently pins simultaneous FO
and HO `ProblemType` values.

RUN and printing-command regressions split aggregate output at the stored byte
offsets and assert the exact ordered frames. The socket-loop regression also
decodes actual packed TCP messages and proves LIST body/status separation.

## Reference decision

`wsl.exe --list --quiet` returns no distributions in this environment, and no
locally executable C deduction server is available. A fresh live C socket trace
could therefore not be captured. This item is closed as an evidence-backed
compatibility decision: the wire boundaries are explicit one-to-one C function
calls rather than inferred buffered writes, the complete byte strings are
literal source constants or already covered batch output, and the Rust TCP
decoder tests the resulting framed bytes. The vendored upstream source remains
unchanged. A future live reference run can validate the same permanent tests
without changing the expected protocol.

### Live-reference follow-up

WSL later became visible from the normal user context. A temporary C build and
live socket trace supersede the no-live-reference limitation above; see
[`../2026-07-17-044-deduction-server-run-framing/FINDINGS.md`](../2026-07-17-044-deduction-server-run-framing/FINDINGS.md).
The intended four-frame call order is confirmed byte-for-byte. The same run
also exposed the default C build's printf-escaped `COMCHAR` PID-search bug,
which makes the stock child abort after the start frame while its parent still
sends success. Rust intentionally preserves the usable intended path.

## Performance decision

The listener now returns to `accept` immediately after a successful thread
spawn instead of blocking for the complete session. Per-client parser and proof
work is unchanged. Thread-local problem-type access removes the former relaxed
atomic load from same-thread hot paths. The new frame-offset vectors are small,
bounded by command output events, and outside saturation hot paths; a synthetic
throughput benchmark would not add useful evidence beyond the concurrent
loopback regression.

## Validation

- focused interactive-mode tests passed
- focused deduction-server tests passed
- concurrent problem-type isolation test passed
- `cargo fmt --all -- --check`
- `cargo test --locked --lib --quiet`: 4,099 passed
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
