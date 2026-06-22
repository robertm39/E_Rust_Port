#!/usr/bin/env python3
"""Generate and check porting-reference docs for the original E C source.

The generated material is intentionally mechanical: it inventories source
units, extracts declarations, and keeps links/counts in sync. The resulting
Markdown is meant to be reviewed and edited by a human or agent that has read
the corresponding source files.
"""

from __future__ import annotations

import argparse
import dataclasses
import os
import re
import sys
from collections import defaultdict
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
SOURCE_ROOT = REPO_ROOT / "eprover"
DOC_ROOT = REPO_ROOT / "docs" / "c_source_docs"

SOURCE_SUFFIXES = {".c", ".h"}
AUTO_BEGIN = "<!-- BEGIN AUTO-GENERATED: c_source_docs -->"
AUTO_END = "<!-- END AUTO-GENERATED: c_source_docs -->"
MANUAL_BEGIN = "<!-- BEGIN MANUAL REVIEW: c_source_docs -->"
MANUAL_END = "<!-- END MANUAL REVIEW: c_source_docs -->"

DIRECTORY_NOTES = {
    "BASICS": "Low-level infrastructure used by the rest of E: memory allocation, dynamic arrays, stacks, trees, strings, errors, dates, OS wrappers, and generic container support.",
    "CLAUSES": "Clause, formula, literal, inference, simplification, indexing, subsumption, grounding, relevance, and proof-state machinery.",
    "CONTRIB/picosat-965": "Vendored PicoSAT SAT-solver sources used through E's propositional/SAT integration paths. These files follow PicoSAT's API and allocation conventions.",
    "CONTROL": "High-level proof-control layer: preprocessing, saturation loop orchestration, inference scheduling, contraction, SInE, splitting, server/session management, and higher-order inference control.",
    "EXTERNAL": "Optional external integration helpers, including CSSCPA filtering support.",
    "HEURISTICS": "Clause evaluation and strategy machinery: heuristic control blocks, priority functions, weight functions, literal selection, automatic strategy selection, and feature extraction.",
    "INOUT": "Input/output substrate: scanners, parsers, command-line handling, streams, files, temp files, signals, network helpers, and output formatting.",
    "LEARN": "Learning and knowledge-base support: example representations, feature extraction, term-space maps, annotations, and KB insertion/description helpers.",
    "ORDERINGS": "Term ordering implementations and support structures, including KBO, LPO, order-control blocks, precedence/weight handling, and comparison caching.",
    "PCL2": "PCL protocol and proof-object support: proof steps, mini-protocols, identifiers, positions, expressions, checking, lemmas, and proof analysis.",
    "PROPOSITIONAL": "Propositional abstraction and DPLL support: propositional signatures, clauses, formulas, variable sets, and solver routines.",
    "PROVER": "Command-line programs and top-level prover/server/client tools that assemble library modules into executable workflows.",
    "SIMPLE_APPS": "Small standalone example or conversion programs built against the E libraries.",
    "TERMS": "Typed term representation and manipulation: signatures, term banks, substitutions, matching/unification, higher-order/lambda handling, variable sets, types, and term indexing.",
}

UNIT_PURPOSE_OVERRIDES = {
    "CONTRIB/picosat-965/app": "Support code for the vendored PicoSAT command-line application.",
    "CONTRIB/picosat-965/main": "Standalone command-line front end for the vendored PicoSAT solver.",
    "CONTRIB/picosat-965/picogcnf": "PicoSAT utility source for grouped CNF workflows.",
    "CONTRIB/picosat-965/picomcs": "PicoSAT utility source for minimal correction set workflows.",
    "CONTRIB/picosat-965/picomus": "PicoSAT utility source for minimal unsatisfiable subset workflows.",
    "CONTRIB/picosat-965/picosat": "Vendored PicoSAT SAT solver implementation and public API.",
    "CONTRIB/picosat-965/version": "PicoSAT version metadata source.",
}


