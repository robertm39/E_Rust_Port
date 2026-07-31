#!/usr/bin/env python3
"""Shared extraction, representation, training, and metrics for the study."""

from __future__ import annotations

import hashlib
import json
import math
import random
import re
import tarfile
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable, Sequence


RECURSIVE_DIM = 12
HIDDEN_DIM = 8
SEEDS = (11, 23, 37, 53, 71)
SCALAR_NAMES = (
    "literal_count",
    "positive_count",
    "negative_count",
    "syntax_node_count",
    "maximum_depth",
    "variable_occurrences",
    "distinct_variables",
    "symbol_occurrences",
    "distinct_symbols",
    "equality_count",
)


@dataclass(frozen=True)
class Node:
    symbol: str
    children: tuple["Node", ...] = ()


@dataclass(frozen=True)
class Literal:
    positive: bool
    atom: Node


@dataclass(frozen=True)
class Observation:
    problem: str
    family: str
    split: str
    index: int
    raw_clause: str
    literals: tuple[Literal, ...]
    canonical: str
    label: int


@dataclass(frozen=True)
class ManifestRecord:
    archive_member: str
    family: str
    problem: str
    split: str
    sha256: str
    given_count: int
    positive_count: int
    proof_evalgc_count: int
    unmatched_evalgc_count: int

    @classmethod
    def from_dict(cls, value: dict[str, object]) -> "ManifestRecord":
        return cls(
            archive_member=str(value["archive_member"]),
            family=str(value["family"]),
            problem=str(value["problem"]),
            split=str(value["split"]),
            sha256=str(value["sha256"]),
            given_count=int(value["given_count"]),
            positive_count=int(value["positive_count"]),
            proof_evalgc_count=int(value["proof_evalgc_count"]),
            unmatched_evalgc_count=int(value["unmatched_evalgc_count"]),
        )


class IntegrityError(RuntimeError):
    """The frozen input or a derived invariant does not match the contract."""


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def load_manifest(path: Path) -> list[ManifestRecord]:
    records = []
    for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        if not line.strip():
            continue
        try:
            records.append(ManifestRecord.from_dict(json.loads(line)))
        except (KeyError, TypeError, ValueError, json.JSONDecodeError) as error:
            raise IntegrityError(f"{path}:{line_number}: invalid manifest: {error}") from error
    if not records:
        raise IntegrityError(f"{path}: empty manifest")
    keys = {(record.problem, record.split) for record in records}
    if len(keys) != len(records):
        raise IntegrityError(f"{path}: duplicate problem/split record")
    return records


def split_top_level(text: str, delimiter: str = ",", maxsplit: int = -1) -> list[str]:
    pieces: list[str] = []
    start = 0
    depth_round = 0
    depth_square = 0
    quote: str | None = None
    escaped = False
    splits = 0
    for index, char in enumerate(text):
        if quote is not None:
            if escaped:
                escaped = False
            elif char == "\\":
                escaped = True
            elif char == quote:
                quote = None
            continue
        if char in ("'", '"'):
            quote = char
        elif char == "(":
            depth_round += 1
        elif char == ")":
            depth_round -= 1
        elif char == "[":
            depth_square += 1
        elif char == "]":
            depth_square -= 1
        elif (
            char == delimiter
            and depth_round == 0
            and depth_square == 0
            and (maxsplit < 0 or splits < maxsplit)
        ):
            pieces.append(text[start:index].strip())
            start = index + 1
            splits += 1
        if depth_round < 0 or depth_square < 0:
            raise ValueError(f"unbalanced expression: {text!r}")
    if quote is not None or depth_round != 0 or depth_square != 0:
        raise ValueError(f"unbalanced expression: {text!r}")
    pieces.append(text[start:].strip())
    return pieces


