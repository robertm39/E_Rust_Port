#!/usr/bin/env python3
"""Gate TPTP solution claims through independent, positive-only validation."""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Sequence


SZS_STATUS_RE = re.compile(
    r"^[%#]\s*SZS\s+status\s+([A-Za-z][A-Za-z0-9_-]*)\b",
    re.MULTILINE | re.IGNORECASE,
)
PROBLEM_STATUS_RE = re.compile(
    r"^[%#]\s*Status\s*:\s*([A-Za-z][A-Za-z0-9_-]*)\b",
    re.MULTILINE | re.IGNORECASE,
)
OUTPUT_START_RE = re.compile(
    r"^[%#]\s*SZS\s+output\s+start\s+([A-Za-z][A-Za-z0-9_-]*)\b",
    re.IGNORECASE,
)
OUTPUT_END_RE = re.compile(
    r"^[%#]\s*SZS\s+output\s+end\s+([A-Za-z][A-Za-z0-9_-]*)\b",
    re.IGNORECASE,
)
ANNOTATED_FORMULA_RE = re.compile(
    r"^\s*(?:cnf|fof|tff|tcf|thf)\s*\(",
    re.MULTILINE | re.IGNORECASE,
)
FALSE_FORMULA_RE = re.compile(r"(?<![A-Za-z0-9_$])\$false(?![A-Za-z0-9_$])")

PROOF_STATUSES = frozenset({"theorem", "unsatisfiable", "contradictoryaxioms"})
MODEL_STATUSES = frozenset({"satisfiable", "countersatisfiable"})
NO_CLAIM_STATUSES = frozenset(
    {
        "gaveup",
        "inappropriate",
        "inputerror",
        "memoryout",
        "nosuccess",
        "outputerror",
        "resourceout",
        "timeout",
        "unknown",
    }
)
PROOF_OUTPUT_TYPES = frozenset({"cnfrefutation", "refutation", "proof"})
MODEL_OUTPUT_TYPES = frozenset(
    {"domaininterpretation", "finitemodel", "interpretation", "model"}
)
ALLOWED_STATUS_BY_EXPECTED = {
    "theorem": frozenset({"theorem", "contradictoryaxioms"}),
    "unsatisfiable": frozenset({"unsatisfiable", "contradictoryaxioms"}),
    "contradictoryaxioms": frozenset({"contradictoryaxioms"}),
    "countersatisfiable": frozenset({"countersatisfiable"}),
    "satisfiable": frozenset({"satisfiable"}),
}
VERIFIED_GOOD = "verifiedgood"
VERIFIED_BAD = "verifiedbad"
INCONCLUSIVE_CHECKER_STATUSES = frozenset({"unknown", "timeout"})

EXIT_VERIFIED = 0
EXIT_REJECTED = 1
EXIT_COVERAGE_GAP = 2
EXIT_ERROR = 3


class ValidationError(RuntimeError):
    """Raised for malformed input or invalid validation configuration."""


@dataclass(frozen=True)
class OutputBlock:
    """One SZS output block extracted from a solution."""

    output_type: str
    start_line: int
    end_line: int
    body: str


@dataclass(frozen=True)
class CommandResult:
    """Captured result of one external validation command."""

    command: tuple[str, ...]
    returncode: int
    stdout: str
    stderr: str


def normalized_status(value: str) -> str:
    """Normalize an SZS token for comparisons while preserving report spelling."""

    return value.replace("_", "").replace("-", "").lower()


def read_text(path: Path) -> str:
    """Read one UTF-8 TPTP artifact with an actionable error."""

    try:
        return path.read_text(encoding="utf-8")
    except (OSError, UnicodeError) as error:
        raise ValidationError(f"cannot read {path}: {error}") from error


def parse_problem_status(problem_text: str) -> str | None:
    """Return the declared TPTP problem status, if present."""

    match = PROBLEM_STATUS_RE.search(problem_text)
    return match.group(1) if match else None


def parse_solution_statuses(solution_text: str) -> list[str]:
    """Return all SZS status tokens in output order."""

    return SZS_STATUS_RE.findall(solution_text)