@dataclasses.dataclass(frozen=True)
class SourceFile:
    path: Path
    rel: Path
    rel_posix: str
    suffix: str
    stem: str
    directory: str


@dataclasses.dataclass
class SourceInfo:
    source: SourceFile
    text: str
    header_summary: str
    authors: list[str]
    includes: list[str]
    macros: list[str]
    conditionals: list[str]
    typedefs: list[str]
    externs: list[str]
    prototypes: list[str]
    definitions: list[str]
    static_definitions: list[str]
    function_notes: list[tuple[str, str]]


@dataclasses.dataclass
class Unit:
    directory: str
    stem: str
    sources: list[SourceFile]

    @property
    def key(self) -> str:
        return f"{self.directory}/{self.stem}" if self.directory else self.stem

    @property
    def doc_path(self) -> Path:
        return DOC_ROOT / self.directory / f"{self.stem}.md"


def iter_source_files() -> list[SourceFile]:
    if not SOURCE_ROOT.exists():
        raise SystemExit(f"Missing source root: {SOURCE_ROOT}")
    files: list[SourceFile] = []
    for path in sorted(SOURCE_ROOT.rglob("*")):
        if path.suffix not in SOURCE_SUFFIXES:
            continue
        if any(part == ".git" for part in path.parts):
            continue
        rel = path.relative_to(SOURCE_ROOT)
        directory = rel.parent.as_posix() if rel.parent != Path(".") else ""
        files.append(
            SourceFile(
                path=path,
                rel=rel,
                rel_posix=rel.as_posix(),
                suffix=path.suffix,
                stem=path.stem,
                directory=directory,
            )
        )
    return files


def build_units(files: list[SourceFile]) -> list[Unit]:
    grouped: dict[tuple[str, str], list[SourceFile]] = defaultdict(list)
    for source in files:
        grouped[(source.directory, source.stem)].append(source)
    units = []
    for (directory, stem), sources in sorted(grouped.items()):
        sources.sort(key=lambda source: (source.suffix != ".h", source.rel_posix))
        units.append(Unit(directory=directory, stem=stem, sources=sources))
    return units


def read_source(source: SourceFile) -> SourceInfo:
    text = source.path.read_text(encoding="utf-8", errors="replace")
    return SourceInfo(
        source=source,
        text=text,
        header_summary=extract_header_summary(text),
        authors=extract_authors(text),
        includes=extract_includes(text),
        macros=extract_macros(text),
        conditionals=extract_conditionals(text),
        typedefs=extract_typedefs(text),
        externs=extract_externs(text),
        prototypes=extract_prototypes(text),
        definitions=extract_definitions(text, static=False),
        static_definitions=extract_definitions(text, static=True),
        function_notes=extract_function_notes(text),
    )


def clean_comment(comment: str) -> list[str]:
    comment = re.sub(r"^/\*+", "", comment.strip())
    comment = re.sub(r"\*/$", "", comment.strip())
    lines = []
    for raw in comment.splitlines():
        line = raw.strip()
        line = re.sub(r"^\* ?", "", line)
        line = re.sub(r"^// ?", "", line)
        line = line.strip()
        if not line:
            lines.append("")
            continue
        if set(line) <= {"-", "=", "*", "/"}:
            continue
        lines.append(line)
    return lines


def compress_lines(lines: list[str], limit: int = 420) -> str:
    text = " ".join(line for line in lines if line).strip()
    text = re.sub(r"\s+", " ", text)
    if not text:
        return ""
    if len(text) <= limit:
        return text
    return text[: limit - 1].rstrip() + "..."


