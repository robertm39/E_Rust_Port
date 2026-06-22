#!/usr/bin/env python3
"""Check local Markdown links in docs/c_source_docs."""

from __future__ import annotations

import re
import sys
from pathlib import Path
from urllib.parse import unquote, urlparse


REPO_ROOT = Path(__file__).resolve().parents[2]
DOC_ROOT = REPO_ROOT / "docs" / "c_source_docs"
LINK_RE = re.compile(r"(?<!!)\[[^\]]+\]\(([^)]+)\)")


def is_external(target: str) -> bool:
    parsed = urlparse(target)
    return bool(parsed.scheme and parsed.scheme not in {"", "file"})


def main() -> int:
    errors: list[str] = []
    for md_path in sorted(DOC_ROOT.rglob("*.md")):
        text = md_path.read_text(encoding="utf-8", errors="replace")
        for match in LINK_RE.finditer(text):
            target = match.group(1).strip()
            if not target or is_external(target) or target.startswith("#"):
                continue
            target = target.split("#", 1)[0]
            if not target:
                continue
            target_path = (md_path.parent / unquote(target)).resolve()
            try:
                target_path.relative_to(REPO_ROOT)
            except ValueError:
                errors.append(f"{md_path.relative_to(REPO_ROOT)} links outside repo: {target}")
                continue
            if not target_path.exists():
                errors.append(f"{md_path.relative_to(REPO_ROOT)} has broken link: {target}")
    if errors:
        for error in errors:
            print(f"ERROR: {error}", file=sys.stderr)
        return 1
    print(f"OK: checked local links in {len(list(DOC_ROOT.rglob('*.md')))} Markdown files.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