def _has_single_outer_parentheses(text: str) -> bool:
    if len(text) < 2 or text[0] != "(" or text[-1] != ")":
        return False
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
        if char in ("'", '"'):
            quote = char
        elif char == "(":
            depth += 1
        elif char == ")":
            depth -= 1
            if depth == 0 and index != len(text) - 1:
                return False
        if depth < 0:
            return False
    return depth == 0 and quote is None


def strip_outer_parentheses(text: str) -> str:
    result = text.strip()
    while _has_single_outer_parentheses(result):
        result = result[1:-1].strip()
    return result


TOKEN_RE = re.compile(
    r"""\s*(?:
        (!=|\+\+|--|[(),=~|])
        |
        ('(?:\\.|[^'])*'|"(?:\\.|[^"])*"|[A-Za-z_$][A-Za-z0-9_$]*|-?[0-9]+(?:\.[0-9]+)?(?:/[0-9]+)?|[^\s(),=~|]+)
    )""",
    re.VERBOSE,
)


def tokenize(text: str) -> list[str]:
    tokens: list[str] = []
    position = 0
    while position < len(text):
        match = TOKEN_RE.match(text, position)
        if match is None:
            raise ValueError(f"cannot tokenize at {text[position:position + 30]!r}")
        tokens.append(match.group(1) or match.group(2))
        position = match.end()
    return tokens


def _parse_term_tokens(tokens: Sequence[str], position: int = 0) -> tuple[Node, int]:
    if position >= len(tokens):
        raise ValueError("missing term")
    symbol = tokens[position]
    if symbol in {"(", ")", ",", "=", "!=", "~", "|", "++", "--"}:
        raise ValueError(f"expected term symbol, found {symbol!r}")
    position += 1
    children: list[Node] = []
    if position < len(tokens) and tokens[position] == "(":
        position += 1
        if position < len(tokens) and tokens[position] == ")":
            return Node(symbol, ()), position + 1
        while True:
            child, position = _parse_term_tokens(tokens, position)
            children.append(child)
            if position >= len(tokens):
                raise ValueError("unterminated term arguments")
            if tokens[position] == ")":
                position += 1
                break
            if tokens[position] != ",":
                raise ValueError(f"expected ',' or ')', found {tokens[position]!r}")
            position += 1
    return Node(symbol, tuple(children)), position


def parse_term(text: str) -> Node:
    tokens = tokenize(strip_outer_parentheses(text))
    node, position = _parse_term_tokens(tokens)
    if position != len(tokens):
        raise ValueError(f"trailing term tokens: {tokens[position:]!r}")
    return node


def _top_level_operator(text: str, operators: Sequence[str]) -> tuple[int, str] | None:
    depth = 0
    quote: str | None = None
    escaped = False
    index = 0
    while index < len(text):
        char = text[index]
        if quote is not None:
            if escaped:
                escaped = False
            elif char == "\\":
                escaped = True
            elif char == quote:
                quote = None
            index += 1
            continue
        if char in ("'", '"'):
            quote = char
        elif char == "(":
            depth += 1
        elif char == ")":
            depth -= 1
        elif depth == 0:
            for operator in operators:
                if text.startswith(operator, index):
                    return index, operator
        index += 1
    return None


def _node_shape(node: Node) -> str:
    symbol = "V" if is_variable(node.symbol) and not node.children else node.symbol
    if not node.children:
        return symbol
    return f"{symbol}({','.join(_node_shape(child) for child in node.children)})"


def _node_text(node: Node, variables: dict[str, str]) -> str:
    if is_variable(node.symbol) and not node.children:
        symbol = variables.setdefault(node.symbol, f"V{len(variables)}")
    else:
        symbol = node.symbol
    if node.symbol == "equal" and len(node.children) == 2:
        children = sorted(_node_text(child, variables) for child in node.children)
    else:
        children = [_node_text(child, variables) for child in node.children]
    if not children:
        return symbol
    return f"{symbol}({','.join(children)})"


