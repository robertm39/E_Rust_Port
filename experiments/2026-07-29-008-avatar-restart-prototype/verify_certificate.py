#!/usr/bin/env python3
"""Independent checker for bounded AVATAR meta-certificates."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
from pathlib import Path
from typing import Any, Callable, Iterable


class VerificationError(RuntimeError):
    """A certificate does not establish its claimed result."""


VARIABLE = re.compile(r"[A-Z_][A-Za-z0-9_]*")


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def uncomment(source: str) -> str:
    """Remove TPTP comments while preserving quoted tokens and newlines."""

    result: list[str] = []
    position = 0
    quoted: str | None = None
    while position < len(source):
        current = source[position]
        if quoted is not None:
            result.append(current)
            if current == "\\" and position + 1 < len(source):
                result.append(source[position + 1])
                position += 2
                continue
            if current == quoted:
                if position + 1 < len(source) and source[position + 1] == quoted:
                    result.append(source[position + 1])
                    position += 2
                    continue
                quoted = None
            position += 1
            continue
        if current in "'\"":
            quoted = current
            result.append(current)
            position += 1
            continue
        if current == "%":
            while position < len(source) and source[position] not in "\r\n":
                result.append(" ")
                position += 1
            continue
        if source[position : position + 2] == "/*":
            result.extend((" ", " "))
            position += 2
            while (
                position < len(source)
                and source[position : position + 2] != "*/"
            ):
                result.append("\n" if source[position] == "\n" else " ")
                position += 1
            if position == len(source):
                raise VerificationError("unterminated block comment")
            result.extend((" ", " "))
            position += 2
            continue
        result.append(current)
        position += 1
    if quoted is not None:
        raise VerificationError("unterminated quote")
    return "".join(result)


def top_level_parts(text: str, separator: str) -> list[str]:
    parts: list[str] = []
    beginning = 0
    stack: list[str] = []
    quoted: str | None = None
    pairs = {")": "(", "]": "[", "}": "{"}
    position = 0
    while position < len(text):
        current = text[position]
        if quoted is not None:
            if current == "\\":
                position += 2
                continue
            if current == quoted:
                if position + 1 < len(text) and text[position + 1] == quoted:
                    position += 2
                    continue
                quoted = None
            position += 1
            continue
        if current in "'\"":
            quoted = current
        elif current in "([{":
            stack.append(current)
        elif current in ")]}":
            if not stack or stack.pop() != pairs[current]:
                raise VerificationError("unbalanced delimiters")
        elif current == separator and not stack:
            parts.append(text[beginning:position].strip())
            beginning = position + 1
        position += 1
    if quoted is not None or stack:
        raise VerificationError("unbalanced expression")
    parts.append(text[beginning:].strip())
    return parts


def source_statements(source: str) -> list[str]:
    return top_level_parts(uncomment(source).strip(), ".")[:-1]


def is_wrapped(text: str) -> bool:
    if len(text) < 2 or text[0] != "(" or text[-1] != ")":
        return False
    depth = 0
    quoted: str | None = None
    position = 0
    while position < len(text):
        current = text[position]
        if quoted is not None:
            if current == "\\":
                position += 2
                continue
            if current == quoted:
                if position + 1 < len(text) and text[position + 1] == quoted:
                    position += 2
                    continue
                quoted = None
        elif current in "'\"":
            quoted = current
        elif current == "(":
            depth += 1
        elif current == ")":
            depth -= 1
            if depth == 0 and position != len(text) - 1:
                return False
        position += 1
    return depth == 0 and quoted is None


def parse_source(source: str) -> list[dict[str, Any]]:
    parsed: list[dict[str, Any]] = []
    for index, statement_body in enumerate(source_statements(source)):
        statement = statement_body.strip() + "."
        prefix, separator, rest = statement_body.partition("(")
        if not separator or prefix.strip().lower() != "cnf":
            raise VerificationError("certificate source is not restricted CNF")
        if not rest.endswith(")"):
            raise VerificationError("malformed CNF source")
        fields = top_level_parts(rest[:-1], ",")
        if len(fields) < 3:
            raise VerificationError("CNF statement has too few fields")
        formula = fields[2].strip()
        while is_wrapped(formula):
            formula = formula[1:-1].strip()
        literals = top_level_parts(formula, "|")
        if not literals or any(not literal for literal in literals):
            raise VerificationError("empty source literal")
        parsed.append(
            {
                "statement_index": index,
                "statement": statement,
                "name": fields[0],
                "role": fields[1],
                "literals": literals,
            }
        )
    return parsed


def variables_in(text: str) -> set[str]:
    found: set[str] = set()
    quoted: str | None = None
    position = 0
    while position < len(text):
        current = text[position]
        if quoted is not None:
            if current == "\\":
                position += 2
                continue
            if current == quoted:
                if position + 1 < len(text) and text[position + 1] == quoted:
                    position += 2
                    continue
                quoted = None
            position += 1
            continue
        if current in "'\"":
            quoted = current
            position += 1
            continue
        match = VARIABLE.match(text, position)
        if match:
            found.add(match.group())
            position = match.end()
        else:
            position += 1
    return found


def independent_components(literals: list[str]) -> list[list[str]]:
    """Compute components with union-find, independently of the generator."""

    parents = list(range(len(literals)))

    def root(item: int) -> int:
        while parents[item] != item:
            parents[item] = parents[parents[item]]
            item = parents[item]
        return item

    def union(left: int, right: int) -> None:
        left_root = root(left)
        right_root = root(right)
        if left_root != right_root:
            parents[right_root] = left_root

    owners: dict[str, int] = {}
    for literal_index, literal in enumerate(literals):
        for variable in variables_in(literal):
            if variable in owners:
                union(owners[variable], literal_index)
            else:
                owners[variable] = literal_index
    grouped: dict[int, list[str]] = {}
    order: list[int] = []
    for literal_index, literal in enumerate(literals):
        component_root = root(literal_index)
        if component_root not in grouped:
            grouped[component_root] = []
            order.append(component_root)
        grouped[component_root].append(literal)
    return [grouped[component_root] for component_root in order]


def alpha_key(literals: Iterable[str]) -> str:
    renaming: dict[str, str] = {}
    normalized: list[str] = []
    for literal_index, literal in enumerate(literals):
        if literal_index:
            normalized.append("|")
        quoted: str | None = None
        position = 0
        while position < len(literal):
            current = literal[position]
            if quoted is not None:
                normalized.append(current)
                if current == "\\" and position + 1 < len(literal):
                    normalized.append(literal[position + 1])
                    position += 2
                    continue
                if current == quoted:
                    if (
                        position + 1 < len(literal)
                        and literal[position + 1] == quoted
                    ):
                        normalized.append(literal[position + 1])
                        position += 2
                        continue
                    quoted = None
                position += 1
                continue
            if current in "'\"":
                quoted = current
                normalized.append(current)
                position += 1
                continue
            match = VARIABLE.match(literal, position)
            if match:
                name = match.group()
                if name not in renaming:
                    renaming[name] = f"V{len(renaming)}"
                normalized.append(renaming[name])
                position = match.end()
                continue
            if not current.isspace():
                normalized.append(current)
            position += 1
    return "".join(normalized)


def expected_abstraction(
    records: list[dict[str, Any]], maximum: int
) -> dict[str, Any]:
    candidates: list[dict[str, Any]] = []
    for record in records:
        components = independent_components(record["literals"])
        if len(components) > 1:
            candidates.append({**record, "components": components})
    selected_set = {
        record["statement_index"]
        for record in sorted(
            candidates,
            key=lambda record: (
                -len(record["components"]),
                -len(record["literals"]),
                record["statement_index"],
            ),
        )[:maximum]
    }
    selectors: dict[str, int] = {}
    splits: list[dict[str, Any]] = []
    for record in candidates:
        if record["statement_index"] not in selected_set:
            continue
        components = []
        for literals in record["components"]:
            key = alpha_key(literals)
            selector = selectors.setdefault(key, len(selectors) + 1)
            components.append(
                {
                    "selector": selector,
                    "canonical": key,
                    "literals": literals,
                }
            )
        splits.append(
            {
                "statement_index": record["statement_index"],
                "name": record["name"],
                "role": record["role"],
                "components": components,
                "selectors": [
                    component["selector"] for component in components
                ],
            }
        )
    return {
        "cnf_count": len(records),
        "selected_split_count": len(splits),
        "selector_count": len(selectors),
        "split_records": splits,
        "split_clauses": [split["selectors"] for split in splits],
    }


def validate_abstraction(
    records: list[dict[str, Any]],
    maximum: int,
    declared: dict[str, Any],
) -> dict[str, Any]:
    candidates = []
    for record in records:
        components = independent_components(record["literals"])
        if len(components) > 1:
            candidates.append({**record, "components": components})
    selected_indices = {
        record["statement_index"]
        for record in sorted(
            candidates,
            key=lambda record: (
                -len(record["components"]),
                -len(record["literals"]),
                record["statement_index"],
            ),
        )[:maximum]
    }
    selected = [
        record
        for record in candidates
        if record["statement_index"] in selected_indices
    ]
    declared_records = declared.get("split_records")
    if (
        declared.get("cnf_count") != len(records)
        or declared.get("selected_split_count") != len(selected)
        or not isinstance(declared_records, list)
        or len(declared_records) != len(selected)
    ):
        raise VerificationError("abstraction counts do not match source")

    selector_by_key: dict[str, int] = {}
    split_clauses: list[list[int]] = []
    for source_record, split_record in zip(
        selected, declared_records, strict=True
    ):
        if (
            split_record.get("statement_index")
            != source_record["statement_index"]
            or split_record.get("name") != source_record["name"]
            or split_record.get("role") != source_record["role"]
        ):
            raise VerificationError("selected split record does not match source")
        components = split_record.get("components")
        if (
            not isinstance(components, list)
            or len(components) != len(source_record["components"])
        ):
            raise VerificationError("declared component count is wrong")
        selectors = []
        for declared_component, independent_component in zip(
            components, source_record["components"], strict=True
        ):
            literals = declared_component.get("literals")
            if (
                not isinstance(literals, list)
                or not all(isinstance(literal, str) for literal in literals)
                or sorted(literals) != sorted(independent_component)
            ):
                raise VerificationError(
                    "declared component is not the independent partition"
                )
            canonical = alpha_key(literals)
            selector = selector_by_key.setdefault(
                canonical, len(selector_by_key) + 1
            )
            if (
                declared_component.get("canonical") != canonical
                or declared_component.get("selector") != selector
            ):
                raise VerificationError("selector reuse is not alpha-canonical")
            selectors.append(selector)
        if split_record.get("selectors") != selectors:
            raise VerificationError("split selector list is inconsistent")
        split_clauses.append(selectors)
    if (
        declared.get("selector_count") != len(selector_by_key)
        or declared.get("split_clauses") != split_clauses
    ):
        raise VerificationError("propositional split abstraction is inconsistent")
    return declared


def render_expected_branch(
    records: list[dict[str, Any]],
    abstraction: dict[str, Any],
    active: list[int],
    source_sha256: str,
    model_index: int,
) -> str:
    split_indices = {
        split["statement_index"] for split in abstraction["split_records"]
    }
    first_component: dict[int, dict[str, Any]] = {}
    for split in abstraction["split_records"]:
        for component in split["components"]:
            first_component.setdefault(component["selector"], component)
    lines = [
        "% Bounded AVATAR restart branch.",
        "% Status   : Unsatisfiable",
        f"% SourceSHA256 : {source_sha256}",
        f"% ModelIndex   : {model_index}",
        f"% ActiveSelectors : {','.join(map(str, active))}",
        "",
    ]
    lines.extend(
        record["statement"]
        for record in records
        if record["statement_index"] not in split_indices
    )
    for selector in active:
        component = first_component[selector]
        formula = " | ".join(component["literals"])
        lines.append(
            f"cnf(avatar_component_{selector}, plain, ({formula}))."
        )
    return "\n".join(lines) + "\n"


def clause_satisfied(clause: list[int], values: dict[int, bool]) -> bool:
    return any(values.get(abs(literal)) == (literal > 0) for literal in clause)


def complete_values(model: list[int], variable_count: int) -> dict[int, bool]:
    values: dict[int, bool] = {}
    for literal in model:
        variable = abs(literal)
        if literal == 0 or not 1 <= variable <= variable_count:
            raise VerificationError("SAT model contains an invalid variable")
        if variable in values:
            raise VerificationError("SAT model assigns a variable twice")
        values[variable] = literal > 0
    if set(values) != set(range(1, variable_count + 1)):
        raise VerificationError("SAT model is incomplete")
    return values


def dpll_satisfiable(
    clauses: list[list[int]], assignment: dict[int, bool] | None = None
) -> bool:
    values = {} if assignment is None else dict(assignment)
    while True:
        reduced: list[list[int]] = []
        unit: int | None = None
        for clause in clauses:
            remaining = [
                literal
                for literal in clause
                if abs(literal) not in values
            ]
            if clause_satisfied(clause, values):
                continue
            if not remaining:
                return False
            if len(remaining) == 1:
                unit = remaining[0]
                break
            reduced.append(remaining)
        if unit is None:
            clauses = reduced
            break
        variable = abs(unit)
        value = unit > 0
        if variable in values and values[variable] != value:
            return False
        values[variable] = value
    if not clauses:
        return True
    branch_literal = clauses[0][0]
    branch_variable = abs(branch_literal)
    for value in (branch_literal > 0, branch_literal < 0):
        branch = dict(values)
        branch[branch_variable] = value
        if dpll_satisfiable(clauses, branch):
            return True
    return False


def default_proof_check(
    problem: Path,
    proof: Path,
    proofcheck: Path,
    validation_gate: Path,
) -> None:
    command = [
        "python3",
        str(validation_gate),
        str(problem),
        str(proof),
        "--proof-command-json",
        json.dumps(
            [
                str(proofcheck),
                "-p",
                "{problem}",
                "{artifact}",
            ]
        ),
    ]
    completed = subprocess.run(
        command,
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        timeout=120,
    )
    if completed.returncode != 0:
        raise VerificationError(
            "ProofCheck rejected branch proof: "
            + (completed.stdout + completed.stderr)[-2000:]
        )


def verify_certificate(
    certificate_path: Path,
    problem_path: Path,
    proofcheck: Path,
    validation_gate: Path,
    proof_callback: Callable[[Path, Path, Path, Path], None] = default_proof_check,
) -> dict[str, Any]:
    certificate = json.loads(certificate_path.read_text(encoding="utf-8"))
    if certificate.get("schema_version") != 1:
        raise VerificationError("unsupported certificate schema")
    if sha256_file(problem_path) != certificate.get("source_sha256"):
        raise VerificationError("source hash mismatch")
    records = parse_source(problem_path.read_text(encoding="utf-8"))
    expected = validate_abstraction(
        records,
        certificate.get("max_split_clauses"),
        certificate.get("abstraction"),
    )

    selector_count = expected["selector_count"]
    clauses = [list(clause) for clause in expected["split_clauses"]]
    base_directory = certificate_path.parent
    all_verified = True
    for expected_index, branch in enumerate(certificate.get("branches", []), 1):
        if branch.get("model_index") != expected_index:
            raise VerificationError("branch indices are not consecutive")
        model = branch.get("sat_model")
        if not isinstance(model, list) or not all(
            isinstance(item, int) for item in model
        ):
            raise VerificationError("malformed SAT model")
        values = complete_values(model, selector_count)
        if not all(clause_satisfied(clause, values) for clause in clauses):
            raise VerificationError("SAT model does not satisfy active formula")
        active = sorted(
            variable for variable, value in values.items() if value
        )
        if branch.get("active_selectors") != active:
            raise VerificationError("active selectors do not match SAT model")

        branch_path = base_directory / branch["branch_path"]
        if (
            not branch_path.is_file()
            or sha256_file(branch_path) != branch.get("branch_sha256")
        ):
            raise VerificationError("branch artifact hash mismatch")
        expected_text = render_expected_branch(
            records,
            expected,
            active,
            certificate["source_sha256"],
            expected_index,
        )
        if branch_path.read_text(encoding="utf-8") != expected_text:
            raise VerificationError("branch artifact is not the declared transform")

        if branch.get("proof_verified") is True:
            proof_path = base_directory / branch["proof_path"]
            if (
                not proof_path.is_file()
                or sha256_file(proof_path) != branch.get("proof_sha256")
            ):
                raise VerificationError("proof artifact hash mismatch")
            proof_callback(
                branch_path, proof_path, proofcheck, validation_gate
            )
            conflict = [-selector for selector in active]
            if branch.get("learned_conflict") != conflict:
                raise VerificationError("learned conflict is not conservative")
            clauses.append(conflict)
        else:
            all_verified = False
            if branch.get("learned_conflict") is not None:
                raise VerificationError("unverified branch learned a conflict")
            if expected_index != len(certificate["branches"]):
                raise VerificationError("search continued after unverified branch")

    final_status = certificate.get("final_status")
    if final_status == "unsatisfiable":
        if not all_verified:
            raise VerificationError("UNSAT claim depends on unverified branch")
        if dpll_satisfiable(clauses):
            raise VerificationError("final propositional formula is SAT")
    elif final_status == "unknown":
        if not dpll_satisfiable(clauses):
            raise VerificationError("unknown claim hides derived SAT UNSAT")
    else:
        raise VerificationError("invalid final status")
    return {
        "verified": True,
        "final_status": final_status,
        "branch_count": len(certificate.get("branches", [])),
        "verified_conflicts": sum(
            branch.get("proof_verified") is True
            for branch in certificate.get("branches", [])
        ),
        "propositional_clause_count": len(clauses),
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("certificate", type=Path)
    parser.add_argument("problem", type=Path)
    parser.add_argument("--proofcheck", type=Path, required=True)
    parser.add_argument("--validation-gate", type=Path, required=True)
    return parser.parse_args()


def main() -> None:
    arguments = parse_args()
    report = verify_certificate(
        arguments.certificate.resolve(),
        arguments.problem.resolve(),
        arguments.proofcheck.resolve(),
        arguments.validation_gate.resolve(),
    )
    print(json.dumps(report, sort_keys=True))


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, VerificationError) as error:
        print(f"verification error: {error}")
        raise SystemExit(1) from error