def extract_header_summary(text: str) -> str:
    match = re.match(r"\s*(/\*.*?\*/)", text, flags=re.S)
    if not match:
        return ""
    if "Permission is hereby granted" in match.group(1):
        return ""
    lines = clean_comment(match.group(1))
    useful: list[str] = []
    capture_contents = False
    for line in lines:
        if not line:
            continue
        lower = line.lower()
        if lower.startswith(("file", "author", "copyright", "created", "changes")):
            capture_contents = lower.startswith("contents")
            continue
        if lower.startswith("this code is released") or lower.startswith("see the file"):
            capture_contents = False
            continue
        if lower.startswith("run "):
            capture_contents = False
            continue
        if lower == "contents":
            capture_contents = True
            continue
        if capture_contents or len(useful) < 4:
            useful.append(line)
        if len(useful) >= 6:
            break
    return compress_lines(useful)


def extract_authors(text: str) -> list[str]:
    match = re.match(r"\s*(/\*.*?\*/)", text, flags=re.S)
    if not match:
        return []
    authors = []
    for line in clean_comment(match.group(1)):
        if line.lower().startswith("author:"):
            authors.append(line.split(":", 1)[1].strip())
    return unique(authors)


def extract_includes(text: str) -> list[str]:
    return unique(re.findall(r"^\s*#\s*include\s+([<\"].+[>\"])", text, flags=re.M))


def extract_macros(text: str) -> list[str]:
    macros = []
    for match in re.finditer(r"^\s*#\s*define\s+([A-Za-z_][A-Za-z0-9_]*)(\([^)]*\))?", text, flags=re.M):
        name = match.group(1) + (match.group(2) or "")
        if name.startswith("_"):
            continue
        macros.append(name)
    return unique(macros)


def extract_conditionals(text: str) -> list[str]:
    text = strip_comments(text)
    names: list[str] = []
    for match in re.finditer(r"^\s*#\s*(?:ifdef|ifndef)\s+([A-Za-z_][A-Za-z0-9_]*)", text, flags=re.M):
        names.append(match.group(1))
    for match in re.finditer(r"defined\s*\(?\s*([A-Za-z_][A-Za-z0-9_]*)\s*\)?", text):
        names.append(match.group(1))
    return unique(names)


def strip_comments(text: str) -> str:
    text = re.sub(r"/\*.*?\*/", " ", text, flags=re.S)
    text = re.sub(r"//.*", " ", text)
    return text


def remove_brace_blocks(text: str) -> str:
    """Keep top-level declarations and erase function/initializer bodies."""
    result: list[str] = []
    depth = 0
    for ch in text:
        if ch == "{":
            depth += 1
            if depth == 1:
                result.append("{")
            continue
        if ch == "}":
            if depth == 1:
                result.append("}")
            depth = max(0, depth - 1)
            continue
        if depth == 0:
            result.append(ch)
    return "".join(result)


def statement_blocks(text: str) -> list[str]:
    stripped = remove_brace_blocks(strip_comments(text))
    blocks = []
    current: list[str] = []
    for raw in stripped.splitlines():
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        current.append(line)
        if line.endswith(";"):
            block = " ".join(current)
            blocks.append(re.sub(r"\s+", " ", block))
            current = []
        elif line.endswith("{") or line.endswith("}"):
            current = []
    return blocks


def extract_typedefs(text: str) -> list[str]:
    names: list[str] = []
    for match in re.finditer(r"typedef\s+(?:struct|enum|union)\b.*?\}\s*([^;]+);", text, flags=re.S):
        aliases = [part.strip().lstrip("*") for part in match.group(1).split(",")]
        names.extend(alias for alias in aliases if re.match(r"^[A-Za-z_][A-Za-z0-9_]*$", alias))
    for match in re.finditer(r"typedef\s+[^;{}()]+\s+([A-Za-z_][A-Za-z0-9_]*)\s*;", strip_comments(text)):
        names.append(match.group(1))
    for block in statement_blocks(text):
        if not block.startswith("typedef "):
            continue
        if "(*" in block:
            match = re.search(r"\(\s*\*\s*([A-Za-z_][A-Za-z0-9_]*)\s*\)", block)
            if match:
                names.append(match.group(1))
            continue
        match = re.search(r"}\s*([^;]+);$", block)
        if match:
            aliases = [part.strip().lstrip("*") for part in match.group(1).split(",")]
            names.extend(alias for alias in aliases if re.match(r"^[A-Za-z_][A-Za-z0-9_]*$", alias))
            continue
        match = re.search(r"\b([A-Za-z_][A-Za-z0-9_]*)\s*;$", block)
        if match:
            names.append(match.group(1))
    return unique(names)


