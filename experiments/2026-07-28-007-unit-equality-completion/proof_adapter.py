#!/usr/bin/env python3
"""Prepare ProofCheck-facing views of axiom-only TSTP refutations.

The adapter is deliberately narrow. It accepts only CNF input leaves and
proves that every checker-facing leaf is alpha-equivalent to its cited source.
The ProofCheck view changes only ``file()`` targets and creates a controller
problem whose clauses use the proof's alpha-renamed variables. This works
around ProofCheck 1.0's spelling-sensitive source-leaf comparison without
changing any proof formula, role, inference, or parent.
"""

from __future__ import annotations

import hashlib
import re
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Sequence


ANNOTATED_RE = re.compile(
    r"^\s*(cnf|fof|tff|tcf|thf)\s*\((.*)\)\s*\.\s*$",
    re.DOTALL | re.IGNORECASE,
)
FILE_SOURCE_RE = re.compile(
    r"^file\s*\(\s*'((?:\\.|[^'])*)'\s*,\s*([^)]+?)\s*\)$",
    re.DOTALL | re.IGNORECASE,
)
VARIABLE_RE = re.compile(r"^[A-Z][A-Za-z0-9_]*$")
TOKEN_RE = re.compile(
    r"""
    '(?:\\.|[^'])*'
    | "(?:\\.|[^"])*"
    | <=> | <~> | => | <= | != | ~\| | ~&
    | [A-Za-z$][A-Za-z0-9_$]*
    | \d+(?:\.\d+)?
    | [()[\]{},.:=|&~!?@+*^<>/-]
    """,
    re.VERBOSE,
)
PROOF_START_RE = re.compile(
    r"^[%#]\s*SZS\s+output\s+start\s+"
    r"(CNFRefutation|Refutation|Proof)\b",
    re.IGNORECASE,
)
PROOF_END_RE = re.compile(
    r"^[%#]\s*SZS\s+output\s+end\s+"
    r"(CNFRefutation|Refutation|Proof)\b",
    re.IGNORECASE,
)


class AdapterError(RuntimeError):
    """A proof is outside the adapter's audited axiom-only CNF contract."""


@dataclass(frozen=True)
class AnnotatedFormula:
    """A parsed top-level TPTP annotated formula."""

    kind: str
    fields: tuple[str, ...]

    @property
    def name(self) -> str:
        return normalize_name(self.fields[0])

    @property
    def role(self) -> str:
        return self.fields[1].strip().lower()

    @property
    def body(self) -> str:
        return self.fields[2].strip()

    def with_fields(self, fields: Sequence[str]) -> "AnnotatedFormula":
        return AnnotatedFormula(self.kind, tuple(fields))

    def render(self) -> str:
        return f"{self.kind}({', '.join(self.fields)})."


def sha256_text(text: str) -> str:
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def normalize_name(value: str) -> str:
    value = value.strip()
    if len(value) >= 2 and value[0] == value[-1] == "'":
        return value[1:-1].replace("\\'", "'").replace("\\\\", "\\")
    return value


def split_tptp_statements(text: str) -> list[str]:
    """Split a TPTP document at top-level full stops."""

    statements: list[str] = []
    current: list[str] = []
    stack: list[str] = []
    quote: str | None = None
    escaped = False
    line_start = True
    comment = False
    pairs = {")": "(", "]": "[", "}": "{"}

    for char in text:
        if comment:
            if char == "\n":
                comment = False
                line_start = True
                if current and current[-1] != "\n":
                    current.append("\n")
            continue
        if quote is not None:
            current.append(char)
            if escaped:
                escaped = False
            elif char == "\\":
                escaped = True
            elif char == quote:
                quote = None
            line_start = char == "\n"
            continue
        if line_start and char in {"%", "#"}:
            comment = True
            continue
        if char in {"'", '"', "`"}:
            quote = char
            current.append(char)
            line_start = False
            continue
        if char in "([{":
            stack.append(char)
        elif char in ")]}":
            if not stack or stack[-1] != pairs[char]:
                raise AdapterError("unbalanced delimiter in TPTP document")
            stack.pop()
        current.append(char)
        if char == "." and not stack:
            statement = "".join(current).strip()
            if statement:
                statements.append(statement)
            current = []
        line_start = char == "\n"

    if quote is not None or stack:
        raise AdapterError("unterminated quote or delimiter in TPTP document")
    if "".join(current).strip():
        raise AdapterError("unterminated TPTP statement")
    return statements


