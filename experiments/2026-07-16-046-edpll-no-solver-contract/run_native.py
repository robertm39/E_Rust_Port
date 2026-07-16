"""Pin the reference edpll trace-only behavior on contradictory units."""

from __future__ import annotations

import subprocess
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
EXPECTED_STDOUT = "New clause: p<-....accepted\nNew clause: <-p....accepted\n"


def main() -> None:
    binary = REPO_ROOT / "target" / "release" / "edpll.exe"
    result = subprocess.run(
        [binary],
        input="p.\n<- p.\n",
        text=True,
        capture_output=True,
        check=False,
        timeout=30,
    )
    if result.returncode != 0:
        raise AssertionError(f"unexpected exit status: {result.returncode}")
    if result.stdout != EXPECTED_STDOUT:
        raise AssertionError(f"unexpected stdout: {result.stdout!r}")
    if result.stderr:
        raise AssertionError(f"unexpected stderr: {result.stderr!r}")
    print("PASS contradictory units: trace only, exit 0, no SAT result")


if __name__ == "__main__":
    main()
