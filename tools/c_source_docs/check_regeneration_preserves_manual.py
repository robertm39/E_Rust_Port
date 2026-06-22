#!/usr/bin/env python3
"""Verify doc regeneration does not change manual-review sections."""

from __future__ import annotations

from pathlib import Path

import generate_c_source_docs as docs


def manual_sections() -> dict[str, str | None]:
    result: dict[str, str | None] = {}
    for path in sorted(docs.DOC_ROOT.rglob("*.md")):
        text = path.read_text(encoding="utf-8", errors="replace")
        begin = text.find(docs.MANUAL_BEGIN)
        end = text.find(docs.MANUAL_END)
        if begin >= 0 and end > begin:
            result[str(path.relative_to(docs.REPO_ROOT))] = text[begin : end + len(docs.MANUAL_END)]
        else:
            result[str(path.relative_to(docs.REPO_ROOT))] = None
    return result


def main() -> int:
    before = manual_sections()
    docs.generate_docs()
    after = manual_sections()
    changed = [path for path, value in before.items() if after.get(path) != value]
    if changed:
        for path in changed[:20]:
            print(f"ERROR: manual section changed during regeneration: {path}")
        if len(changed) > 20:
            print(f"ERROR: ... {len(changed) - 20} more")
        return 1
    print(f"OK: regeneration preserved manual sections in {len(before)} Markdown files.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