def extract_output_blocks(solution_text: str) -> list[OutputBlock]:
    """Extract non-nested, type-matched SZS output blocks."""

    blocks: list[OutputBlock] = []
    active_type: str | None = None
    active_line = 0
    active_body: list[str] = []

    for line_number, line in enumerate(solution_text.splitlines(keepends=True), 1):
        start = OUTPUT_START_RE.match(line)
        end = OUTPUT_END_RE.match(line)
        if start:
            if active_type is not None:
                raise ValidationError(
                    f"nested SZS output block at line {line_number}"
                )
            active_type = start.group(1)
            active_line = line_number
            active_body = []
            continue
        if end:
            if active_type is None:
                raise ValidationError(
                    f"SZS output end without start at line {line_number}"
                )
            if normalized_status(end.group(1)) != normalized_status(active_type):
                raise ValidationError(
                    "SZS output block type mismatch at line "
                    f"{line_number}: started {active_type}, ended {end.group(1)}"
                )
            blocks.append(
                OutputBlock(
                    output_type=active_type,
                    start_line=active_line,
                    end_line=line_number,
                    body="".join(active_body),
                )
            )
            active_type = None
            active_body = []
            continue
        if active_type is not None:
            active_body.append(line)

    if active_type is not None:
        raise ValidationError(
            f"unterminated SZS output block {active_type} from line {active_line}"
        )
    return blocks


def parse_command_json(raw: str | None, option_name: str) -> list[str] | None:
    """Parse a shell-free command vector from one JSON CLI value."""

    if raw is None:
        return None
    try:
        parsed = json.loads(raw)
    except json.JSONDecodeError as error:
        raise ValidationError(f"{option_name} is not valid JSON: {error}") from error
    if (
        not isinstance(parsed, list)
        or not parsed
        or any(not isinstance(item, str) or not item for item in parsed)
    ):
        raise ValidationError(f"{option_name} must be a non-empty JSON string array")
    return parsed


def expand_command(command: Sequence[str], values: dict[str, str]) -> list[str]:
    """Expand the documented placeholders in a command vector."""

    expanded: list[str] = []
    for argument in command:
        value = argument
        for name, replacement in values.items():
            value = value.replace("{" + name + "}", replacement)
        expanded.append(value)
    return expanded


def run_external_command(
    command: Sequence[str],
    *,
    values: dict[str, str],
    timeout_seconds: float,
) -> CommandResult:
    """Run a checker without a shell and capture both output streams."""

    expanded = expand_command(command, values)
    try:
        completed = subprocess.run(
            expanded,
            check=False,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=timeout_seconds,
        )
    except (OSError, subprocess.SubprocessError) as error:
        raise ValidationError(
            f"external command failed to execute: {expanded[0]}: {error}"
        ) from error
    return CommandResult(
        command=tuple(expanded),
        returncode=completed.returncode,
        stdout=completed.stdout,
        stderr=completed.stderr,
    )


def command_report(name: str, result: CommandResult) -> dict[str, Any]:
    """Return bounded, reproducible command evidence for the JSON report."""

    limit = 16_384
    return {
        "name": name,
        "command": list(result.command),
        "returncode": result.returncode,
        "stdout": result.stdout[-limit:],
        "stderr": result.stderr[-limit:],
    }


def checker_verdict(result: CommandResult) -> str | None:
    """Return the last SZS verdict reported by an external checker."""

    statuses = parse_solution_statuses(result.stdout + "\n" + result.stderr)
    return normalized_status(statuses[-1]) if statuses else None


def classify_claim(status: str) -> str:
    """Classify one normalized final SZS status."""

    if status in PROOF_STATUSES:
        return "proof"
    if status in MODEL_STATUSES:
        return "model"
    if status in NO_CLAIM_STATUSES:
        return "no_claim"
    return "unsupported"


def status_consistency_error(
    expected_status: str | None, claimed_status: str
) -> str | None:
    """Return a message when a success claim contradicts a declared status."""

    if expected_status is None:
        return None
    expected = normalized_status(expected_status)
    allowed = ALLOWED_STATUS_BY_EXPECTED.get(expected)
    if allowed is None or claimed_status not in PROOF_STATUSES | MODEL_STATUSES:
        return None
    if claimed_status not in allowed:
        return (
            f"claimed SZS status {claimed_status} is incompatible with "
            f"problem status {expected}"
        )
    return None


def matching_blocks(
    blocks: Sequence[OutputBlock], allowed_types: frozenset[str]
) -> list[OutputBlock]:
    """Return output blocks whose normalized type is in ``allowed_types``."""

    return [
        block
        for block in blocks
        if normalized_status(block.output_type) in allowed_types
    ]