def parse_literal(text: str, proof_syntax: bool = False) -> Literal:
    value = strip_outer_parentheses(text)
    positive = True
    if proof_syntax and value.startswith(("++", "--")):
        positive = value.startswith("++")
        value = value[2:].strip()
    elif value.startswith("~"):
        positive = False
        value = strip_outer_parentheses(value[1:].strip())

    equality = _top_level_operator(value, ("!=", "="))
    if equality is not None:
        index, operator = equality
        left = parse_term(value[:index])
        right = parse_term(value[index + len(operator) :])
        if operator == "!=":
            positive = not positive
        atom = Node("equal", (left, right))
    else:
        atom = parse_term(value)
    return Literal(positive, atom)


def parse_clause(text: str, proof_syntax: bool = False) -> tuple[Literal, ...]:
    value = text.strip()
    if proof_syntax:
        if not (value.startswith("[") and value.endswith("]")):
            raise ValueError(f"proof clause is not bracketed: {value!r}")
        value = value[1:-1].strip()
        if not value:
            return ()
        parts = split_top_level(value)
    else:
        value = strip_outer_parentheses(value)
        parts = split_top_level(value, delimiter="|")
    return tuple(parse_literal(part, proof_syntax=proof_syntax) for part in parts)


def canonical_clause(literals: Sequence[Literal]) -> str:
    ordered = sorted(
        literals,
        key=lambda literal: (
            0 if literal.positive else 1,
            _node_shape(literal.atom),
        ),
    )
    variables: dict[str, str] = {}
    rendered = [
        ("+" if literal.positive else "-") + _node_text(literal.atom, variables)
        for literal in ordered
    ]
    return "|".join(sorted(rendered))


def _balanced_slice(text: str, start: int, opening: str, closing: str) -> str:
    if start >= len(text) or text[start] != opening:
        raise ValueError(f"expected {opening!r}")
    depth = 0
    quote: str | None = None
    escaped = False
    for index in range(start, len(text)):
        char = text[index]
        if quote is not None:
            if escaped:
                escaped = False
            elif char == "\\":
                escaped = True
            elif char == quote:
                quote = None
            continue
        if char in ("'", '"'):
            quote = char
        elif char == opening:
            depth += 1
        elif char == closing:
            depth -= 1
            if depth == 0:
                return text[start : index + 1]
    raise ValueError(f"unterminated {opening!r} expression")


def extract_trace(
    text: str, record: ManifestRecord
) -> tuple[list[Observation], dict[str, int]]:
    marker = "% SZS output start CNFRefutation"
    end_marker = "% SZS output end CNFRefutation"
    if marker not in text or end_marker not in text:
        raise IntegrityError(f"{record.problem}: missing completed CNF refutation")
    prefix, proof = text.split(marker, 1)
    proof = proof.split(end_marker, 1)[0]

    given: list[tuple[str, tuple[Literal, ...], str]] = []
    for line_number, line in enumerate(prefix.splitlines(), 1):
        if not line.startswith("%cnf("):
            continue
        if not line.endswith(")."):
            raise IntegrityError(f"{record.problem}:{line_number}: split given-clause line")
        inner = line[len("%cnf(") : -2]
        parts = split_top_level(inner, maxsplit=2)
        if len(parts) != 3:
            raise IntegrityError(f"{record.problem}:{line_number}: malformed cnf comment")
        raw_clause = parts[2]
        try:
            literals = parse_clause(raw_clause)
        except ValueError as error:
            raise IntegrityError(
                f"{record.problem}:{line_number}: cannot parse given clause: {error}"
            ) from error
        given.append((raw_clause, literals, canonical_clause(literals)))

    proof_canonicals: list[str] = []
    for line_number, line in enumerate(proof.splitlines(), 1):
        if ": evalgc(" not in line:
            continue
        start = line.find("[")
        if start < 0:
            raise IntegrityError(f"{record.problem}:proof:{line_number}: missing clause")
        try:
            raw_clause = _balanced_slice(line, start, "[", "]")
            literals = parse_clause(raw_clause, proof_syntax=True)
        except ValueError as error:
            raise IntegrityError(
                f"{record.problem}:proof:{line_number}: cannot parse evalgc: {error}"
            ) from error
        proof_canonicals.append(canonical_clause(literals))

    proof_set = set(proof_canonicals)
    observations = [
        Observation(
            problem=record.problem,
            family=record.family,
            split=record.split,
            index=index,
            raw_clause=raw_clause,
            literals=literals,
            canonical=canonical,
            label=int(canonical in proof_set),
        )
        for index, (raw_clause, literals, canonical) in enumerate(given)
    ]
    given_set = {observation.canonical for observation in observations}
    summary = {
        "given_count": len(observations),
        "positive_count": sum(observation.label for observation in observations),
        "proof_evalgc_count": len(proof_canonicals),
        "unmatched_evalgc_count": sum(
            canonical not in given_set for canonical in proof_canonicals
        ),
    }
    expected = {
        "given_count": record.given_count,
        "positive_count": record.positive_count,
        "proof_evalgc_count": record.proof_evalgc_count,
        "unmatched_evalgc_count": record.unmatched_evalgc_count,
    }
    if summary != expected:
        raise IntegrityError(
            f"{record.problem}: extraction mismatch: expected {expected}, got {summary}"
        )
    return observations, summary


