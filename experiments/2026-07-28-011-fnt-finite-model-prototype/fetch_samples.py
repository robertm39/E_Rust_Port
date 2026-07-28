#!/usr/bin/env python3
"""Fetch and hash the preregistered public TPTP FNT sample problems."""

from __future__ import annotations

import argparse
import hashlib
import json
import urllib.request
from html.parser import HTMLParser
from pathlib import Path


PROBLEMS = (
    ("NLP042+1", "NLP"),
    ("SWV017+1", "SWV"),
    ("KRS173+1", "KRS"),
    ("MGT033+2", "MGT"),
)
URL_TEMPLATE = (
    "https://tptp.org/cgi-bin/SeeTPTP?"
    "Category=Problems&Domain={domain}&File={problem}.p"
)


class FirstPreformattedBlock(HTMLParser):
    """Extract the first preformatted block while decoding HTML entities."""

    def __init__(self) -> None:
        super().__init__(convert_charrefs=True)
        self.in_pre = False
        self.finished = False
        self.parts: list[str] = []

    def handle_starttag(
        self, tag: str, attrs: list[tuple[str, str | None]]
    ) -> None:
        del attrs
        if tag.lower() == "pre" and not self.finished:
            self.in_pre = True

    def handle_endtag(self, tag: str) -> None:
        if tag.lower() == "pre" and self.in_pre:
            self.in_pre = False
            self.finished = True

    def handle_data(self, data: str) -> None:
        if self.in_pre:
            self.parts.append(data)


def fetch(url: str) -> str:
    request = urllib.request.Request(
        url,
        headers={"User-Agent": "Umlaut finite-model experiment"},
    )
    with urllib.request.urlopen(request, timeout=60.0) as response:
        page = response.read().decode("utf-8")
    parser = FirstPreformattedBlock()
    parser.feed(page)
    text = "".join(parser.parts).strip() + "\n"
    if not parser.finished or "% File" not in text:
        raise ValueError(f"response does not contain a TPTP problem: {url}")
    return text


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)

    records: list[dict[str, object]] = []
    for problem, domain in PROBLEMS:
        url = URL_TEMPLATE.format(domain=domain, problem=problem)
        text = fetch(url)
        path = args.output / f"{problem}.p"
        path.write_text(text, encoding="utf-8", newline="\n")
        data = path.read_bytes()
        records.append(
            {
                "schema_version": 1,
                "problem_id": problem,
                "category": "FNT-sample",
                "family": domain,
                "split": "calibration",
                "path": path.name,
                "bytes": len(data),
                "sha256": hashlib.sha256(data).hexdigest(),
                "source": "TPTP online problem access",
                "source_url": url,
            }
        )

    manifest = args.output / "manifest.jsonl"
    manifest.write_text(
        "".join(json.dumps(record, sort_keys=True) + "\n" for record in records),
        encoding="utf-8",
        newline="\n",
    )
    print(json.dumps(records, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
