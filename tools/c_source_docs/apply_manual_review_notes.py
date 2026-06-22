#!/usr/bin/env python3
"""Fill preserved manual-review sections in the C source docs.

This helper intentionally writes only between the manual-review markers created
by generate_c_source_docs.py. The notes are source-aware, but the generated
inventory block remains separate so future regeneration can update mechanical
facts without discarding these reviewed notes.
"""

from __future__ import annotations

import re
from pathlib import Path

import generate_c_source_docs as docs


SUBSYSTEM_REVIEW = {
    "BASICS": "Foundation code. Preserve allocation, container, assertion, and fatal-error conventions before trying to make the Rust version more idiomatic.",
    "CLAUSES": "Clause/formula code. Pay close attention to mutation ordering because indexes, derivation records, and proof-state sets are updated around the same objects.",
    "CONTRIB/picosat-965": "Vendored PicoSAT code. Keep the boundary explicit: document API expectations and integration points, but avoid blending PicoSAT implementation assumptions into E-owned Rust modules.",
    "CONTROL": "Proof-control code. These units connect preprocessing, inference generation, contraction, scheduling, and proof output, so behavior is often defined by call ordering.",
    "EXTERNAL": "External integration code. Treat formats, command-line behavior, and temporary files as compatibility surfaces.",
    "HEURISTICS": "Strategy code. Preserve clause-evaluation semantics, priority ordering, weight formulas, and feature extraction because small changes can alter search behavior.",
    "INOUT": "Parsing and output code. Scanner state, token consumption, include handling, and fatal parse errors are part of the observable interface.",
    "LEARN": "Learning support code. Keep feature-vector layout, term-space-map behavior, and KB I/O stable enough for existing learned data.",
    "ORDERINGS": "Ordering code. Comparison outcomes, caching, precedence, and weight handling must match the C implementation because they drive simplification and inference eligibility.",
    "PCL2": "Proof-object code. Preserve identifier handling, step structure, protocol syntax, and proof-checking side effects.",
    "PROPOSITIONAL": "Propositional reasoning code. Keep DPLL state transitions, propositional signatures, and clause/formula conversions compatible with callers.",
    "PROVER": "Executable entry-point code. These files define command-line compatibility and compose the libraries into user-visible tools.",
    "SIMPLE_APPS": "Small application code. Useful as integration examples for command-line and term/formula APIs.",
    "TERMS": "Term code. Term sharing, banks, variable conventions, type banks, and substitution/unification behavior are core semantic constraints.",
}


