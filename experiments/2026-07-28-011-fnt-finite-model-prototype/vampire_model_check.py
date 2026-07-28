#!/usr/bin/env python3
"""Adapt a TPTP finite interpretation to Vampire's semantic model checker."""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
import tempfile
from pathlib import Path


def split_top_level(text: str) -> list[str]:
    parts: list[str] = []
    start = 0
    depth = 0
    quote: str | None = None
    escaped = False
    for index, char in enumerate(text):
        if quote is not None:
            if escaped:
                escaped = False
            elif char == "\\":
                escaped = True
            elif char == quote:
                quote = None
            continue
        if char in {"'", '"'}:
            quote = char
        elif char in "([":
            depth += 1
        elif char in ")]":
            depth -= 1
        elif char == "," and depth == 0:
            parts.append(text[start:index].strip())
            start = index + 1
    parts.append(text[start:].strip())
    return parts


def statements(text: str) -> list[str]:
    result: list[str] = []
    start = 0
    depth = 0
    quote: str | None = None
    escaped = False
    for index, char in enumerate(text):
        if quote is not None:
            if escaped:
                escaped = False
            elif char == "\\":
                escaped = True
            elif char == quote:
                quote = None
            continue
        if char in {"'", '"'}:
            quote = char
        elif char in "([":
            depth += 1
        elif char in ")]":
            depth -= 1
        elif char == "." and depth == 0:
            item = text[start : index + 1].strip()
            if item:
                result.append(item)
            start = index + 1
    if text[start:].strip():
        raise ValueError("unterminated TPTP statement")
    return result


def semantic_problem(text: str) -> str:
    """Turn conjectures into negated axioms for direct model evaluation."""

    comments = "\n".join(
        line for line in text.splitlines() if line.lstrip().startswith("%")
    )
    logical_text = "\n".join(
        line for line in text.splitlines() if not line.lstrip().startswith("%")
    )
    output: list[str] = [comments] if comments else []
    for statement in statements(logical_text):
        match = re.match(r"(?is)^\s*(fof|cnf)\s*\((.*)\)\s*\.\s*$", statement)
        if match is None:
            output.append(statement)
            continue
        language, content = match.groups()
        fields = split_top_level(content)
        if len(fields) < 3:
            raise ValueError(f"malformed annotated formula: {statement[:100]}")
        role = fields[1].strip().lower()
        if role == "conjecture":
            output.append(f"fof({fields[0]},axiom,~({fields[2]})).")
        elif role == "negated_conjecture":
            output.append(f"{language}({fields[0]},axiom,{fields[2]}).")
        else:
            output.append(statement)
    return "\n".join(output) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--vampire", type=Path, required=True)
    parser.add_argument("--timeout-seconds", type=float, default=120.0)
    parser.add_argument("problem", type=Path)
    parser.add_argument("artifact", type=Path)
    args = parser.parse_args()

    try:
        problem = semantic_problem(args.problem.read_text(encoding="utf-8"))
        model = args.artifact.read_text(encoding="utf-8")
    except (OSError, UnicodeError, ValueError) as error:
        print(f"% SZS status VerifiedBad\n% {error}")
        return 1

    wrapper = (
        "vampire(model_check,formulas_start).\n"
        + problem
        + "vampire(model_check,formulas_end).\n"
        + "vampire(model_check,model_start).\n"
        + model
        + "vampire(model_check,model_end).\n"
    )
    try:
        with tempfile.NamedTemporaryFile(
            mode="w",
            encoding="utf-8",
            suffix=".p",
            prefix=".umlaut-model-check-",
            dir=args.problem.resolve().parent,
            delete=False,
        ) as temporary:
            temporary.write(wrapper)
            wrapper_path = Path(temporary.name)
        try:
            completed = subprocess.run(
                [str(args.vampire), "--mode", "model_check", str(wrapper_path)],
                check=False,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                timeout=args.timeout_seconds,
            )
        finally:
            wrapper_path.unlink(missing_ok=True)
    except (OSError, subprocess.SubprocessError) as error:
        print(f"% SZS status VerifiedBad\n% checker execution failed: {error}")
        return 1

    output = completed.stdout + "\n" + completed.stderr
    # Vampire's model_check mode returns its "success" process code 1 even
    # after the explicit all-true summary; the semantic summary is the gate.
    if (
        completed.returncode in {0, 1}
        and "All formulas evaluated to True!" in output
        and "There was a false formula!" not in output
    ):
        print("% SZS status VerifiedGood")
        return 0
    print("% SZS status VerifiedBad")
    print(f"% Vampire exit code: {completed.returncode}")
    for line in output.splitlines()[-20:]:
        print(f"% {line}")
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
