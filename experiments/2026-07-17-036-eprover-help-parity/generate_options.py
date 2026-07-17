"""Regenerate the Rust eprover option table's order and prose from C."""

from __future__ import annotations

import argparse
import ast
import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
C_OPTIONS = ROOT / "eprover" / "PROVER" / "e_options.h"
RUST_OPTIONS = ROOT / "src" / "prover" / "options.rs"


def split_fields(entry: str) -> list[str]:
    fields: list[str] = []
    start = 0
    in_string = False
    in_char = False
    escaped = False
    nesting = 0

    for index, character in enumerate(entry):
        if escaped:
            escaped = False
            continue
        if in_string:
            if character == "\\":
                escaped = True
            elif character == '"':
                in_string = False
            continue
        if in_char:
            if character == "\\":
                escaped = True
            elif character == "'":
                in_char = False
            continue

        if character == '"':
            in_string = True
        elif character == "'":
            in_char = True
        elif character in "([{":
            nesting += 1
        elif character in ")]}":
            nesting -= 1
        elif character == "," and nesting == 0:
            fields.append(entry[start:index].strip())
            start = index + 1

    fields.append(entry[start:].strip())
    return fields


def balanced_items(text: str, open_character: str, close_character: str) -> list[str]:
    items: list[str] = []
    start: int | None = None
    depth = 0
    in_string = False
    in_char = False
    escaped = False

    for index, character in enumerate(text):
        if escaped:
            escaped = False
            continue
        if in_string:
            if character == "\\":
                escaped = True
            elif character == '"':
                in_string = False
            continue
        if in_char:
            if character == "\\":
                escaped = True
            elif character == "'":
                in_char = False
            continue

        if character == '"':
            in_string = True
        elif character == "'":
            in_char = True
        elif character == open_character:
            if depth == 0:
                start = index + 1
            depth += 1
        elif character == close_character:
            depth -= 1
            if depth == 0:
                if start is None:
                    raise ValueError("balanced item has no start")
                items.append(text[start:index])
                start = None

    if depth != 0 or in_string or in_char:
        raise ValueError("unbalanced source while scanning items")
    return items


def c_strings(field: str) -> str:
    parts: list[str] = []
    macros = {
        "NAME": "eprover",
        "WATCHLIST_INLINE_QSTRING": "'Use inline watchlist type'",
    }
    index = 0
    while index < len(field):
        if field[index].isspace():
            index += 1
            continue
        if field.startswith("\\\n", index):
            index += 2
            continue
        if field[index].isalpha() or field[index] == "_":
            start = index
            index += 1
            while index < len(field) and (field[index].isalnum() or field[index] == "_"):
                index += 1
            macro = field[start:index]
            if macro not in macros:
                raise ValueError(f"unknown C string macro {macro!r} in {field!r}")
            parts.append(macros[macro])
            continue
        if field[index] != '"':
            raise ValueError(f"expected C string literal in {field!r}")
        start = index
        index += 1
        escaped = False
        while index < len(field):
            character = field[index]
            index += 1
            if escaped:
                escaped = False
            elif character == "\\":
                escaped = True
            elif character == '"':
                break
        parts.append(ast.literal_eval(field[start:index]))
    return "".join(parts)


def c_option_descriptions() -> list[tuple[str, str]]:
    source = C_OPTIONS.read_text(encoding="utf-8")
    table_start = source.index("OptCell opts[]")
    body_start = source.index("{", table_start) + 1
    body_end = source.index("\n};", body_start)
    result: list[tuple[str, str]] = []

    for entry in balanced_items(source[body_start:body_end], "{", "}"):
        fields = split_fields(entry)
        if fields[0] == "OPT_NOOPT":
            continue
        if len(fields) != 6:
            raise ValueError(f"expected six C option fields, got {len(fields)}")
        result.append((c_strings(fields[2]), c_strings(fields[5])))
    return result


def rust_option_arguments(source: str) -> tuple[int, int, dict[str, list[str]]]:
    table_start = source.index("pub const EPROVER_OPTIONS")
    initializer = "= &["
    body_start = source.index(initializer, table_start) + len(initializer)
    body_end = source.index("\n];", body_start)
    body = source[body_start:body_end]
    result: dict[str, list[str]] = {}
    marker = "OptCell::new("
    search_from = 0

    while True:
        marker_start = body.find(marker, search_from)
        if marker_start < 0:
            break
        arguments_start = marker_start + len(marker)
        depth = 1
        index = arguments_start
        in_string = False
        in_char = False
        escaped = False
        while index < len(body) and depth:
            character = body[index]
            if escaped:
                escaped = False
            elif in_string:
                if character == "\\":
                    escaped = True
                elif character == '"':
                    in_string = False
            elif in_char:
                if character == "\\":
                    escaped = True
                elif character == "'":
                    in_char = False
            elif character == '"':
                in_string = True
            elif character == "'":
                in_char = True
            elif character == "(":
                depth += 1
            elif character == ")":
                depth -= 1
            index += 1
        if depth:
            raise ValueError("unterminated Rust OptCell::new call")
        arguments = split_fields(body[arguments_start : index - 1])
        if arguments[-1] == "":
            arguments.pop()
        if len(arguments) != 6:
            raise ValueError(
                f"expected six Rust option fields at offset {marker_start}, "
                f"got {len(arguments)}: {arguments!r}"
            )
        long_option = arguments[2]
        prefix = 'Some("'
        if not long_option.startswith(prefix) or not long_option.endswith('")'):
            raise ValueError(f"expected Rust long option, got {long_option!r}")
        name = long_option[len(prefix) : -2]
        if name in result:
            raise ValueError(f"duplicate Rust option {name}")
        result[name] = arguments
        search_from = index

    return body_start, body_end, result


def rust_string(value: str) -> str:
    rendered = json.dumps(value, ensure_ascii=True)
    if "\\u" in rendered:
        raise ValueError("unexpected non-ASCII option description")
    return rendered


def generated_source() -> str:
    source = RUST_OPTIONS.read_text(encoding="utf-8")
    body_start, body_end, rust_options = rust_option_arguments(source)
    c_options = c_option_descriptions()
    c_names = {name for name, _ in c_options}
    if c_names != rust_options.keys():
        missing = sorted(c_names - rust_options.keys())
        extra = sorted(rust_options.keys() - c_names)
        raise ValueError(f"option surface mismatch: missing={missing}, extra={extra}")

    blocks: list[str] = []
    for name, description in c_options:
        arguments = rust_options[name]
        arguments[5] = rust_string(description)
        rendered_arguments = "\n".join(f"        {argument}," for argument in arguments)
        blocks.append(f"    OptCell::new(\n{rendered_arguments}\n    ),")
    body = "\n" + "\n".join(blocks)
    return source[:body_start] + body + source[body_end:]


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    generated = generated_source()
    current = RUST_OPTIONS.read_text(encoding="utf-8")
    if args.check:
        if generated != current:
            print(f"out of date: {RUST_OPTIONS.relative_to(ROOT)}")
            return 1
        print(f"OK: {RUST_OPTIONS.relative_to(ROOT)} matches C option order and prose.")
        return 0
    RUST_OPTIONS.write_text(generated, encoding="utf-8", newline="\n")
    print(f"updated {RUST_OPTIONS.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