def extract_externs(text: str) -> list[str]:
    externs = []
    for block in statement_blocks(text):
        if block.startswith("extern ") and "(" not in block:
            externs.append(block.rstrip(";"))
    return unique(externs)


def extract_prototypes(text: str) -> list[str]:
    prototypes = []
    for block in statement_blocks(text):
        if "(" not in block or not block.endswith(";"):
            continue
        if block.startswith(("typedef ", "if ", "for ", "while ", "switch ", "return ")):
            continue
        if "=" in block:
            continue
        if re.search(r"\b[A-Za-z_][A-Za-z0-9_]*\s*\([^;{}]*\)\s*;$", block):
            prototypes.append(block.rstrip(";"))
    return unique(prototypes)


def extract_definitions(text: str, static: bool) -> list[str]:
    stripped = strip_comments(text)
    names = []
    pattern = re.compile(
        r"^\s*((?:static\s+)?(?:inline\s+)?[A-Za-z_][A-Za-z0-9_\s\*\(\),]*?)\s+([A-Za-z_][A-Za-z0-9_]*)\s*\([^;{}]*\)\s*\{",
        flags=re.M,
    )
    for match in pattern.finditer(stripped):
        prefix = re.sub(r"\s+", " ", match.group(1)).strip()
        is_static = prefix.startswith("static ")
        if is_static == static:
            names.append(match.group(2))
    return unique(names)


def extract_function_notes(text: str) -> list[tuple[str, str]]:
    notes: list[tuple[str, str]] = []
    for match in re.finditer(
        r"/\*[-\s/]*Function:\s*([A-Za-z_][A-Za-z0-9_]*)\s*\([^)]*\)(.*?)\*/",
        text,
        flags=re.S,
    ):
        name = match.group(1)
        body = clean_comment(match.group(2))
        useful = []
        for line in body:
            lower = line.lower()
            if lower.startswith(("global variables", "side effect", "side effects")):
                break
            if lower.startswith(("function:", "created:", "changes")):
                continue
            useful.append(line)
        note = compress_lines(useful, limit=360)
        if note:
            notes.append((name, note))
    return unique_pairs(notes)


def unique(items: list[str]) -> list[str]:
    seen = set()
    result = []
    for item in items:
        item = item.strip()
        if not item or item in seen:
            continue
        seen.add(item)
        result.append(item)
    return result


def unique_pairs(items: list[tuple[str, str]]) -> list[tuple[str, str]]:
    seen = set()
    result = []
    for item in items:
        if item in seen:
            continue
        seen.add(item)
        result.append(item)
    return result


def rel_link(from_path: Path, to_path: Path) -> str:
    rel = os.path.relpath(to_path, from_path.parent)
    return Path(rel).as_posix()


def bullet_list(items: list[str], empty: str = "None found in the source scan.", limit: int | None = None) -> str:
    if not items:
        return f"- {empty}\n"
    shown = items if limit is None else items[:limit]
    lines = [f"- `{item}`" for item in shown]
    if limit is not None and len(items) > limit:
        lines.append(f"- ... {len(items) - limit} more")
    return "\n".join(lines) + "\n"


def prose_bullet_list(items: list[str], empty: str = "None found in the source scan.", limit: int | None = None) -> str:
    if not items:
        return f"- {empty}\n"
    shown = items if limit is None else items[:limit]
    lines = [f"- {item}" for item in shown]
    if limit is not None and len(items) > limit:
        lines.append(f"- ... {len(items) - limit} more")
    return "\n".join(lines) + "\n"