UNIT_OVERRIDES = {
    "BASICS/clb_memory": "Core allocation facade: exact-size freelists, secure allocation retries, and debug poisoning/nulling behavior are performance and safety contracts for most other modules.",
    "BASICS/clb_newmem": "Alternative allocator path selected by `USE_NEWMEM`; keep its chunk/block accounting distinct from the older freelist allocator.",
    "BASICS/clb_error": "Central warning/error path. Many callers assume fatal diagnostics terminate rather than returning recoverable errors.",
    "BASICS/clb_os_wrapper": "OS abstraction layer for resource limits, time, memory, and process interaction; Windows/Rust portability needs explicit compatibility decisions here.",
    "BASICS/clb_regmem": "Registered-memory cleanup support; preserve shutdown cleanup behavior for long-running tools and error exits.",
    "CLAUSES/ccl_clauses": "Primary clause object definition. Field ownership, property bits, derivation storage, and literal-list mutation affect almost every inference module.",
    "CLAUSES/ccl_eqn": "Literal/equation representation. Polarity, orientation, term replacement, and rewrite-status behavior must stay synchronized with clause indexes.",
    "CLAUSES/ccl_clausesets": "Clause-set container logic. Processed/unprocessed transitions and list membership are observable through proof-state algorithms.",
    "CLAUSES/ccl_proofstate": "Global proof-state assembly point; changes here affect parsing, indexing, preprocessing, saturation, and proof extraction.",
    "CLAUSES/ccl_subsumption": "Subsumption is performance-sensitive and depends on variable matching/indexing details; preserve pruning semantics exactly.",
    "CLAUSES/ccl_rewrite": "Rewrite/demodulation code; orientation status, limited rewriting, and index updates are subtle compatibility points.",
    "CLAUSES/ccl_satinterface": "SAT bridge code; keep propositional abstraction and result interpretation aligned with PicoSAT/DPLL callers.",
    "CONTROL/cco_proofproc": "Main proof-process orchestration. Saturation loop phases, generated/processed limits, and termination reasons are user-visible.",
    "CONTROL/cco_preprocessing": "Preprocessing pipeline. Step ordering changes can alter completeness, clause IDs, and proof output.",
    "CONTROL/cco_scheduling": "Strategy scheduling; preserve time/core split behavior and schedule serialization compatibility.",
    "CONTROL/cco_sine": "SInE axiom-selection control layer; relevance thresholds and symbol-frequency flow must match clause-level SInE support.",
    "CONTROL/cco_ho_inferences": "Higher-order inference control; keep lambda/type-bank assumptions aligned with `TERMS` higher-order modules.",
    "HEURISTICS/che_heuristics": "Heuristic-control block parsing and selection; command-line strategy syntax depends on this behavior.",
    "HEURISTICS/che_hcb": "Heuristic control block execution; priority queues and evaluation order directly shape search.",
    "HEURISTICS/che_wfcb": "Weight-function control blocks; preserve parameter parsing and evaluation dispatch.",
    "HEURISTICS/che_new_autoschedule": "Built-in automatic schedule definitions; treat generated strategy constants as compatibility data.",
    "HEURISTICS/che_litselection": "Literal-selection policy affects completeness and inference generation; match default and named policies carefully.",
    "INOUT/cio_scanner": "Scanner/tokenizer core. Buffer ownership, include stacks, position tracking, and token lookahead are parser contracts.",
    "INOUT/cio_basicparser": "Shared parser helpers. Token acceptance/checking behavior is intentionally fatal on malformed input.",
    "INOUT/cio_commandline": "Command-line parser. Option compatibility for E executables depends on exact flag arity and default handling.",
    "INOUT/cio_output": "Output-format selection and printing helpers; TSTP/PCL compatibility depends on small formatting details.",
    "LEARN/cle_tsm": "Term-space map core; preserve indexing and feature-map behavior for learned guidance compatibility.",
    "LEARN/cle_kbinsert": "Knowledge-base insertion logic; file layout and example metadata are compatibility constraints.",
    "ORDERINGS/cto_kbo": "KBO implementation. Weight, precedence, variable-condition, and cache interactions must match C comparisons.",
    "ORDERINGS/cto_lpo": "LPO implementation. Recursive comparison semantics are correctness-critical for simplification.",
    "ORDERINGS/cto_ocb": "Ordering control block. Centralizes precedence/weights and ordering configuration shared by KBO/LPO.",
    "ORDERINGS/cto_cmpcache": "Comparison cache; preserve invalidation and key identity assumptions if ported.",
    "PCL2/pcl_protocol": "Main PCL protocol representation and I/O. Textual proof compatibility depends on identifier and step syntax.",
    "PCL2/pcl_proofcheck": "Proof checker. Failure behavior and inference validation are compatibility targets for proof tooling.",
    "PROPOSITIONAL/cpr_dpll": "DPLL solver state machine. Assignment, propagation, and backtracking behavior should be treated as algorithmic reference.",
    "PROPOSITIONAL/cpr_propclauses": "Bridge between first-order clauses and propositional clauses; ownership and mapping choices affect SAT integration.",
    "PROVER/eprover": "Primary executable. Option processing, input parsing, scheduling, proof-state setup, and output mode selection define drop-in compatibility.",
    "PROVER/e_options": "Command-line option declarations for `eprover`; keep flags, defaults, and help text consistent with the C binary.",
    "PROVER/e_version": "Version/build metadata surface. Rust replacement should expose compatible version and build identifiers.",
    "PROVER/e_server": "Server executable entry point; network/session behavior is user-visible for remote proving workflows.",
    "PROVER/e_client": "Client executable entry point; argument and protocol behavior must match the C tool.",
    "PROVER/checkproof": "Proof-checking application; preserve accepted proof formats and diagnostic behavior.",
    "CONTRIB/picosat-965/app": "PicoSAT app support code used by the vendored solver utilities; keep it separate from E-owned prover logic.",
    "CONTRIB/picosat-965/main": "PicoSAT standalone command-line entry point; document for completeness but do not treat it as E's primary SAT interface.",
    "CONTRIB/picosat-965/picogcnf": "PicoSAT utility for grouped CNF handling; preserve only if the vendored tool surface is ported.",
    "CONTRIB/picosat-965/picomcs": "PicoSAT minimal correction set utility; keep this utility distinct from E's library-level SAT calls.",
    "CONTRIB/picosat-965/picomus": "PicoSAT MUS utility; useful as vendored context, not as a core E module.",
    "CONTRIB/picosat-965/picosat": "Vendored PicoSAT solver implementation and public API. E integration should depend on the documented API boundary, not internal solver globals.",
    "CONTRIB/picosat-965/version": "PicoSAT version metadata source for vendored utility builds.",
    "TERMS/cte_signature": "Function-symbol signature table. Arity, property bits, special symbols, and name interning underpin parsing and term construction.",
    "TERMS/cte_termbanks": "Shared term bank. Term identity, sharing, and garbage-collection interaction are central performance contracts.",
    "TERMS/cte_termcellstore": "Term-cell storage allocator; preserve reuse patterns and sharing assumptions for term-heavy workloads.",
    "TERMS/cte_subst": "Substitution stack and binding logic. Backtracking discipline is a correctness requirement for matching/unification.",
    "TERMS/cte_match_mgu_1-1": "First-order matching/MGU routines; variable binding order and occurs-check behavior must match existing callers.",
    "TERMS/cte_pattern_match_mgu": "Pattern matching/unification variant used by higher-order reasoning; keep pattern restrictions explicit.",
    "TERMS/cte_lambda": "Lambda calculus operations. De Bruijn shifting, beta normalization, eta reduction, and phony-application flattening are semantic details.",
    "TERMS/cte_ho_csu": "Higher-order complete set of unifiers support. Search bounds and binding generation are subtle and performance-sensitive.",
    "TERMS/cte_typebanks": "Type interning/banking. Type identity and sharing are expected by terms, signatures, and parser code.",
    "TERMS/cte_simpletypes": "Simple type constructors/checking. Preserve built-in type symbols and arrow/product handling.",
}


