#!/usr/bin/env python3
"""Canonical ground-theory protocol, renderer, and exact evidence verifier."""

from __future__ import annotations

import dataclasses
import hashlib
import json
import re
import subprocess
import time
from fractions import Fraction
from pathlib import Path
from typing import Any, Iterable, Sequence


SAFE_SYMBOL = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*$")


class ProtocolError(ValueError):
    """Raised when a corpus or solver response violates the frozen protocol."""


@dataclasses.dataclass(frozen=True)
class SolverResult:
    workload_id: str
    branch_id: str
    raw_status: str
    elapsed_ns: int
    core: tuple[str, ...] = ()
    model: tuple[tuple[str, str], ...] = ()
    reason: str = ""

    def normalized_evidence(self) -> dict[str, Any]:
        return {
            "workload": self.workload_id,
            "branch": self.branch_id,
            "status": self.raw_status,
            "core": sorted(self.core),
            "model": sorted(self.model),
            "reason": self.reason,
        }


@dataclasses.dataclass(frozen=True)
class Verification:
    trusted_status: str
    verified: bool
    evidence_kind: str
    reason: str


def load_corpus(path: Path) -> dict[str, Any]:
    corpus = json.loads(path.read_text(encoding="utf-8"))
    if corpus.get("schema") != "umlaut-ground-theory-corpus-v1":
        raise ProtocolError("unexpected corpus schema")
    workload_ids: set[str] = set()
    for workload in corpus.get("workloads", []):
        validate_workload(workload)
        if workload["id"] in workload_ids:
            raise ProtocolError(f"duplicate workload ID: {workload['id']}")
        workload_ids.add(workload["id"])
    return corpus


def validate_workload(workload: dict[str, Any]) -> None:
    required = {
        "id",
        "partition",
        "cohort",
        "sort",
        "eligible",
        "expected_closed",
        "variables",
        "base",
        "branches",
    }
    missing = required - workload.keys()
    if missing:
        raise ProtocolError(f"workload is missing {sorted(missing)}")
    if workload["sort"] not in {"Int", "Real"}:
        raise ProtocolError(f"unsupported sort: {workload['sort']}")
    variables = workload["variables"]
    if len(variables) != len(set(variables)):
        raise ProtocolError("duplicate variable")
    if any(not SAFE_SYMBOL.fullmatch(variable) for variable in variables):
        raise ProtocolError("unsafe variable name")
    labels: set[str] = set()
    for constraint in workload["base"]:
        validate_constraint(constraint, variables, workload["sort"], labels)
    branch_ids: set[str] = set()
    for branch in workload["branches"]:
        if branch["id"] in branch_ids:
            raise ProtocolError("duplicate branch ID")
        branch_ids.add(branch["id"])
        branch_labels = set(labels)
        for constraint in branch["constraints"]:
            validate_constraint(
                constraint, variables, workload["sort"], branch_labels
            )
    if workload["eligible"]:
        constraints = list(workload["base"])
        constraints.extend(
            constraint
            for branch in workload["branches"]
            for constraint in branch["constraints"]
        )
        if any(constraint["kind"] != "difference" for constraint in constraints):
            raise ProtocolError("eligible workload contains unsupported constraint")


def validate_constraint(
    constraint: dict[str, Any],
    variables: Sequence[str],
    sort: str,
    labels: set[str],
) -> None:
    label = constraint.get("label")
    if not isinstance(label, str) or not SAFE_SYMBOL.fullmatch(label):
        raise ProtocolError(f"unsafe constraint label: {label!r}")
    if label in labels:
        raise ProtocolError(f"duplicate constraint label: {label}")
    labels.add(label)
    bound = parse_fraction(str(constraint.get("bound")))
    if sort == "Int" and bound.denominator != 1:
        raise ProtocolError("integer workload has a fractional bound")
    if constraint.get("kind") == "difference":
        for endpoint in (constraint.get("lhs"), constraint.get("rhs")):
            if endpoint != "zero" and endpoint not in variables:
                raise ProtocolError(f"unknown difference endpoint: {endpoint}")
    elif constraint.get("kind") == "general_linear":
        terms = constraint.get("terms")
        if not isinstance(terms, dict) or not terms:
            raise ProtocolError("general linear constraint has no terms")
        if any(variable not in variables for variable in terms):
            raise ProtocolError("general linear constraint has unknown variable")
        if any(not isinstance(coefficient, int) for coefficient in terms.values()):
            raise ProtocolError("general linear coefficient is not an integer")
    else:
        raise ProtocolError(f"unknown constraint kind: {constraint.get('kind')}")