def initial_report(
    problem: Path,
    solution: Path,
    expected_status: str | None,
    statuses: Sequence[str],
    blocks: Sequence[OutputBlock],
) -> dict[str, Any]:
    """Create the stable top-level report fields."""

    return {
        "schema_version": 1,
        "problem": str(problem.resolve()),
        "solution": str(solution.resolve()),
        "problem_status": expected_status,
        "solution_statuses": list(statuses),
        "claimed_status": statuses[-1] if statuses else None,
        "claim_kind": None,
        "output_blocks": [
            {
                "type": block.output_type,
                "start_line": block.start_line,
                "end_line": block.end_line,
                "bytes": len(block.body.encode("utf-8")),
            }
            for block in blocks
        ],
        "checks": [],
        "verdict": None,
        "reasons": [],
    }


def finish(
    report: dict[str, Any], verdict: str, *reasons: str
) -> tuple[dict[str, Any], int]:
    """Finalize one report and map its verdict to a stable exit code."""

    report["verdict"] = verdict
    report["reasons"].extend(reasons)
    exit_code = {
        "verified": EXIT_VERIFIED,
        "not_applicable": EXIT_VERIFIED,
        "rejected": EXIT_REJECTED,
        "coverage_gap": EXIT_COVERAGE_GAP,
        "error": EXIT_ERROR,
    }[verdict]
    return report, exit_code


def validate_artifact_with_checker(
    *,
    report: dict[str, Any],
    checker_name: str,
    command: Sequence[str] | None,
    problem: Path,
    solution: Path,
    artifact: Path,
    timeout_seconds: float,
) -> tuple[str, str]:
    """Run a positive-only semantic checker and return verdict plus reason."""

    if command is None:
        return "coverage_gap", f"{checker_name} is not configured"
    result = run_external_command(
        command,
        values={
            "problem": str(problem.resolve()),
            "solution": str(solution.resolve()),
            "artifact": str(artifact.resolve()),
        },
        timeout_seconds=timeout_seconds,
    )
    report["checks"].append(command_report(checker_name, result))
    verdict = checker_verdict(result)
    if verdict == VERIFIED_GOOD and result.returncode == 0:
        return "verified", f"{checker_name} reported VerifiedGood"
    if verdict == VERIFIED_BAD:
        return "rejected", f"{checker_name} reported VerifiedBad"
    if verdict in INCONCLUSIVE_CHECKER_STATUSES:
        return "coverage_gap", f"{checker_name} reported {verdict}"
    if verdict == VERIFIED_GOOD:
        return (
            "rejected",
            f"{checker_name} reported VerifiedGood but exited {result.returncode}",
        )
    return (
        "rejected",
        f"{checker_name} did not emit a recognized positive verdict "
        f"(exit {result.returncode})",
    )


def validate_solution(
    *,
    problem: Path,
    solution: Path,
    proof_command: Sequence[str] | None,
    model_command: Sequence[str] | None,
    syntax_command: Sequence[str] | None,
    timeout_seconds: float,
) -> tuple[dict[str, Any], int]:
    """Validate one solver output and return its report and process exit code."""

    problem_text = read_text(problem)
    solution_text = read_text(solution)
    expected_status = parse_problem_status(problem_text)
    statuses = parse_solution_statuses(solution_text)
    blocks = extract_output_blocks(solution_text)
    report = initial_report(problem, solution, expected_status, statuses, blocks)

    if not statuses:
        return finish(report, "rejected", "solution has no SZS status")
    claimed_status = normalized_status(statuses[-1])
    claim_kind = classify_claim(claimed_status)
    report["claim_kind"] = claim_kind

    inconsistency = status_consistency_error(expected_status, claimed_status)
    if inconsistency is not None:
        report["checks"].append(
            {"name": "problem_status_consistency", "outcome": "fail"}
        )
        return finish(report, "rejected", inconsistency)
    report["checks"].append(
        {"name": "problem_status_consistency", "outcome": "pass"}
    )

    if claim_kind == "no_claim":
        return finish(
            report,
            "not_applicable",
            "final SZS status makes no proof or model success claim",
        )
    if claim_kind == "unsupported":
        return finish(
            report,
            "coverage_gap",
            f"unsupported final SZS status: {statuses[-1]}",
        )

    allowed_types = PROOF_OUTPUT_TYPES if claim_kind == "proof" else MODEL_OUTPUT_TYPES
    claim_blocks = matching_blocks(blocks, allowed_types)
    if len(claim_blocks) != 1:
        noun = "proof" if claim_kind == "proof" else "interpretation"
        return finish(
            report,
            "coverage_gap" if not claim_blocks else "rejected",
            f"expected exactly one {noun} output block, found {len(claim_blocks)}",
        )
    artifact = claim_blocks[0]
    if not artifact.body.strip():
        return finish(report, "rejected", "SZS output block is empty")
    if not ANNOTATED_FORMULA_RE.search(artifact.body):
        return finish(
            report, "rejected", "SZS output block has no annotated formula"
        )
    if claim_kind == "proof" and not FALSE_FORMULA_RE.search(artifact.body):
        return finish(
            report, "rejected", "refutation output block has no $false formula"
        )
    report["checks"].append({"name": "output_structure", "outcome": "pass"})

    with tempfile.TemporaryDirectory(prefix="umlaut-validation-") as temporary:
        suffix = ".proof.p" if claim_kind == "proof" else ".model.p"
        artifact_path = Path(temporary) / f"artifact{suffix}"
        artifact_path.write_text(artifact.body, encoding="utf-8", newline="\n")
        values = {
            "problem": str(problem.resolve()),
            "solution": str(solution.resolve()),
            "artifact": str(artifact_path.resolve()),
        }

        if syntax_command is not None:
            syntax_result = run_external_command(
                syntax_command,
                values=values,
                timeout_seconds=timeout_seconds,
            )
            report["checks"].append(command_report("syntax_checker", syntax_result))
            if syntax_result.returncode != 0:
                return finish(
                    report,
                    "rejected",
                    f"syntax checker exited {syntax_result.returncode}",
                )

        checker_name = (
            "external_proof_checker"
            if claim_kind == "proof"
            else "external_model_checker"
        )
        command = proof_command if claim_kind == "proof" else model_command
        verdict, reason = validate_artifact_with_checker(
            report=report,
            checker_name=checker_name,
            command=command,
            problem=problem,
            solution=solution,
            artifact=artifact_path,
            timeout_seconds=timeout_seconds,
        )
        return finish(report, verdict, reason)