def read_split_from_archive(
    archive_path: Path,
    manifest: Sequence[ManifestRecord],
    splits: set[str],
    expected_archive_sha256: str,
) -> tuple[list[Observation], list[dict[str, object]]]:
    actual_archive_hash = sha256_file(archive_path)
    if actual_archive_hash != expected_archive_sha256:
        raise IntegrityError(
            f"archive SHA-256 mismatch: expected {expected_archive_sha256}, "
            f"got {actual_archive_hash}"
        )
    observations: list[Observation] = []
    extraction: list[dict[str, object]] = []
    with tarfile.open(archive_path, "r:*") as archive:
        names = set(archive.getnames())
        for record in manifest:
            if record.split not in splits:
                continue
            if record.archive_member not in names:
                raise IntegrityError(f"archive member missing: {record.archive_member}")
            extracted = archive.extractfile(record.archive_member)
            if extracted is None:
                raise IntegrityError(f"not a regular member: {record.archive_member}")
            data = extracted.read()
            actual_hash = sha256_bytes(data)
            if actual_hash != record.sha256:
                raise IntegrityError(
                    f"{record.problem}: trace SHA-256 mismatch: expected "
                    f"{record.sha256}, got {actual_hash}"
                )
            try:
                text = data.decode("utf-8")
            except UnicodeDecodeError as error:
                raise IntegrityError(f"{record.problem}: trace is not UTF-8") from error
            rows, summary = extract_trace(text, record)
            observations.extend(rows)
            extraction.append(
                {
                    "problem": record.problem,
                    "family": record.family,
                    "split": record.split,
                    "archive_member": record.archive_member,
                    "sha256": actual_hash,
                    **summary,
                }
            )
    return observations, extraction


def is_variable(symbol: str) -> bool:
    return bool(symbol) and (symbol[0].isupper() or symbol[0] == "_")


def _walk_node(node: Node, depth: int = 1) -> Iterable[tuple[Node, int]]:
    yield node, depth
    for child in node.children:
        yield from _walk_node(child, depth + 1)


def scalar_features(literals: Sequence[Literal]) -> list[float]:
    nodes: list[tuple[Node, int]] = []
    for literal in literals:
        nodes.extend(_walk_node(literal.atom))
    variables = [
        node.symbol
        for node, _ in nodes
        if not node.children and is_variable(node.symbol)
    ]
    symbols = [
        node.symbol
        for node, _ in nodes
        if node.children or not is_variable(node.symbol)
    ]
    return [
        float(len(literals)),
        float(sum(literal.positive for literal in literals)),
        float(sum(not literal.positive for literal in literals)),
        float(len(nodes)),
        float(max((depth for _, depth in nodes), default=0)),
        float(len(variables)),
        float(len(set(variables))),
        float(len(symbols)),
        float(len(set(symbols))),
        float(sum(literal.atom.symbol == "equal" for literal in literals)),
    ]