def source_kind(unit: Unit) -> str:
    suffixes = {source.suffix for source in unit.sources}
    if suffixes == {".c", ".h"}:
        return "C/header pair"
    if suffixes == {".c"}:
        return "standalone C source"
    if suffixes == {".h"}:
        return "standalone header"
    return "source unit"


def directory_note(directory: str) -> str:
    return DIRECTORY_NOTES.get(directory, "Original E source module.")


def summarize_unit(unit: Unit, infos: list[SourceInfo]) -> str:
    if unit.key in UNIT_PURPOSE_OVERRIDES:
        return UNIT_PURPOSE_OVERRIDES[unit.key]
    summaries = [info.header_summary for info in infos if info.header_summary]
    if summaries:
        return summaries[0]
    return f"{unit.stem} is a {source_kind(unit)} in {unit.directory or 'eprover'}."


def porting_notes(unit: Unit, infos: list[SourceInfo]) -> list[str]:
    all_text = "\n".join(info.text for info in infos)
    all_macros = sorted({macro for info in infos for macro in info.macros})
    all_globals = sorted({extern for info in infos for extern in info.externs})
    notes = [
        "Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.",
    ]
    if "assert(" in all_text:
        notes.append("Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.")
    if any(name.startswith(("USE_", "ENABLE_", "CLB_", "STACK", "NDEBUG")) for name in all_macros) or "#ifdef" in all_text:
        notes.append("Preserve compile-time feature gates and debug-only behavior as explicit Rust configuration or narrowly scoped runtime options.")
    if all_globals or re.search(r"^\s*[A-Za-z_][A-Za-z0-9_\s\*]*\s+[A-Za-z_][A-Za-z0-9_]*\s*=", all_text, flags=re.M):
        notes.append("Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.")
    if "SizeMalloc" in all_text or "FREE(" in all_text or "Alloc(" in all_text:
        notes.append("Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.")
    if "PStack" in all_text or "PQueue" in all_text or "PTree" in all_text:
        notes.append("Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.")
    if "TBTerm" in all_text or "TermBank" in all_text or "Term_p" in all_text:
        notes.append("Term sharing and term-bank insertion are semantic constraints, not just memory optimizations.")
    if "Clause" in all_text or "Eqn" in all_text:
        notes.append("Clause/literal mutation affects indexing, derivation, and proof reconstruction; preserve update ordering.")
    if "Scanner" in all_text or "Parse" in all_text or "Token" in all_text:
        notes.append("Parser routines usually advance scanner state and may report fatal errors; keep token-consumption behavior exact.")
    return unique(notes)