def parse_fraction(text: str) -> Fraction:
    try:
        return Fraction(text)
    except (ValueError, ZeroDivisionError) as error:
        raise ProtocolError(f"invalid exact rational: {text}") from error


def fraction_smt(value: Fraction) -> str:
    if value.denominator == 1:
        if value.numerator >= 0:
            return str(value.numerator)
        return f"(- {-value.numerator})"
    numerator = abs(value.numerator)
    body = f"(/ {numerator} {value.denominator})"
    return body if value.numerator >= 0 else f"(- {body})"


def linear_term_smt(terms: dict[str, int]) -> str:
    rendered: list[str] = []
    for variable in sorted(terms):
        coefficient = terms[variable]
        if coefficient == 0:
            continue
        if coefficient == 1:
            rendered.append(variable)
        elif coefficient == -1:
            rendered.append(f"(- {variable})")
        else:
            rendered.append(f"(* {coefficient} {variable})")
    if not rendered:
        return "0"
    if len(rendered) == 1:
        return rendered[0]
    return f"(+ {' '.join(rendered)})"


def constraint_smt(constraint: dict[str, Any]) -> str:
    bound = fraction_smt(parse_fraction(constraint["bound"]))
    if constraint["kind"] == "difference":
        lhs = "0" if constraint["lhs"] == "zero" else constraint["lhs"]
        rhs = "0" if constraint["rhs"] == "zero" else constraint["rhs"]
        expression = f"(- {lhs} {rhs})"
    else:
        expression = linear_term_smt(constraint["terms"])
    return f"(<= {expression} {bound})"


def named_assertion_smt(constraint: dict[str, Any]) -> str:
    return (
        f"(assert (! {constraint_smt(constraint)} "
        f":named {constraint['label']}))"
    )


def declarations_smt(workload: dict[str, Any]) -> list[str]:
    return [
        f"(declare-const {variable} {workload['sort']})"
        for variable in workload["variables"]
    ]


def asserted_constraints(
    workload: dict[str, Any], branch: dict[str, Any]
) -> list[dict[str, Any]]:
    return [*workload["base"], *branch["constraints"]]


def tokenize_sexpr(text: str) -> list[str]:
    tokens: list[str] = []
    index = 0
    while index < len(text):
        character = text[index]
        if character.isspace():
            index += 1
        elif character in "()":
            tokens.append(character)
            index += 1
        elif character == "|":
            end = index + 1
            while end < len(text) and text[end] != "|":
                end += 1
            if end >= len(text):
                raise ProtocolError("unterminated quoted symbol")
            tokens.append(text[index + 1 : end])
            index = end + 1
        else:
            end = index
            while end < len(text) and not text[end].isspace() and text[end] not in "()":
                end += 1
            tokens.append(text[index:end])
            index = end
    return tokens


def parse_sexpr(text: str) -> Any:
    tokens = tokenize_sexpr(text)
    position = 0

    def parse_one() -> Any:
        nonlocal position
        if position >= len(tokens):
            raise ProtocolError("unexpected end of S-expression")
        token = tokens[position]
        position += 1
        if token == "(":
            values = []
            while position < len(tokens) and tokens[position] != ")":
                values.append(parse_one())
            if position >= len(tokens):
                raise ProtocolError("unterminated S-expression")
            position += 1
            return values
        if token == ")":
            raise ProtocolError("unexpected close parenthesis")
        return token

    value = parse_one()
    if position != len(tokens):
        raise ProtocolError("trailing S-expression tokens")
    return value