class RecursiveEncoder:
    def __init__(self, seed: int, dimension: int = RECURSIVE_DIM):
        self.seed = seed
        self.dimension = dimension
        self._base_cache: dict[tuple[str, int], tuple[float, ...]] = {}

    def _base(self, symbol: str, arity: int) -> tuple[float, ...]:
        key = ("VAR" if is_variable(symbol) and arity == 0 else symbol, arity)
        cached = self._base_cache.get(key)
        if cached is not None:
            return cached
        values = []
        for coordinate in range(self.dimension):
            digest = hashlib.blake2b(
                f"{self.seed}|{key[0]}|{arity}|{coordinate}".encode("utf-8"),
                digest_size=8,
            ).digest()
            unit = int.from_bytes(digest, "big") / float((1 << 64) - 1)
            values.append(2.0 * unit - 1.0)
        result = tuple(values)
        self._base_cache[key] = result
        return result

    def node(self, node: Node) -> list[float]:
        base = self._base(node.symbol, len(node.children))
        children = [self.node(child) for child in node.children]
        output = []
        for coordinate in range(self.dimension):
            value = base[coordinate]
            for child_index, child in enumerate(children):
                value += (0.65 / (child_index + 1)) * child[coordinate]
                value += 0.15 * child[
                    (coordinate + child_index + 1) % self.dimension
                ]
            output.append(math.tanh(value))
        return output

    def clause(self, literals: Sequence[Literal]) -> list[float]:
        if not literals:
            return [0.0] * (2 * self.dimension)
        encoded = []
        for literal in literals:
            wrapper = Node("POS" if literal.positive else "NEG", (literal.atom,))
            encoded.append(self.node(wrapper))
        means = [
            sum(row[coordinate] for row in encoded) / len(encoded)
            for coordinate in range(self.dimension)
        ]
        maxima = [
            max(row[coordinate] for row in encoded)
            for coordinate in range(self.dimension)
        ]
        return means + maxima


def fit_normalizer(rows: Sequence[Sequence[float]]) -> tuple[list[float], list[float]]:
    if not rows:
        raise ValueError("cannot normalize an empty matrix")
    width = len(rows[0])
    means = [sum(row[index] for row in rows) / len(rows) for index in range(width)]
    scales = []
    for index, mean in enumerate(means):
        variance = sum((row[index] - mean) ** 2 for row in rows) / len(rows)
        scale = math.sqrt(variance)
        scales.append(scale if scale >= 1e-9 else 1.0)
    return means, scales


def normalize_rows(
    rows: Sequence[Sequence[float]], means: Sequence[float], scales: Sequence[float]
) -> list[list[float]]:
    return [
        [(value - means[index]) / scales[index] for index, value in enumerate(row)]
        for row in rows
    ]


def _sigmoid(value: float) -> float:
    if value >= 0.0:
        inverse = math.exp(-value)
        return 1.0 / (1.0 + inverse)
    exponential = math.exp(value)
    return exponential / (1.0 + exponential)


def _clip_scale(values: Iterable[float], maximum_norm: float = 5.0) -> float:
    squared = sum(value * value for value in values)
    norm = math.sqrt(squared)
    return maximum_norm / norm if norm > maximum_norm else 1.0


@dataclass
class LinearModel:
    means: list[float]
    scales: list[float]
    weights: list[float]
    bias: float

    def score_features(self, values: Sequence[float]) -> float:
        normalized = [
            (value - self.means[index]) / self.scales[index]
            for index, value in enumerate(values)
        ]
        return self.bias + sum(
            weight * value for weight, value in zip(self.weights, normalized)
        )

    def score_clause(self, literals: Sequence[Literal]) -> float:
        return self.score_features(scalar_features(literals))

    def to_dict(self) -> dict[str, object]:
        return {
            "kind": "linear",
            "scalar_names": list(SCALAR_NAMES),
            "means": self.means,
            "scales": self.scales,
            "weights": self.weights,
            "bias": self.bias,
        }