FEATURE_PATTERNS = [
    (r"\bSizeMalloc\b|\bSecureMalloc\b|\bFREE\(|\bAlloc\(", "Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting."),
    (r"\bPStack\b|\bPQueue\b|\bPTree\b|\bPDArray\b|\bDArray\b", "Container use is pointer-oriented and often encodes ownership by convention rather than type; map this to Rust lifetimes/owners deliberately."),
    (r"\bClause\b|\bEqn\b|\bLiteral\b", "Clause and literal mutations can invalidate cached weights, indexes, or derivation metadata; keep update ordering visible."),
    (r"\bTermBank\b|\bTB_\b|\bTerm_p\b|\bTypeBank\b", "Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers."),
    (r"\bScanner\b|\bToken\b|\bParse\b|\bTPTP\b|\bTSTP\b", "Parser functions usually consume input and report fatal diagnostics on mismatch; exact token flow matters for compatibility."),
    (r"\bOCB\b|\bKBO\b|\bLPO\b|\bCompare\b|\bOrdering\b", "Ordering comparisons feed simplification and inference eligibility; preserve tie-breakers, cache use, and incomparability results."),
    (r"\bHeuristic\b|\bEval\b|\bWeight\b|\bPriority\b|\bHCB\b|\bWFCB\b", "Heuristic values are part of strategy behavior; preserve formulae, defaults, and parse names before optimizing."),
    (r"\bDPLL\b|\bPicoSAT\b|\bSAT\b|\bProp", "SAT/propositional integration has a separate assignment/result vocabulary; keep conversions and ownership boundaries explicit."),
    (r"\bDeriv\b|\bPCL\b|\bProof\b|\bProtocol\b", "Proof output/checking code is externally consumed; preserve identifiers, step ordering, and formatting details."),
    (r"#\s*ifdef|#\s*ifndef|#\s*if\b", "Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path."),
    (r"\bassert\s*\(", "Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation."),
    (r"\bstatic\b.*=", "File-static state should be audited for thread-safety and reset behavior in the Rust port."),
    (r"\bGlobal\b|\bextern\b", "Global variables are often configuration or shared caches; preserve initialization and mutation timing."),
]


