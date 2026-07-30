#!/usr/bin/env python3
"""Independently verify every reproducible induction-only test proof."""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import os
import sys
from pathlib import Path
from types import ModuleType
from typing import Any, Sequence

import analyze
import verify_schema


EXPERIMENT_ROOT = Path(__file__).resolve().parent
REPO_ROOT = EXPERIMENT_ROOT.parent.parent
BASE_EXPERIMENT = (
    EXPERIMENT_ROOT.parent / "2026-07-28-007-unit-equality-completion"
)
BASE_VERIFY_PATH = BASE_EXPERIMENT / "verify.py"
ADAPTER_PATH = BASE_EXPERIMENT / "proof_adapter.py"


class VerificationError(RuntimeError):
    """A checker setup, typed adapter, or proof-validation gate failed."""


def load_module(name: str, path: Path) -> ModuleType:
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise VerificationError(f"cannot load module: {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


BASE_VERIFY = load_module("integer_induction_base_verifier", BASE_VERIFY_PATH)
ADAPTER = load_module("integer_induction_typed_adapter", ADAPTER_PATH)
PROOFCHECK = BASE_VERIFY.PROOFCHECK


class FormulaParser:
    """Parse the first-order fragment emitted in typed Umlaut proofs."""

    BINARY_PRECEDENCE = {
        "<=>": 1,
        "<~>": 1,
        "=>": 2,
        "<=": 2,
        "|": 3,
        "~|": 3,
        "&": 4,
        "~&": 4,
        "=": 5,
        "!=": 5,
    }
    RIGHT_ASSOCIATIVE = {"<=>", "<~>", "=>", "<="}

    def __init__(self, value: str) -> None:
        self.tokens = ADAPTER.tokenize_formula(value)
        self.position = 0
        self.binder_count = 0
        self.bound_variables: dict[str, str] = {}
        self.free_variables: dict[str, str] = {}

    def parse(self) -> tuple[Any, ...]:
        expression = self.parse_expression(0)
        if self.position != len(self.tokens):
            raise VerificationError(
                "unsupported formula suffix near "
                f"{self.tokens[self.position:self.position + 8]!r}"
            )
        return expression

    def current(self) -> str | None:
        if self.position == len(self.tokens):
            return None
        return self.tokens[self.position]

    def consume(self, expected: str | None = None) -> str:
        token = self.current()
        if token is None:
            raise VerificationError("unexpected end of formula")
        if expected is not None and token != expected:
            raise VerificationError(
                f"expected formula token {expected!r}, found {token!r}"
            )
        self.position += 1
        return token

    def parse_expression(self, minimum_precedence: int) -> tuple[Any, ...]:
        left = self.parse_prefix()
        while True:
            operator = self.current()
            precedence = self.BINARY_PRECEDENCE.get(operator or "")
            if precedence is None or precedence < minimum_precedence:
                break
            self.consume()
            next_precedence = (
                precedence
                if operator in self.RIGHT_ASSOCIATIVE
                else precedence + 1
            )
            right = self.parse_expression(next_precedence)
            left = ("binary", operator, left, right)
        return left

    def parse_prefix(self) -> tuple[Any, ...]:
        token = self.current()
        if token in {"!", "?"}:
            return self.parse_quantifier()
        if token in {"~", "+", "-"}:
            self.consume()
            return ("unary", token, self.parse_prefix())
        if token == "(":
            self.consume("(")
            expression = self.parse_expression(0)
            self.consume(")")
            return expression
        if token is None or token in {")", "]", ",", ":"}:
            raise VerificationError(f"expected formula atom, found {token!r}")
        name = self.consume()
        if ADAPTER.VARIABLE_RE.match(name):
            canonical_name = self.bound_variables.get(name)
            if canonical_name is None:
                canonical_name = self.free_variables.setdefault(
                    name, f"FREE{len(self.free_variables) + 1}"
                )
            atom: tuple[Any, ...] = ("variable", canonical_name)
        else:
            atom = ("constant", name)
        if self.current() != "(":
            return atom
        self.consume("(")
        arguments = []
        if self.current() != ")":
            while True:
                arguments.append(self.parse_expression(0))
                if self.current() != ",":
                    break
                self.consume(",")
        self.consume(")")
        return ("application", atom, tuple(arguments))

    def parse_quantifier(self) -> tuple[Any, ...]:
        quantifier = self.consume()
        self.consume("[")
        binders: list[tuple[str, tuple[str, ...]]] = []
        previous: dict[str, str | None] = {}
        while True:
            name = self.consume()
            if not ADAPTER.VARIABLE_RE.match(name):
                raise VerificationError(
                    f"invalid quantified variable {name!r}"
                )
            self.consume(":")
            type_tokens = []
            depth = 0
            while True:
                token = self.current()
                if token is None:
                    raise VerificationError(
                        "unterminated quantified-variable type"
                    )
                if depth == 0 and token in {",", "]"}:
                    break
                token = self.consume()
                if token in {"(", "[", "{"}:
                    depth += 1
                elif token in {")", "]", "}"}:
                    depth -= 1
                type_tokens.append(token)
            if not type_tokens:
                raise VerificationError(
                    f"quantified variable {name!r} has no type"
                )
            self.binder_count += 1
            canonical_name = f"BOUND{self.binder_count}"
            previous[name] = self.bound_variables.get(name)
            self.bound_variables[name] = canonical_name
            binders.append((canonical_name, tuple(type_tokens)))
            if self.current() != ",":
                break
            self.consume(",")
        self.consume("]")
        self.consume(":")
        body = self.parse_expression(0)
        for name, prior in previous.items():
            if prior is None:
                del self.bound_variables[name]
            else:
                self.bound_variables[name] = prior
        return ("quantifier", quantifier, tuple(binders), body)


def formula_canonical(value: str) -> tuple[Any, ...]:
    """Canonicalize alpha-renaming and redundant formula parentheses."""

    return FormulaParser(value).parse()


def source_formulas(path: Path) -> list[object]:
    formulas = []
    for statement in ADAPTER.split_tptp_statements(
        path.read_text(encoding="utf-8")
    ):
        formula = ADAPTER.parse_annotated(statement)
        if formula is not None:
            formulas.append(formula)
    return formulas


def adapt_typed_sources(
    *,
    proof_text: str,
    proof_base: Path,
    controller_path: Path,
) -> tuple[str, str, dict[str, Any]]:
    """Create an alpha-audited TFF controller without changing proof logic."""

    parsed = []
    for statement in ADAPTER.split_tptp_statements(proof_text):
        formula = ADAPTER.parse_annotated(statement)
        if formula is None:
            raise VerificationError("proof contains a non-annotated statement")
        parsed.append(formula)
    if not parsed:
        raise VerificationError("proof block is empty")
    if len({formula.name for formula in parsed}) != len(parsed):
        raise VerificationError("proof formula names are not unique")

    source_cache: dict[Path, dict[str, object]] = {}
    controller_types: dict[tuple[str, str], object] = {}
    controller_premises: dict[str, object] = {}
    leaf_audits = []
    adapted = []
    quoted_controller = str(controller_path.resolve()).replace("\\", "\\\\")
    quoted_controller = quoted_controller.replace("'", "\\'")

    for formula in parsed:
        fields = list(formula.fields)
        file_source = (
            ADAPTER.parse_file_source(fields[3], proof_base)
            if len(fields) >= 4
            else None
        )
        if file_source is None:
            adapted.append(formula)
            continue
        if formula.kind != "tff":
            raise VerificationError(
                f"file-cited leaf {formula.name!r} is not TFF"
            )
        source_path, source_label = file_source
        resolved = source_path.resolve()
        if resolved not in source_cache:
            formulas = source_formulas(resolved)
            source_cache[resolved] = {
                source.name: source for source in formulas
            }
            for source in formulas:
                if source.kind == "tff" and source.role == "type":
                    controller_types[(source.name, source.body)] = source
        source = source_cache[resolved].get(source_label)
        if source is None:
            raise VerificationError(
                f"source label {source_label!r} is absent from {resolved}"
            )
        if source.kind != "tff":
            raise VerificationError(f"source {source_label!r} is not TFF")
        if source.role not in {"axiom", "conjecture"}:
            raise VerificationError(
                f"unsupported source role {source.role!r}"
            )
        if formula.role != source.role:
            raise VerificationError(
                f"leaf/source role mismatch for {source_label!r}"
            )
        proof_canonical = formula_canonical(formula.body)
        source_canonical = formula_canonical(source.body)
        if proof_canonical != source_canonical:
            raise VerificationError(
                f"leaf {formula.name!r} is not alpha-equivalent to "
                f"{source_label!r}"
            )
        premise = ADAPTER.AnnotatedFormula(
            "tff", (source.fields[0], source.fields[1], formula.body)
        )
        existing = controller_premises.get(source_label)
        if existing is not None and existing.fields != premise.fields:
            raise VerificationError(
                f"source label {source_label!r} has distinct proof spellings"
            )
        controller_premises[source_label] = premise
        fields[3] = f"file('{quoted_controller}',{source.fields[0]})"
        adapted.append(formula.with_fields(fields))
        leaf_audits.append(
            {
                "proof_name": formula.name,
                "source_label": source_label,
                "source_path": str(resolved),
                "source_role": source.role,
                "proof_body_sha256": ADAPTER.sha256_text(formula.body),
                "source_body_sha256": ADAPTER.sha256_text(source.body),
                "alpha_canonical_sha256": ADAPTER.sha256_text(
                    json.dumps(
                        source_canonical,
                        ensure_ascii=True,
                        separators=(",", ":"),
                    )
                ),
            }
        )

    if not leaf_audits:
        raise VerificationError("proof has no file-cited TFF leaves")
    if not any(
        premise.role == "conjecture"
        for premise in controller_premises.values()
    ):
        raise VerificationError("proof has no conjecture source leaf")
    controller = (
        "\n".join(
            formula.render()
            for formula in [
                *controller_types.values(),
                *controller_premises.values(),
            ]
        )
        + "\n"
    )
    prepared = "\n".join(formula.render() for formula in adapted) + "\n"
    report = {
        "schema_version": 1,
        "adapter": "proofcheck-typed-alpha-source-controller",
        "proof_formula_count": len(parsed),
        "input_leaf_count": len(leaf_audits),
        "controller_type_count": len(controller_types),
        "controller_premise_count": len(controller_premises),
        "changed_fields": ["input_leaf.file_source"],
        "logical_proof_fields_unchanged": True,
        "leaf_audits": leaf_audits,
        "original_proof_sha256": ADAPTER.sha256_text(proof_text),
        "prepared_proof_sha256": ADAPTER.sha256_text(prepared),
        "controller_sha256": ADAPTER.sha256_text(controller),
    }
    return prepared, controller, report


def write_typed_view(
    *,
    solution_path: Path,
    prepared_path: Path,
    controller_path: Path,
) -> dict[str, Any]:
    proof_text = ADAPTER.extract_proof_block(
        solution_path.read_text(encoding="utf-8", errors="strict")
    )
    prepared, controller, report = adapt_typed_sources(
        proof_text=proof_text,
        proof_base=solution_path.parent,
        controller_path=controller_path,
    )
    prepared_path.parent.mkdir(parents=True, exist_ok=True)
    controller_path.parent.mkdir(parents=True, exist_ok=True)
    prepared_path.write_text(prepared, encoding="utf-8", newline="\n")
    controller_path.write_text(controller, encoding="utf-8", newline="\n")
    return report


def claims(
    contract: dict[str, Any], results: Sequence[dict[str, Any]]
) -> list[dict[str, Any]]:
    baseline = analyze.reproducible_coverage(contract, results, "baseline")
    induction = analyze.reproducible_coverage(contract, results, "induction")
    unique = induction - baseline
    return sorted(
        [
            result
            for result in results
            if result["strategy"] == "induction"
            and result["problem_id"] in unique
            and result["szs_status"] in analyze.PROOF_STATUSES
        ],
        key=lambda value: (value["problem_id"], value["repetition"]),
    )


def verify_claims(
    *,
    repo: Path,
    experiment_root: Path,
    problem_root: Path,
    output_root: Path,
    proofcheck: Path,
) -> dict[str, Any]:
    contract, results = analyze.load_phase(experiment_root, "test")
    selected_claims = claims(contract, results)
    output_root.mkdir(parents=True, exist_ok=True)
    commands_dir = output_root / "commands"
    reports_dir = output_root / "reports"
    adapted_dir = output_root / "adapted"
    commands_dir.mkdir(exist_ok=True)
    reports_dir.mkdir(exist_ok=True)
    adapted_dir.mkdir(exist_ok=True)
    environment = os.environ.copy()
    environment["TPTP"] = str(problem_root / "problems" / "casc_2025")

    self_certify = BASE_VERIFY.run_command(
        [str(proofcheck), "-self-certify"],
        cwd=proofcheck.parent,
        timeout=300,
        environment=environment,
        stdout_path=commands_dir / "proofcheck-self-certify.stdout",
        stderr_path=commands_dir / "proofcheck-self-certify.stderr",
    )
    self_certify_text = (
        (commands_dir / "proofcheck-self-certify.stdout").read_text(
            encoding="utf-8", errors="replace"
        )
        + (commands_dir / "proofcheck-self-certify.stderr").read_text(
            encoding="utf-8", errors="replace"
        )
    )
    if self_certify.returncode != 0 or "117 passed" not in self_certify_text:
        raise VerificationError("ProofCheck did not pass all 117 self-tests")

    gate = repo / "tools" / "validation" / "validate_tptp_solution.py"
    cases = []
    for result in selected_claims:
        problem_id = result["problem_id"]
        repetition = result["repetition"]
        result_path = (
            experiment_root / "test" / result["_path"]
        ).resolve()
        solution_path = result_path.parent / "stdout.txt"
        augmented_path = Path(
            contract["materialized_inputs"][problem_id]["induction"]["path"]
        )
        source_path = (
            repo
            / "experiments"
            / "2026-07-29-022-integer-induction-schema"
            / "fixtures"
            / "test"
            / f"{problem_id}.p"
        )
        schema_report = verify_schema.verify_structure(
            source_path.read_text(encoding="utf-8"),
            augmented_path.read_text(encoding="utf-8"),
        )
        case_name = f"{problem_id}--rep-{repetition}"
        prepared_path = adapted_dir / f"{case_name}.proof.p"
        controller_path = adapted_dir / f"{case_name}.problem.p"
        adapter_report = write_typed_view(
            solution_path=solution_path,
            prepared_path=prepared_path,
            controller_path=controller_path,
        )
        adapter_report_path = reports_dir / f"{case_name}.adapter.json"
        adapter_report_path.write_bytes(
            analyze.canonical_json(adapter_report) + b"\n"
        )
        gate_report_path = reports_dir / f"{case_name}.gate.json"
        proof_command = [
            str(proofcheck),
            "-v",
            "-j",
            "2",
            "-t",
            "5",
            "-T",
            "120",
            "-p",
            str(controller_path),
            str(prepared_path),
        ]
        command = [
            sys.executable,
            str(gate),
            str(augmented_path),
            str(solution_path),
            "--report",
            str(gate_report_path),
            "--timeout-seconds",
            "120",
            "--proof-command-json",
            json.dumps(proof_command, separators=(",", ":")),
        ]
        completed = BASE_VERIFY.run_command(
            command,
            cwd=repo,
            timeout=180,
            environment=environment,
            stdout_path=commands_dir / f"{case_name}.stdout",
            stderr_path=commands_dir / f"{case_name}.stderr",
        )
        gate_report = json.loads(
            gate_report_path.read_text(encoding="utf-8")
        )
        verified = (
            completed.returncode == 0
            and gate_report["verdict"] == "verified"
        )
        cases.append(
            {
                "problem_id": problem_id,
                "repetition": repetition,
                "schema_id": schema_report["schema_id"],
                "solution_sha256": analyze.sha256_file(solution_path),
                "prepared_proof_sha256": analyze.sha256_file(prepared_path),
                "controller_sha256": analyze.sha256_file(controller_path),
                "adapter_report_sha256": analyze.sha256_file(
                    adapter_report_path
                ),
                "adapter_input_leaf_count": adapter_report["input_leaf_count"],
                "logical_proof_fields_unchanged": True,
                "gate_returncode": completed.returncode,
                "gate_verdict": gate_report["verdict"],
                "gate_reasons": gate_report["reasons"],
                "verified": verified,
            }
        )
        print(
            f"{len(cases)}/{len(selected_claims)}: "
            f"{problem_id}/rep-{repetition} -> {gate_report['verdict']}",
            flush=True,
        )

    verified_cases = sum(case["verified"] for case in cases)
    body = {
        "schema_version": 1,
        "test_contract_id": contract["contract_id"],
        "test_binary_sha256": contract["binary_sha256"],
        "proofcheck": {
            "tag": PROOFCHECK.PROOFCHECK_TAG,
            "release_archive_sha256": PROOFCHECK.PROOFCHECK_SHA256,
            "executable_sha256": analyze.sha256_file(proofcheck),
            "self_certify_returncode": self_certify.returncode,
        },
        "adapter": {
            "name": "proofcheck-typed-alpha-source-controller",
            "source_sha256": analyze.sha256_file(Path(__file__).resolve()),
            "logical_proof_fields_unchanged": True,
        },
        "expected_cases": len(selected_claims),
        "verified_cases": verified_cases,
        "all_verified": verified_cases == len(selected_claims),
        "cases": cases,
    }
    return {
        **body,
        "report_id": hashlib.sha256(analyze.canonical_json(body)).hexdigest(),
    }


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, default=REPO_ROOT)
    parser.add_argument("--experiment-root", type=Path, required=True)
    parser.add_argument("--problem-root", type=Path, required=True)
    parser.add_argument("--output-root", type=Path, required=True)
    parser.add_argument("--proofcheck", type=Path)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parse_args(argv)
    if sys.platform != "linux":
        raise VerificationError("independent proof checking requires Linux")
    output_root = arguments.output_root.resolve()
    proofcheck = BASE_VERIFY.find_or_download_proofcheck(
        output_root, arguments.proofcheck
    )
    report = verify_claims(
        repo=arguments.repo.resolve(),
        experiment_root=arguments.experiment_root.resolve(),
        problem_root=arguments.problem_root.resolve(),
        output_root=output_root,
        proofcheck=proofcheck,
    )
    report_path = output_root / "proof-validation.json"
    report_path.write_bytes(analyze.canonical_json(report) + b"\n")
    print(
        f"RESULT: {report['verified_cases']}/{report['expected_cases']} "
        f"proof claims verified; report {report['report_id']}"
    )
    return 0 if report["all_verified"] else 1


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (
        VerificationError,
        analyze.AnalysisError,
        OSError,
        ValueError,
        json.JSONDecodeError,
    ) as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(2) from error
