#!/usr/bin/env python3
"""Check C-source docs use the standard Change Later section wording."""

from __future__ import annotations

import sys
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
DOC_ROOT = REPO_ROOT / "docs" / "c_source_docs"

LEGACY_TEXT = (
    "C Behaviors To Revisit After Compatibility",
    "Cleanup Candidates",
    "Compatibility Candidates",
    "Change-Later",
    "Change-later",
    "Change Later Candidates",
    "Change later candidate",
    "Change later candidates",
    "Change later:",
)


def iter_markdown() -> list[Path]:
    return [REPO_ROOT / "DOCS.md", *sorted(DOC_ROOT.rglob("*.md"))]


def main() -> int:
    errors: list[str] = []
    for path in iter_markdown():
        text = path.read_text(encoding="utf-8", errors="replace")
        rel_path = path.relative_to(REPO_ROOT)
        for line_number, line in enumerate(text.splitlines(), start=1):
            stripped = line.strip()
            for legacy in LEGACY_TEXT:
                if legacy in line:
                    errors.append(f"{rel_path}:{line_number}: legacy wording: {legacy}")
            if path.is_relative_to(DOC_ROOT):
                if stripped.startswith("### Change") and stripped != "### Change Later":
                    errors.append(
                        f"{rel_path}:{line_number}: use exactly '### Change Later'"
                    )
    if errors:
        for error in errors:
            print(f"ERROR: {error}", file=sys.stderr)
        return 1

    print(f"OK: checked Change Later wording in {len(iter_markdown())} Markdown files.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