@dataclass
class RecursiveModel:
    seed: int
    means: list[float]
    scales: list[float]
    weights1: list[list[float]]
    bias1: list[float]
    weights2: list[float]
    bias2: float

    def raw_features(
        self, literals: Sequence[Literal], encoder: RecursiveEncoder | None = None
    ) -> list[float]:
        active_encoder = encoder or RecursiveEncoder(self.seed)
        return scalar_features(literals) + active_encoder.clause(literals)

    def score_features(self, values: Sequence[float]) -> float:
        normalized = [
            (value - self.means[index]) / self.scales[index]
            for index, value in enumerate(values)
        ]
        hidden = [
            math.tanh(
                bias
                + sum(weight * value for weight, value in zip(row, normalized))
            )
            for row, bias in zip(self.weights1, self.bias1)
        ]
        return self.bias2 + sum(
            weight * value for weight, value in zip(self.weights2, hidden)
        )

    def score_clause(
        self, literals: Sequence[Literal], encoder: RecursiveEncoder | None = None
    ) -> float:
        return self.score_features(self.raw_features(literals, encoder))

    def to_dict(self) -> dict[str, object]:
        return {
            "kind": "recursive",
            "seed": self.seed,
            "recursive_dimension": RECURSIVE_DIM,
            "hidden_dimension": HIDDEN_DIM,
            "scalar_names": list(SCALAR_NAMES),
            "means": self.means,
            "scales": self.scales,
            "weights1": self.weights1,
            "bias1": self.bias1,
            "weights2": self.weights2,
            "bias2": self.bias2,
        }


def model_from_dict(value: dict[str, object]) -> LinearModel | RecursiveModel:
    kind = value.get("kind")
    if kind == "linear":
        return LinearModel(
            means=[float(item) for item in value["means"]],
            scales=[float(item) for item in value["scales"]],
            weights=[float(item) for item in value["weights"]],
            bias=float(value["bias"]),
        )
    if kind == "recursive":
        if int(value["recursive_dimension"]) != RECURSIVE_DIM:
            raise ValueError("recursive dimension mismatch")
        if int(value["hidden_dimension"]) != HIDDEN_DIM:
            raise ValueError("hidden dimension mismatch")
        return RecursiveModel(
            seed=int(value["seed"]),
            means=[float(item) for item in value["means"]],
            scales=[float(item) for item in value["scales"]],
            weights1=[
                [float(item) for item in row] for row in value["weights1"]
            ],
            bias1=[float(item) for item in value["bias1"]],
            weights2=[float(item) for item in value["weights2"]],
            bias2=float(value["bias2"]),
        )
    raise ValueError(f"unsupported model kind: {kind!r}")


def train_linear(observations: Sequence[Observation]) -> LinearModel:
    raw = [scalar_features(observation.literals) for observation in observations]
    labels = [observation.label for observation in observations]
    means, scales = fit_normalizer(raw)
    rows = normalize_rows(raw, means, scales)
    width = len(rows[0])
    weights = [0.0] * width
    bias = 0.0
    positives = sum(labels)
    negatives = len(labels) - positives
    if positives == 0 or negatives == 0:
        raise ValueError("linear training requires both classes")
    positive_weight = len(labels) / (2.0 * positives)
    negative_weight = len(labels) / (2.0 * negatives)
    for _ in range(300):
        gradient = [0.0] * width
        bias_gradient = 0.0
        for row, label in zip(rows, labels):
            prediction = _sigmoid(bias + sum(w * x for w, x in zip(weights, row)))
            class_weight = positive_weight if label else negative_weight
            error = class_weight * (prediction - label)
            bias_gradient += error
            for index, value in enumerate(row):
                gradient[index] += error * value
        divisor = float(len(rows))
        gradient = [
            value / divisor + 0.0001 * weights[index]
            for index, value in enumerate(gradient)
        ]
        bias_gradient /= divisor
        scale = _clip_scale([*gradient, bias_gradient])
        for index in range(width):
            weights[index] -= 0.03 * scale * gradient[index]
        bias -= 0.03 * scale * bias_gradient
    return LinearModel(means, scales, weights, bias)