def split_top_level(value: str, delimiter: str = ",") -> list[str]:
    """Split one parenthesized TPTP payload at top-level delimiters."""

    fields: list[str] = []
    current: list[str] = []
    stack: list[str] = []
    quote: str | None = None
    escaped = False
    pairs = {")": "(", "]": "[", "}": "{"}

    for char in value:
        if quote is not None:
            current.append(char)
            if escaped:
                escaped = False
            elif char == "\\":
                escaped = True
            elif char == quote:
                quote = None
            continue
        if char in {"'", '"', "`"}:
            quote = char
            current.append(char)
            continue
        if char in "([{":
            stack.append(char)
        elif char in ")]}":
            if not stack or stack[-1] != pairs[char]:
                raise AdapterError("unbalanced annotated-formula field")
            stack.pop()
        if char == delimiter and not stack:
            fields.append("".join(current).strip())
            current = []
        else:
            current.append(char)

    if quote is not None or stack:
        raise AdapterError("unterminated annotated-formula field")
    fields.append("".join(current).strip())
    return fields


def parse_annotated(statement: str) -> AnnotatedFormula | None:
    match = ANNOTATED_RE.match(statement)
    if match is None:
        return None
    fields = split_top_level(match.group(2))
    if len(fields) < 3:
        raise AdapterError("annotated formula has fewer than three fields")
    return AnnotatedFormula(match.group(1).lower(), tuple(fields))


def tokenize_formula(value: str) -> list[str]:
    """Tokenize enough untyped CNF syntax to audit alpha equivalence."""

    tokens: list[str] = []
    position = 0
    while position < len(value):
        if value[position].isspace():
            position += 1
            continue
        match = TOKEN_RE.match(value, position)
        if match is None:
            raise AdapterError(
                f"unsupported formula token near {value[position:position + 24]!r}"
            )
        tokens.append(match.group(0))
        position = match.end()
    return strip_outer_parentheses(tokens)


def strip_outer_parentheses(tokens: list[str]) -> list[str]:
    result = list(tokens)
    while len(result) >= 2 and result[0] == "(" and result[-1] == ")":
        depth = 0
        encloses_all = True
        for index, token in enumerate(result):
            if token == "(":
                depth += 1
            elif token == ")":
                depth -= 1
                if depth == 0 and index != len(result) - 1:
                    encloses_all = False
                    break
        if not encloses_all or depth != 0:
            break
        result = result[1:-1]
    return result


def alpha_canonical_tokens(value: str) -> tuple[str, ...]:
    variable_names: dict[str, str] = {}
    canonical: list[str] = []
    for token in tokenize_formula(value):
        if VARIABLE_RE.match(token):
            replacement = variable_names.setdefault(
                token, f"VAR{len(variable_names) + 1}"
            )
            canonical.append(replacement)
        else:
            canonical.append(token)
    return tuple(canonical)


def alpha_equivalent(left: str, right: str) -> bool:
    return alpha_canonical_tokens(left) == alpha_canonical_tokens(right)


def extract_proof_block(solution_text: str) -> str:
    """Extract the sole proof block using the same SZS envelope contract."""

    blocks: list[str] = []
    active: list[str] | None = None
    for line in solution_text.splitlines(keepends=True):
        if PROOF_START_RE.match(line):
            if active is not None:
                raise AdapterError("nested proof output blocks")
            active = []
            continue
        if PROOF_END_RE.match(line):
            if active is None:
                raise AdapterError("proof output end without start")
            blocks.append("".join(active))
            active = None
            continue
        if active is not None:
            active.append(line)
    if active is not None:
        raise AdapterError("unterminated proof output block")
    if len(blocks) != 1:
        raise AdapterError(f"expected one proof output block, found {len(blocks)}")
    return blocks[0]


def source_formula(
    source_path: Path,
    label: str,
    cache: dict[Path, dict[str, AnnotatedFormula]],
) -> AnnotatedFormula:
    resolved = source_path.resolve()
    formulas = cache.get(resolved)
    if formulas is None:
        formulas = {}
        for statement in split_tptp_statements(
            resolved.read_text(encoding="utf-8")
        ):
            formula = parse_annotated(statement)
            if formula is not None:
                if formula.name in formulas:
                    raise AdapterError(
                        f"duplicate source label {formula.name!r} in {resolved}"
                    )
                formulas[formula.name] = formula
        cache[resolved] = formulas
    try:
        return formulas[label]
    except KeyError as error:
        raise AdapterError(
            f"source label {label!r} is absent from {resolved}"
        ) from error


def parse_file_source(
    source: str, base_path: Path
) -> tuple[Path, str] | None:
    match = FILE_SOURCE_RE.match(source.strip())
    if match is None:
        return None
    raw_path = match.group(1).replace("\\'", "'").replace("\\\\", "\\")
    path = Path(raw_path)
    if not path.is_absolute():
        path = base_path / path
    return path, normalize_name(match.group(2))