def numeral_fraction(value: Any) -> Fraction:
    if isinstance(value, str):
        if value.endswith("?"):
            raise ProtocolError("inexact algebraic numeral")
        return parse_fraction(value)
    if not isinstance(value, list) or not value:
        raise ProtocolError("invalid numeral expression")
    if value[0] == "-" and len(value) == 2:
        return -numeral_fraction(value[1])
    if value[0] == "/" and len(value) == 3:
        denominator = numeral_fraction(value[2])
        if denominator == 0:
            raise ProtocolError("zero numeral denominator")
        return numeral_fraction(value[1]) / denominator
    if value[0] == "to_real" and len(value) == 2:
        return numeral_fraction(value[1])
    raise ProtocolError(f"unsupported numeral expression: {value!r}")


def parse_core(text: str) -> tuple[str, ...]:
    parsed = parse_sexpr(text)
    if not isinstance(parsed, list) or any(not isinstance(item, str) for item in parsed):
        raise ProtocolError("unsat core is not a symbol list")
    core = tuple(parsed)
    if len(core) != len(set(core)):
        raise ProtocolError("unsat core contains a duplicate label")
    return core


def parse_get_value(text: str) -> tuple[tuple[str, str], ...]:
    parsed = parse_sexpr(text)
    if not isinstance(parsed, list):
        raise ProtocolError("model values are not a list")
    model: list[tuple[str, str]] = []
    for pair in parsed:
        if (
            not isinstance(pair, list)
            or len(pair) != 2
            or not isinstance(pair[0], str)
        ):
            raise ProtocolError("malformed get-value pair")
        value = numeral_fraction(pair[1])
        model.append((pair[0], fraction_text(value)))
    if len(model) != len({name for name, _ in model}):
        raise ProtocolError("model contains a duplicate variable")
    return tuple(model)


def parse_ffi_model(text: str) -> tuple[tuple[str, str], ...]:
    if not text:
        return ()
    model = []
    for field in text.split(";"):
        if not field:
            continue
        if "=" not in field:
            raise ProtocolError("malformed FFI model field")
        name, raw_value = field.split("=", 1)
        if not SAFE_SYMBOL.fullmatch(name):
            raise ProtocolError("unsafe FFI model variable")
        model.append((name, fraction_text(numeral_fraction(parse_sexpr(raw_value)))))
    return tuple(model)


def fraction_text(value: Fraction) -> str:
    if value.denominator == 1:
        return str(value.numerator)
    return f"{value.numerator}/{value.denominator}"


def verify_result(
    workload: dict[str, Any],
    branch: dict[str, Any],
    result: SolverResult,
) -> Verification:
    if not workload["eligible"]:
        return Verification("unknown", False, "unsupported", "workload is outside replay fragment")
    constraints = asserted_constraints(workload, branch)
    if result.raw_status == "unsat":
        try:
            verified = verify_unsat_core(constraints, result.core)
        except ProtocolError as error:
            return Verification("unknown", False, "core", str(error))
        return Verification(
            "unsat" if verified else "unknown",
            verified,
            "core",
            "exact negative cycle" if verified else "core has no negative cycle",
        )
    if result.raw_status == "sat":
        try:
            verified = verify_model(
                workload["variables"], constraints, dict(result.model)
            )
        except ProtocolError as error:
            return Verification("unknown", False, "model", str(error))
        return Verification(
            "sat" if verified else "unknown",
            verified,
            "model",
            "all exact constraints hold" if verified else "model violates a constraint",
        )
    return Verification("unknown", False, "none", result.reason or result.raw_status)