def train_recursive(observations: Sequence[Observation], seed: int) -> RecursiveModel:
    encoder = RecursiveEncoder(seed)
    raw = [
        scalar_features(observation.literals) + encoder.clause(observation.literals)
        for observation in observations
    ]
    labels = [observation.label for observation in observations]
    means, scales = fit_normalizer(raw)
    rows = normalize_rows(raw, means, scales)
    width = len(rows[0])
    positives = [index for index, label in enumerate(labels) if label]
    negatives = [index for index, label in enumerate(labels) if not label]
    if not positives or not negatives:
        raise ValueError("recursive training requires both classes")
    rng = random.Random(seed)
    rng.shuffle(negatives)
    weights1 = [
        [rng.uniform(-1.0 / math.sqrt(width), 1.0 / math.sqrt(width)) for _ in range(width)]
        for _ in range(HIDDEN_DIM)
    ]
    bias1 = [0.0] * HIDDEN_DIM
    weights2 = [
        rng.uniform(-1.0 / math.sqrt(HIDDEN_DIM), 1.0 / math.sqrt(HIDDEN_DIM))
        for _ in range(HIDDEN_DIM)
    ]
    bias2 = 0.0
    cursor = 0

    for _ in range(160):
        selected_negatives = []
        while len(selected_negatives) < len(positives):
            remaining = len(negatives) - cursor
            take = min(len(positives) - len(selected_negatives), remaining)
            selected_negatives.extend(negatives[cursor : cursor + take])
            cursor += take
            if cursor == len(negatives):
                rng.shuffle(negatives)
                cursor = 0
        batch = positives + selected_negatives
        rng.shuffle(batch)
        gradient1 = [[0.0] * width for _ in range(HIDDEN_DIM)]
        bias_gradient1 = [0.0] * HIDDEN_DIM
        gradient2 = [0.0] * HIDDEN_DIM
        bias_gradient2 = 0.0

        for row_index in batch:
            row = rows[row_index]
            label = labels[row_index]
            hidden = [
                math.tanh(
                    bias
                    + sum(weight * value for weight, value in zip(weight_row, row))
                )
                for weight_row, bias in zip(weights1, bias1)
            ]
            output = bias2 + sum(
                weight * value for weight, value in zip(weights2, hidden)
            )
            error = _sigmoid(output) - label
            bias_gradient2 += error
            for hidden_index in range(HIDDEN_DIM):
                gradient2[hidden_index] += error * hidden[hidden_index]
                hidden_error = (
                    error
                    * weights2[hidden_index]
                    * (1.0 - hidden[hidden_index] * hidden[hidden_index])
                )
                bias_gradient1[hidden_index] += hidden_error
                gradient_row = gradient1[hidden_index]
                for feature_index, value in enumerate(row):
                    gradient_row[feature_index] += hidden_error * value

        divisor = float(len(batch))
        flat_gradients: list[float] = []
        for hidden_index in range(HIDDEN_DIM):
            for feature_index in range(width):
                gradient1[hidden_index][feature_index] = (
                    gradient1[hidden_index][feature_index] / divisor
                    + 0.0001 * weights1[hidden_index][feature_index]
                )
                flat_gradients.append(gradient1[hidden_index][feature_index])
            bias_gradient1[hidden_index] /= divisor
            flat_gradients.append(bias_gradient1[hidden_index])
            gradient2[hidden_index] = (
                gradient2[hidden_index] / divisor
                + 0.0001 * weights2[hidden_index]
            )
            flat_gradients.append(gradient2[hidden_index])
        bias_gradient2 /= divisor
        flat_gradients.append(bias_gradient2)
        scale = _clip_scale(flat_gradients)

        for hidden_index in range(HIDDEN_DIM):
            for feature_index in range(width):
                weights1[hidden_index][feature_index] -= (
                    0.03 * scale * gradient1[hidden_index][feature_index]
                )
            bias1[hidden_index] -= 0.03 * scale * bias_gradient1[hidden_index]
            weights2[hidden_index] -= 0.03 * scale * gradient2[hidden_index]
        bias2 -= 0.03 * scale * bias_gradient2

    return RecursiveModel(
        seed, means, scales, weights1, bias1, weights2, bias2
    )