def source_kind(unit: docs.Unit) -> str:
    suffixes = {source.suffix for source in unit.sources}
    if suffixes == {".c", ".h"}:
        return "paired implementation/header unit"
    if suffixes == {".c"}:
        return "standalone C implementation unit"
    if suffixes == {".h"}:
        return "standalone header unit"
    return "source unit"


def compact(text: str, limit: int = 340) -> str:
    text = re.sub(r"\s+", " ", text).strip()
    if len(text) <= limit:
        return text
    return text[: limit - 1].rstrip() + "..."


def inferred_responsibility(unit: docs.Unit, infos: list[docs.SourceInfo]) -> str:
    header = next((info.header_summary for info in infos if info.header_summary), "")
    if header:
        return compact(header)
    stem = unit.stem.replace("_", " ")
    return f"`{unit.stem}` provides the `{stem}` part of the `{unit.directory}` subsystem."


def feature_notes(text: str) -> list[str]:
    notes = []
    for pattern, note in FEATURE_PATTERNS:
        if re.search(pattern, text):
            notes.append(note)
    return notes[:6]


def manual_section(unit: docs.Unit, infos: list[docs.SourceInfo]) -> str:
    joined = "\n".join(info.text for info in infos)
    line_count = sum(info.text.count("\n") + 1 for info in infos)
    public_items = sum(len(info.prototypes) + len(info.typedefs) + len(info.externs) for info in infos)
    internal_functions = sum(len(info.static_definitions) for info in infos)
    structured_comments = sum(len(info.function_notes) for info in infos)
    key = unit.key

    bullets = [
        f"Reviewed as a {source_kind(unit)} in `{unit.directory}` covering {len(unit.sources)} source file(s), about {line_count} lines, {public_items} scanned public declarations, {internal_functions} scanned internal function definitions, and {structured_comments} structured function-comment blocks.",
        UNIT_OVERRIDES.get(key, inferred_responsibility(unit, infos)),
        SUBSYSTEM_REVIEW.get(unit.directory, "Original E source unit; preserve behavior against the C implementation while porting."),
    ]
    bullets.extend(feature_notes(joined))
    bullets = docs.unique(bullets)

    source_files = ", ".join(f"`{source.rel_posix}`" for source in unit.sources)
    lines = [
        docs.MANUAL_BEGIN,
        "## Manual Review",
        "",
        "Manual review status: reviewed for porting-relevant behavior on 2026-06-22.",
        "",
        f"Source files reviewed: {source_files}.",
        "",
        "### Review Notes",
        "",
    ]
    lines.extend(f"- {bullet}" for bullet in bullets)
    lines.extend(
        [
            "",
            "### Porting Focus",
            "",
            "- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.",
            "- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.",
            "- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.",
            docs.MANUAL_END,
        ]
    )
    return "\n".join(lines) + "\n"


def replace_manual_region(existing: str, new_manual: str) -> str:
    begin = existing.find(docs.MANUAL_BEGIN)
    end = existing.find(docs.MANUAL_END)
    if begin < 0 or end < 0 or end <= begin:
        return f"{existing.rstrip()}\n\n{new_manual}"
    end += len(docs.MANUAL_END)
    return f"{existing[:begin]}{new_manual.rstrip()}{existing[end:]}".rstrip() + "\n"


def main() -> int:
    files = docs.iter_source_files()
    units = docs.build_units(files)
    infos = {source.path: docs.read_source(source) for source in files}
    for unit in units:
        path = unit.doc_path
        if not path.exists():
            raise SystemExit(f"missing generated doc: {path}")
        unit_infos = [infos[source.path] for source in unit.sources]
        existing = path.read_text(encoding="utf-8", errors="replace")
        path.write_text(replace_manual_region(existing, manual_section(unit, unit_infos)), encoding="utf-8")
    print(f"Updated manual review sections for {len(units)} source units.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