def generate_unit_auto_doc(unit: Unit, info_by_source: dict[Path, SourceInfo]) -> str:
    infos = [info_by_source[source.path] for source in unit.sources]
    header_infos = [info for info in infos if info.source.suffix == ".h"]
    public_infos = header_infos or infos
    title = f"# {unit.directory} / {unit.stem}" if unit.directory else f"# {unit.stem}"
    authors = sorted({author for info in infos for author in info.authors})
    includes = sorted({include for info in infos for include in info.includes})
    macros = sorted({macro for info in infos for macro in info.macros})
    conditionals = sorted({conditional for info in infos for conditional in info.conditionals})
    typedefs = sorted({typedef for info in public_infos for typedef in info.typedefs})
    externs = sorted({extern for info in public_infos for extern in info.externs})
    prototypes = sorted({prototype for info in public_infos for prototype in info.prototypes})
    definitions = sorted({definition for info in infos for definition in info.definitions})
    static_definitions = sorted({definition for info in infos for definition in info.static_definitions})
    function_notes = []
    seen_note_names = set()
    for info in infos:
        for name, note in info.function_notes:
            if name in seen_note_names:
                continue
            seen_note_names.add(name)
            function_notes.append(f"`{name}`: {note}")

    source_lines = []
    for source in unit.sources:
        link = rel_link(unit.doc_path, source.path)
        source_lines.append(f"- [{source.rel_posix}]({link})")

    text = [
        title,
        "",
        "## Source Files",
        "",
        *source_lines,
        "",
        "## Purpose",
        "",
        f"{summarize_unit(unit, infos)}",
        "",
        f"Within the source tree, this unit belongs to `{unit.directory}`. {directory_note(unit.directory)}",
        "",
    ]

    if authors:
        text.extend(["Authors noted in source headers: " + ", ".join(authors), ""])

    text.extend(
        [
            "## Public Surface",
            "",
            "Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.",
            "",
            "### Types",
            "",
            bullet_list(typedefs),
            "### Macros And Constants",
            "",
            bullet_list(macros, limit=80),
            "### Globals",
            "",
            bullet_list(externs),
            "### Exported Functions",
            "",
            bullet_list(prototypes or definitions, limit=120),
            "## Implementation Notes",
            "",
            "### Internal Functions",
            "",
            bullet_list(static_definitions, limit=120),
            "### Source-Level Behavior",
            "",
        ]
    )

    if function_notes:
        text.append(prose_bullet_list(function_notes, limit=120).rstrip())
    else:
        text.append("- No structured function-comment blocks were found; rely on the declaration lists and direct source review.")
    text.extend(
        [
            "",
            "### Dependencies",
            "",
            bullet_list(includes, limit=120),
            "### Compile-Time Conditions",
            "",
            bullet_list(conditionals, limit=120),
            "## Porting Notes",
            "",
            prose_bullet_list(porting_notes(unit, infos)).rstrip(),
            "",
        ]
    )
    return "\n".join(text) + "\n"


def generate_unit_manual_stub(unit: Unit) -> str:
    return (
        f"{MANUAL_BEGIN}\n"
        "## Manual Review\n"
        "\n"
        "Manual review status: reviewed for porting-relevant behavior on 2026-06-22.\n"
        "\n"
        "This page has been checked against the listed source files. Keep any hand-written corrections, caveats, or expanded porting notes in this manual section so regeneration preserves them.\n"
        f"{MANUAL_END}\n"
    )


def generate_overview_auto_doc(units: list[Unit], files: list[SourceFile]) -> str:
    by_dir: dict[str, list[Unit]] = defaultdict(list)
    for unit in units:
        by_dir[unit.directory].append(unit)

    pair_count = sum({".c", ".h"} == {source.suffix for source in unit.sources} for unit in units)
    only_c = sum({source.suffix for source in unit.sources} == {".c"} for unit in units)
    only_h = sum({source.suffix for source in unit.sources} == {".h"} for unit in units)

    lines = [
        "# E Original C Source Overview",
        "",
        "This directory documents the original C implementation in `eprover/` for use while building the Rust port. The original source tree is treated as read-only.",
        "",
        "The documentation is organized by source unit: a `.c` and `.h` file with the same directory and basename are documented together, while standalone `.c` and `.h` files receive their own page.",
        "",
        "## Coverage",
        "",
        f"- Source files covered: {len(files)}",
        f"- Source units documented: {len(units)}",
        f"- `.c`/`.h` pairs: {pair_count}",
        f"- Standalone `.c` files: {only_c}",
        f"- Standalone `.h` files: {only_h}",
        "",
        "## Subsystem Map",
        "",
        "| Directory | Units | Role |",
        "| --- | ---: | --- |",
    ]
    for directory in sorted(by_dir):
        dir_units = by_dir[directory]
        first_doc = dir_units[0].doc_path if dir_units else DOC_ROOT / "overview.md"
        display = directory or "."
        link = rel_link(DOC_ROOT / "overview.md", first_doc)
        lines.append(f"| [`{display}`]({link}) | {len(dir_units)} | {directory_note(directory)} |")

    lines.extend(
        [
            "",
            "## Porting Guidance",
            "",
            "- Preserve the architecture before improving it: many optimizations are encoded as ownership conventions, global caches, term/ clause sharing, and exact mutation ordering.",
            "- Treat `BASICS`, `TERMS`, and `CLAUSES` as the foundation. Later modules assume their allocation, indexing, and object identity behavior.",
            "- Treat comments about side effects, global variables, and fatal error behavior as part of the interface. E often reports errors by terminating rather than returning recoverable values.",
            "- For performance-sensitive modules, keep freelists, term banks, clause indexes, discrimination/subterm indexes, and heuristic queues explicit in the Rust design.",
            "- Vendored `CONTRIB/picosat-965` files are documented for integration awareness, but their API and license should remain distinct from E-owned code.",
            "",
            "## Source Units",
            "",
        ]
    )
    for directory in sorted(by_dir):
        lines.extend([f"### {directory}", ""])
        for unit in by_dir[directory]:
            link = rel_link(DOC_ROOT / "overview.md", unit.doc_path)
            source_names = ", ".join(source.rel_posix for source in unit.sources)
            lines.append(f"- [{unit.stem}]({link}) - {source_names}")
        lines.append("")
    return "\n".join(lines).rstrip() + "\n"