def score_observations(
    model: LinearModel | RecursiveModel, observations: Sequence[Observation]
) -> list[float]:
    if isinstance(model, RecursiveModel):
        encoder = RecursiveEncoder(model.seed)
        return [
            model.score_clause(observation.literals, encoder)
            for observation in observations
        ]
    return [model.score_clause(observation.literals) for observation in observations]


def _problem_metrics(labels: Sequence[int], scores: Sequence[float]) -> dict[str, float]:
    if len(labels) != len(scores) or not labels:
        raise ValueError("metrics require equally sized non-empty vectors")
    positives = sum(labels)
    negatives = len(labels) - positives
    if positives == 0 or negatives == 0:
        raise ValueError("metrics require both classes")
    ranked = sorted(range(len(labels)), key=lambda index: (-scores[index], index))
    found = 0
    precision_sum = 0.0
    positive_ranks = []
    for rank, index in enumerate(ranked, 1):
        if labels[index]:
            found += 1
            precision_sum += found / rank
            positive_ranks.append(rank)
    comparisons = 0.0
    for positive_index, label in enumerate(labels):
        if not label:
            continue
        for negative_index, negative_label in enumerate(labels):
            if negative_label:
                continue
            if scores[positive_index] > scores[negative_index]:
                comparisons += 1.0
            elif scores[positive_index] == scores[negative_index]:
                comparisons += 0.5
    auc = comparisons / (positives * negatives)
    result = {
        "average_precision": precision_sum / positives,
        "roc_auc": auc,
        "pairwise_accuracy": auc,
        "all_positive_prefix_fraction": max(positive_ranks) / len(labels),
    }
    for percentage in (1, 5, 10, 20):
        limit = max(1, math.ceil(len(labels) * percentage / 100.0))
        result[f"top_{percentage}_percent_recall"] = (
            sum(labels[index] for index in ranked[:limit]) / positives
        )
    return result


def evaluate_scores(
    observations: Sequence[Observation], scores: Sequence[float]
) -> dict[str, object]:
    grouped: dict[str, list[int]] = {}
    for index, observation in enumerate(observations):
        grouped.setdefault(observation.problem, []).append(index)
    problems: dict[str, dict[str, float]] = {}
    for problem, indices in grouped.items():
        labels = [observations[index].label for index in indices]
        problem_scores = [scores[index] for index in indices]
        problems[problem] = _problem_metrics(labels, problem_scores)
    metric_names = next(iter(problems.values())).keys()
    macro = {
        metric: sum(problem[metric] for problem in problems.values()) / len(problems)
        for metric in metric_names
    }
    return {"macro": macro, "problems": problems}


def chronological_scores(observations: Sequence[Observation]) -> list[float]:
    return [-float(observation.index) for observation in observations]


def scores_checksum(scores: Sequence[float]) -> str:
    data = "\n".join(format(score, ".17g") for score in scores).encode("ascii")
    return sha256_bytes(data)


def save_model(model: LinearModel | RecursiveModel, path: Path) -> int:
    data = (
        json.dumps(model.to_dict(), sort_keys=True, separators=(",", ":")) + "\n"
    ).encode("utf-8")
    path.write_bytes(data)
    return len(data)


def load_model(path: Path) -> LinearModel | RecursiveModel:
    return model_from_dict(json.loads(path.read_text(encoding="utf-8")))
