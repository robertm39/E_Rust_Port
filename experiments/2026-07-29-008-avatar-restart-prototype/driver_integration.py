#!/usr/bin/env python3
"""Exercise the persistent SAT-driver protocol and independently check replies."""

from __future__ import annotations

import argparse
import json
import subprocess
from pathlib import Path
from typing import Any


class DriverError(RuntimeError):
    """The driver transcript violates the experiment contract."""


def parse_model(fields: list[str]) -> list[int]:
    if len(fields) < 3 or fields[0] != "sat" or fields[-1] != "0":
        raise DriverError(f"expected SAT model, received {fields}")
    return [int(field) for field in fields[2:-1]]


def satisfies(clause: list[int], model: list[int]) -> bool:
    return any(literal in model for literal in clause)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--driver", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    return parser.parse_args()


def main() -> None:
    arguments = parse_args()
    process = subprocess.Popen(
        [str(arguments.driver.resolve())],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        bufsize=1,
    )
    transcript: list[dict[str, Any]] = []

    def read() -> list[str]:
        assert process.stdout is not None
        line = process.stdout.readline()
        if not line:
            raise DriverError("driver ended before replying")
        return line.split()

    def request(command: str) -> list[str]:
        assert process.stdin is not None
        process.stdin.write(command + "\n")
        process.stdin.flush()
        reply = read()
        transcript.append({"command": command, "reply": reply})
        return reply

    ready = read()
    if ready[:2] != ["ready", "1"]:
        raise DriverError(f"invalid ready response: {ready}")
    transcript.append({"command": None, "reply": ready})
    if request("a 1 2 0") != ["ok", "1"]:
        raise DriverError("initial clause was not retained")
    first = parse_model(request("s"))
    if set(map(abs, first)) != {1, 2} or not satisfies([1, 2], first):
        raise DriverError("first model is incomplete or invalid")
    first_active = [literal for literal in first if literal > 0]
    if not first_active:
        raise DriverError("first model activates no component")
    if request("a " + " ".join(str(-item) for item in first_active) + " 0")[
        0
    ] != "ok":
        raise DriverError("first conflict was not retained")
    second = parse_model(request("s"))
    if (
        set(map(abs, second)) != {1, 2}
        or not satisfies([1, 2], second)
        or all(literal in second for literal in first_active)
    ):
        raise DriverError("second model ignores the learned conflict")
    second_active = [literal for literal in second if literal > 0]
    if request(
        "a " + " ".join(str(-item) for item in second_active) + " 0"
    )[0] != "ok":
        raise DriverError("second conflict was not retained")
    final = request("s")
    if not final or final[0] != "unsat":
        raise DriverError(f"expected incremental UNSAT, received {final}")
    if request("q") != ["bye"]:
        raise DriverError("driver did not close cleanly")
    process.wait(timeout=5)
    report = {
        "schema_version": 1,
        "passed": True,
        "complete_models": [first, second],
        "transcript": transcript,
    }
    arguments.output.parent.mkdir(parents=True, exist_ok=True)
    arguments.output.write_text(
        json.dumps(report, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    print(json.dumps(report, sort_keys=True))


if __name__ == "__main__":
    try:
        main()
    except (DriverError, OSError, ValueError) as error:
        print(f"driver integration error: {error}")
        raise SystemExit(1) from error
