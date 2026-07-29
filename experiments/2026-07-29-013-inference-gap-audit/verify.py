#!/usr/bin/env python3
"""Run the established first-order proof checks on the audit phase."""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path
from types import ModuleType
from typing import Sequence


EXPERIMENT_ROOT = Path(__file__).resolve().parent
PRIOR_VERIFY_PATH = (
    EXPERIMENT_ROOT.parent
    / "2026-07-28-008-stronger-redundancy"
    / "verify.py"
)


def load_module(name: str, path: Path) -> ModuleType:
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load module: {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


PRIOR = load_module("inference_gap_prior_verify", PRIOR_VERIFY_PATH)
_load_phase = PRIOR.ANALYZE.load_phase


def load_audit_phase(experiment_root: Path, phase: str):
    if phase != "test":
        raise PRIOR.ANALYZE.AnalysisError(
            f"proof wrapper expected the prior test alias, got {phase}"
        )
    return _load_phase(experiment_root, "audit")


def main(argv: Sequence[str] | None = None) -> int:
    PRIOR.ANALYZE.load_phase = load_audit_phase
    return PRIOR.main(argv)


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (
        PRIOR.VerificationError,
        PRIOR.ANALYZE.AnalysisError,
        OSError,
        ValueError,
        RuntimeError,
    ) as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(2) from error