def verify_unsat_core(
    constraints: Sequence[dict[str, Any]], core: Sequence[str]
) -> bool:
    if not core:
        raise ProtocolError("empty unsat core")
    by_label = {constraint["label"]: constraint for constraint in constraints}
    if any(label not in by_label for label in core):
        raise ProtocolError("unsat core refers to an unknown assertion")
    selected = [by_label[label] for label in core]
    if any(constraint["kind"] != "difference" for constraint in selected):
        raise ProtocolError("unsat core contains an unsupported constraint")
    vertices = {"zero"}
    for constraint in selected:
        vertices.add(constraint["lhs"])
        vertices.add(constraint["rhs"])
    distances = {vertex: Fraction(0) for vertex in vertices}
    for iteration in range(len(vertices)):
        changed = False
        for constraint in selected:
            source = constraint["rhs"]
            target = constraint["lhs"]
            candidate = distances[source] + parse_fraction(constraint["bound"])
            if candidate < distances[target]:
                distances[target] = candidate
                changed = True
                if iteration == len(vertices) - 1:
                    return True
        if not changed:
            return False
    return False


def verify_model(
    variables: Sequence[str],
    constraints: Sequence[dict[str, Any]],
    raw_model: dict[str, str],
) -> bool:
    if set(raw_model) != set(variables):
        raise ProtocolError("model variable set does not match workload")
    model = {name: parse_fraction(value) for name, value in raw_model.items()}
    model["zero"] = Fraction(0)
    for constraint in constraints:
        bound = parse_fraction(constraint["bound"])
        if constraint["kind"] == "difference":
            value = model[constraint["lhs"]] - model[constraint["rhs"]]
        else:
            value = sum(
                coefficient * model[variable]
                for variable, coefficient in constraint["terms"].items()
            )
        if value > bound:
            return False
    return True


class ProcessSession:
    """Persistent shell-free SMT-LIB process with incremental branch checks."""

    def __init__(self, executable: Path, timeout_ms: int = 5_000) -> None:
        self.executable = executable
        self.timeout_ms = timeout_ms
        self.process: subprocess.Popen[str] | None = None
        self.startup_ns = 0
        self.shutdown_ns = 0

    def __enter__(self) -> "ProcessSession":
        started = time.perf_counter_ns()
        self.process = subprocess.Popen(
            [str(self.executable), "-in", "-smt2"],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            encoding="utf-8",
            errors="strict",
            bufsize=1,
        )
        self.startup_ns = time.perf_counter_ns() - started
        return self

    def __exit__(self, exc_type: Any, exc: Any, traceback: Any) -> None:
        if self.process is None:
            return
        started = time.perf_counter_ns()
        try:
            self._write("(exit)")
            self.process.wait(timeout=2)
        except (BrokenPipeError, subprocess.TimeoutExpired):
            self.process.kill()
            self.process.wait(timeout=2)
        self.shutdown_ns = time.perf_counter_ns() - started

    def _write(self, command: str) -> None:
        if self.process is None or self.process.stdin is None:
            raise ProtocolError("SMT process is not running")
        self.process.stdin.write(command + "\n")
        self.process.stdin.flush()

    def _read_line(self) -> str:
        if self.process is None or self.process.stdout is None:
            raise ProtocolError("SMT process is not running")
        line = self.process.stdout.readline()
        if line == "":
            stderr = ""
            if self.process.stderr is not None:
                stderr = self.process.stderr.read()
            raise ProtocolError(f"SMT process ended unexpectedly: {stderr.strip()}")
        return line.strip()

    def _read_sexpr(self) -> str:
        parts: list[str] = []
        balance = 0
        started = False
        while True:
            line = self._read_line()
            parts.append(line)
            for character in line:
                if character == "(":
                    balance += 1
                    started = True
                elif character == ")":
                    balance -= 1
            if started and balance == 0:
                text = " ".join(parts)
                if text.startswith("(error"):
                    raise ProtocolError(f"SMT solver error: {text}")
                return text
            if balance < 0 or len(parts) > 10_000:
                raise ProtocolError("malformed SMT S-expression response")

    def run_workload(self, workload: dict[str, Any]) -> list[SolverResult]:
        if self.process is None:
            raise ProtocolError("SMT process is not running")
        self._write("(reset)")
        self._write("(set-option :print-success false)")
        self._write("(set-option :produce-unsat-cores true)")
        self._write("(set-option :produce-models true)")
        self._write(f"(set-option :timeout {self.timeout_ms})")
        self._write("(set-option :smt.random_seed 0)")
        self._write("(set-option :sat.random_seed 0)")
        for declaration in declarations_smt(workload):
            self._write(declaration)
        for constraint in workload["base"]:
            self._write(named_assertion_smt(constraint))

        results = []
        for branch in workload["branches"]:
            self._write("(push 1)")
            for constraint in branch["constraints"]:
                self._write(named_assertion_smt(constraint))
            started = time.perf_counter_ns()
            self._write("(check-sat)")
            status = self._read_line()
            core: tuple[str, ...] = ()
            model: tuple[tuple[str, str], ...] = ()
            reason = ""
            if status == "unsat":
                self._write("(get-unsat-core)")
                core = parse_core(self._read_sexpr())
            elif status == "sat":
                names = " ".join(workload["variables"])
                self._write(f"(get-value ({names}))")
                model = parse_get_value(self._read_sexpr())
            elif status == "unknown":
                self._write("(get-info :reason-unknown)")
                reason = self._read_sexpr()
            else:
                raise ProtocolError(f"unrecognized SMT status: {status}")
            elapsed_ns = time.perf_counter_ns() - started
            self._write("(pop 1)")
            results.append(
                SolverResult(
                    workload["id"],
                    branch["id"],
                    status,
                    elapsed_ns,
                    core,
                    model,
                    reason,
                )
            )
        return results