def adapt_proofcheck_sources(
    *,
    proof_text: str,
    proof_base: Path,
    controller_path: Path,
) -> tuple[str, str, dict[str, Any]]:
    """Return a ProofCheck view with alpha-audited source-leaf spelling."""

    parsed: list[AnnotatedFormula] = []
    for statement in split_tptp_statements(proof_text):
        formula = parse_annotated(statement)
        if formula is None:
            raise AdapterError("proof contains a non-annotated TPTP statement")
        parsed.append(formula)
    if not parsed:
        raise AdapterError("proof contains no annotated formulas")
    names = [formula.name for formula in parsed]
    if len(set(names)) != len(names):
        raise AdapterError("proof formula names are not unique")

    cache: dict[Path, dict[str, AnnotatedFormula]] = {}
    controller_premises: dict[str, AnnotatedFormula] = {}
    adapted: list[AnnotatedFormula] = []
    leaf_audits: list[dict[str, Any]] = []
    negated_input_count = 0
    quoted_controller = str(controller_path.resolve()).replace("\\", "\\\\")
    quoted_controller = quoted_controller.replace("'", "\\'")

    for formula in parsed:
        fields = list(formula.fields)
        file_source = (
            parse_file_source(fields[3], proof_base)
            if len(fields) >= 4
            else None
        )
        if file_source is None:
            adapted.append(formula)
            continue
        if formula.kind != "cnf":
            raise AdapterError(
                f"input leaf {formula.name!r} is not an untyped CNF clause"
            )
        source_path, source_label = file_source
        source = source_formula(source_path, source_label, cache)
        if source.kind != "cnf":
            raise AdapterError(
                f"cited source {source_label!r} is not an untyped CNF clause"
            )
        if source.role not in {"axiom", "negated_conjecture"}:
            raise AdapterError(
                f"unsupported source role {source.role!r} in axiom-only proof"
            )
        if formula.role != source.role:
            raise AdapterError(
                f"proof leaf {formula.name!r} role {formula.role!r} does not "
                f"match cited source role {source.role!r}"
            )
        if not alpha_equivalent(formula.body, source.body):
            raise AdapterError(
                f"input leaf {formula.name!r} is not alpha-equivalent "
                f"to cited source {source_label!r}"
            )
        if source.role == "negated_conjecture":
            negated_input_count += 1

        premise = AnnotatedFormula(
            "cnf", (source.fields[0], source.fields[1], formula.body)
        )
        existing = controller_premises.get(source_label)
        if existing is not None and existing.fields != premise.fields:
            raise AdapterError(
                f"distinct input leaves reuse source label {source_label!r}"
            )
        controller_premises[source_label] = premise
        fields[3] = f"file('{quoted_controller}',{source.fields[0]})"
        adapted.append(formula.with_fields(fields))
        leaf_audits.append(
            {
                "proof_name": formula.name,
                "source_label": source_label,
                "source_path": str(source_path.resolve()),
                "source_role": source.role,
                "proof_body_sha256": sha256_text(formula.body),
                "source_body_sha256": sha256_text(source.body),
                "alpha_canonical_sha256": sha256_text(
                    "\x1f".join(alpha_canonical_tokens(source.body))
                ),
            }
        )

    if not leaf_audits:
        raise AdapterError("proof has no file-cited input leaves")
    if negated_input_count == 0:
        raise AdapterError("proof has no negated-conjecture input premise")
    controller = (
        "\n".join(
            premise.render()
            for premise in controller_premises.values()
        )
        + "\n"
    )
    prepared = "\n".join(formula.render() for formula in adapted) + "\n"
    audit = {
        "schema_version": 1,
        "adapter": "proofcheck-1.0-alpha-source-controller",
        "proof_formula_count": len(parsed),
        "input_leaf_count": len(leaf_audits),
        "negated_input_count": negated_input_count,
        "controller_premise_count": len(controller_premises),
        "changed_fields": ["input_leaf.file_source"],
        "logical_proof_fields_unchanged": True,
        "leaf_audits": leaf_audits,
        "original_proof_sha256": sha256_text(proof_text),
        "prepared_proof_sha256": sha256_text(prepared),
        "controller_sha256": sha256_text(controller),
    }
    return prepared, controller, audit


def write_proofcheck_view(
    *,
    solution_path: Path,
    prepared_path: Path,
    controller_path: Path,
) -> dict[str, Any]:
    """Extract and persist one alpha-audited ProofCheck view."""

    proof_text = extract_proof_block(
        solution_path.read_text(encoding="utf-8", errors="strict")
    )
    prepared, controller, audit = adapt_proofcheck_sources(
        proof_text=proof_text,
        proof_base=solution_path.parent,
        controller_path=controller_path,
    )
    prepared_path.parent.mkdir(parents=True, exist_ok=True)
    controller_path.parent.mkdir(parents=True, exist_ok=True)
    prepared_path.write_text(prepared, encoding="utf-8", newline="\n")
    controller_path.write_text(controller, encoding="utf-8", newline="\n")
    return audit