def build_parser() -> argparse.ArgumentParser:
    """Build the command-line parser."""

    parser = argparse.ArgumentParser(
        description=(
            "Validate the final SZS claim in a TPTP solver output. External "
            "semantic checkers must report SZS VerifiedGood to be accepted."
        )
    )
    parser.add_argument("problem", type=Path, help="original TPTP problem")
    parser.add_argument("solution", type=Path, help="solver output to validate")
    parser.add_argument(
        "--proof-command-json",
        help=(
            "JSON command vector for proof validation; placeholders are "
            "{problem}, {solution}, and the extracted {artifact}"
        ),
    )
    parser.add_argument(
        "--model-command-json",
        help=(
            "JSON command vector for model validation; placeholders are "
            "{problem}, {solution}, and the extracted {artifact}"
        ),
    )
    parser.add_argument(
        "--syntax-command-json",
        help=(
            "optional JSON command vector for syntax validation; exit zero "
            "means syntactically accepted"
        ),
    )
    parser.add_argument(
        "--timeout-seconds",
        type=float,
        default=300.0,
        help="wall timeout for each external command (default: 300)",
    )
    parser.add_argument(
        "--report",
        type=Path,
        help="write the JSON report here instead of stdout",
    )
    parser.add_argument(
        "--allow-coverage-gap",
        action="store_true",
        help="return zero for an explicit coverage_gap verdict",
    )
    return parser


def write_report(report: dict[str, Any], path: Path | None) -> None:
    """Write one deterministic JSON report."""

    rendered = json.dumps(report, indent=2, sort_keys=True) + "\n"
    if path is None:
        sys.stdout.write(rendered)
        return
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(rendered, encoding="utf-8", newline="\n")


def main(argv: Sequence[str] | None = None) -> int:
    """Run the command-line gate."""

    parser = build_parser()
    args = parser.parse_args(argv)
    if args.timeout_seconds <= 0:
        parser.error("--timeout-seconds must be positive")
    try:
        proof_command = parse_command_json(
            args.proof_command_json, "--proof-command-json"
        )
        model_command = parse_command_json(
            args.model_command_json, "--model-command-json"
        )
        syntax_command = parse_command_json(
            args.syntax_command_json, "--syntax-command-json"
        )
        report, exit_code = validate_solution(
            problem=args.problem,
            solution=args.solution,
            proof_command=proof_command,
            model_command=model_command,
            syntax_command=syntax_command,
            timeout_seconds=args.timeout_seconds,
        )
    except ValidationError as error:
        report = {
            "schema_version": 1,
            "problem": str(args.problem.resolve()),
            "solution": str(args.solution.resolve()),
            "verdict": "error",
            "reasons": [str(error)],
        }
        exit_code = EXIT_ERROR
    write_report(report, args.report)
    if args.allow_coverage_gap and exit_code == EXIT_COVERAGE_GAP:
        return EXIT_VERIFIED
    return exit_code


if __name__ == "__main__":
    raise SystemExit(main())