def write_ffi_protocol(path: Path, workloads: Iterable[dict[str, Any]]) -> None:
    lines = ["UMLAUT_GROUND_THEORY_FFI_V1"]
    for workload in workloads:
        lines.append(
            "\t".join(
                ["WORKLOAD", workload["id"], workload["sort"]]
            )
        )
        for variable in workload["variables"]:
            lines.append("\t".join(["VAR", variable]))
        for constraint in workload["base"]:
            lines.append(
                "\t".join(
                    ["BASE", constraint["label"], constraint_smt(constraint)]
                )
            )
        for branch in workload["branches"]:
            lines.append("\t".join(["BRANCH", branch["id"]]))
            for constraint in branch["constraints"]:
                lines.append(
                    "\t".join(
                        ["ASSERT", constraint["label"], constraint_smt(constraint)]
                    )
                )
            lines.append("END_BRANCH")
        lines.append("END_WORKLOAD")
    lines.append("END")
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def parse_ffi_results(path: Path) -> tuple[list[SolverResult], dict[str, str]]:
    results: list[SolverResult] = []
    metadata: dict[str, str] = {}
    for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        fields = line.split("\t")
        if fields[0] == "META" and len(fields) == 3:
            metadata[fields[1]] = fields[2]
        elif fields[0] == "RESULT" and len(fields) == 8:
            core = tuple(filter(None, fields[5].split(",")))
            model = parse_ffi_model(fields[6])
            results.append(
                SolverResult(
                    workload_id=fields[1],
                    branch_id=fields[2],
                    raw_status=fields[3],
                    elapsed_ns=int(fields[4]),
                    core=core,
                    model=model,
                    reason=fields[7],
                )
            )
        else:
            raise ProtocolError(f"malformed FFI result line {line_number}")
    return results, metadata


def evidence_hash(results: Sequence[SolverResult]) -> str:
    normalized = [result.normalized_evidence() for result in results]
    payload = json.dumps(normalized, sort_keys=True, separators=(",", ":")).encode()
    return hashlib.sha256(payload).hexdigest()