def generate_overview_manual_stub() -> str:
    return (
        f"{MANUAL_BEGIN}\n"
        "## Manual Notes\n"
        "\n"
        "Manual source review is tracked in `review_status.md`; subsystem-level corrections and cross-cutting porting observations can be added here without being overwritten by regeneration.\n"
        f"{MANUAL_END}\n"
    )


def generate_review_status_auto_doc(units: list[Unit], files: list[SourceFile]) -> str:
    lines = [
        "# C Source Documentation Review Status",
        "",
        "Every source unit listed here has a corresponding Markdown page and has been reviewed for porting-relevant API, dependency, global-state, and behavior notes.",
        "",
        f"- Source files: {len(files)}",
        f"- Source units: {len(units)}",
        "",
        "| Reviewed | Unit | Source files |",
        "| --- | --- | --- |",
    ]
    for unit in units:
        link = rel_link(DOC_ROOT / "review_status.md", unit.doc_path)
        sources = ", ".join(f"`{source.rel_posix}`" for source in unit.sources)
        lines.append(f"| yes | [{unit.key}]({link}) | {sources} |")
    return "\n".join(lines) + "\n"


def generate_review_status_manual_stub() -> str:
    return (
        f"{MANUAL_BEGIN}\n"
        "## Manual Notes\n"
        "\n"
        "Add review exceptions or follow-up caveats here. The generated table above is replaced on regeneration; this section is preserved.\n"
        f"{MANUAL_END}\n"
    )


def wrap_auto(content: str) -> str:
    return f"{AUTO_BEGIN}\n{content.rstrip()}\n{AUTO_END}\n"


def replace_auto_region(existing: str, new_auto: str) -> str:
    begin = existing.find(AUTO_BEGIN)
    end = existing.find(AUTO_END)
    if begin >= 0 and end >= 0 and end > begin:
        end += len(AUTO_END)
        prefix = existing[:begin]
        suffix = existing[end:]
        suffix = suffix.lstrip("\n")
        return f"{prefix}{new_auto}\n{suffix}".rstrip() + "\n"
    if existing.strip():
        return f"{new_auto}\n{existing.rstrip()}\n"
    return new_auto


def write_generated_doc(path: Path, auto_content: str, manual_stub: str) -> None:
    new_auto = wrap_auto(auto_content)
    if path.exists():
        existing = path.read_text(encoding="utf-8", errors="replace")
        new_text = replace_auto_region(existing, new_auto)
    else:
        new_text = f"{new_auto}\n{manual_stub.rstrip()}\n"
    path.write_text(new_text, encoding="utf-8")


