#!/usr/bin/env python3
"""Independently verify every reproducible larger-budget AC proof claim."""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path


BASE_PATH = (
    Path(__file__).resolve().parent.parent
    / "2026-07-28-007-unit-equality-completion"
    / "verify.py"
)
SPEC = importlib.util.spec_from_file_location("ac_mode_proof_verifier", BASE_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError(f"cannot load proof verifier: {BASE_PATH}")
BASE = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = BASE
SPEC.loader.exec_module(BASE)


if __name__ == "__main__":
    raise SystemExit(BASE.main())