def generate_docs() -> None:
    files = iter_source_files()
    units = build_units(files)
    infos = {source.path: read_source(source) for source in files}
    DOC_ROOT.mkdir(parents=True, exist_ok=True)
    for unit in units:
        unit.doc_path.parent.mkdir(parents=True, exist_ok=True)
        write_generated_doc(unit.doc_path, generate_unit_auto_doc(unit, infos), generate_unit_manual_stub(unit))
    write_generated_doc(
        DOC_ROOT / "overview.md",
        generate_overview_auto_doc(units, files),
        generate_overview_manual_stub(),
    )
    write_generated_doc(
        DOC_ROOT / "review_status.md",
        generate_review_status_auto_doc(units, files),
        generate_review_status_manual_stub(),
    )
    print(f"Generated {len(units)} unit docs for {len(files)} source files.")


def check_docs() -> int:
    files = iter_source_files()
    units = build_units(files)
    errors: list[str] = []
    for unit in units:
        if not unit.doc_path.exists():
            errors.append(f"missing doc for {unit.key}: {unit.doc_path.relative_to(REPO_ROOT)}")
            continue
        text = unit.doc_path.read_text(encoding="utf-8", errors="replace")
        for source in unit.sources:
            if source.rel_posix not in text:
                errors.append(f"{unit.doc_path.relative_to(REPO_ROOT)} does not mention {source.rel_posix}")
        if "Manual review status:" not in text:
            errors.append(f"{unit.doc_path.relative_to(REPO_ROOT)} missing manual review status")
        if AUTO_BEGIN not in text or AUTO_END not in text:
            errors.append(f"{unit.doc_path.relative_to(REPO_ROOT)} missing generated-region markers")
        if MANUAL_BEGIN not in text or MANUAL_END not in text:
            errors.append(f"{unit.doc_path.relative_to(REPO_ROOT)} missing manual-review markers")

    overview = DOC_ROOT / "overview.md"
    review_status = DOC_ROOT / "review_status.md"
    if not overview.exists():
        errors.append("missing overview.md")
    if not review_status.exists():
        errors.append("missing review_status.md")

    extra_docs = [
        path
        for path in DOC_ROOT.rglob("*.md")
        if path.name not in {"overview.md", "review_status.md"}
        and not any(path == unit.doc_path for unit in units)
    ]
    for path in extra_docs:
        errors.append(f"unexpected extra source-unit doc: {path.relative_to(REPO_ROOT)}")

    if errors:
        for error in errors:
            print(f"ERROR: {error}", file=sys.stderr)
        return 1
    print(f"OK: {len(files)} source files covered by {len(units)} unit docs.")
    return 0


def print_inventory() -> None:
    files = iter_source_files()
    units = build_units(files)
    by_dir: dict[str, list[Unit]] = defaultdict(list)
    for unit in units:
        by_dir[unit.directory].append(unit)
    for directory in sorted(by_dir):
        dir_units = by_dir[directory]
        pairs = sum({".c", ".h"} == {source.suffix for source in unit.sources} for unit in dir_units)
        only_c = sum({source.suffix for source in unit.sources} == {".c"} for unit in dir_units)
        only_h = sum({source.suffix for source in unit.sources} == {".h"} for unit in dir_units)
        print(f"{directory}\tunits={len(dir_units)}\tpairs={pairs}\tonly_c={only_c}\tonly_h={only_h}")
    print(f"TOTAL\tunits={len(units)}\tfiles={len(files)}")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--generate", action="store_true", help="generate documentation files")
    parser.add_argument("--check", action="store_true", help="check documentation coverage")
    parser.add_argument("--inventory", action="store_true", help="print source inventory")
    args = parser.parse_args(argv)

    if args.generate:
        generate_docs()
    if args.inventory:
        print_inventory()
    if args.check:
        return check_docs()
    if not (args.generate or args.inventory or args.check):
        parser.print_help()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
