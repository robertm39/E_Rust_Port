#!/usr/bin/env python3
"""Deterministic framed storage and atomic replay for exact proof bytes."""

from __future__ import annotations

import argparse
import hashlib
import io
import os
import struct
import sys
import tempfile
import time
import zlib
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import BinaryIO, Callable


MAGIC = b"UPTL\x00\x01\r\n"
FRAME_RAW = 1
FRAME_ZLIB = 2
TRAILER = 0
FRAME_HEADER = struct.Struct(">BIII")
TRAILER_RECORD = struct.Struct(">BQQ32s")
MAX_FRAME_BYTES = 64 * 1024 * 1024
TARGET_FRAME_BYTES = 64 * 1024


class TraceFormatError(ValueError):
    """A compact proof trace failed a framing or integrity check."""


@dataclass(frozen=True)
class TraceStats:
    frame_count: int
    raw_bytes: int
    trace_bytes: int
    sha256: str


def _read_exact(stream: BinaryIO, size: int, context: str) -> bytes:
    data = stream.read(size)
    if len(data) != size:
        raise TraceFormatError(
            f"{context}: expected {size} bytes, found {len(data)}"
        )
    return data


def encode_stream(source: BinaryIO, destination: BinaryIO) -> TraceStats:
    """Encode one byte stream as deterministic line-boundary-aligned frames."""

    destination.write(MAGIC)
    digest = hashlib.sha256()
    frame_count = 0
    raw_bytes = 0
    pending = bytearray()

    def write_frame(raw: bytes) -> None:
        nonlocal frame_count
        compressed = zlib.compress(raw, level=9)
        if len(compressed) < len(raw):
            kind = FRAME_ZLIB
            payload = compressed
        else:
            kind = FRAME_RAW
            payload = raw
        destination.write(
            FRAME_HEADER.pack(
                kind,
                len(raw),
                len(payload),
                zlib.crc32(raw),
            )
        )
        destination.write(payload)
        frame_count += 1

    while True:
        line = source.readline(MAX_FRAME_BYTES + 1)
        if not line:
            break
        if len(line) > MAX_FRAME_BYTES:
            raise TraceFormatError(
                f"frame {frame_count}: line exceeds the format limit"
            )
        if pending and len(pending) + len(line) > TARGET_FRAME_BYTES:
            write_frame(bytes(pending))
            pending.clear()
        pending.extend(line)
        if len(pending) >= TARGET_FRAME_BYTES:
            write_frame(bytes(pending))
            pending.clear()
        digest.update(line)
        raw_bytes += len(line)

    if pending:
        write_frame(bytes(pending))

    destination.write(
        TRAILER_RECORD.pack(
            TRAILER,
            frame_count,
            raw_bytes,
            digest.digest(),
        )
    )
    trace_bytes = destination.tell()
    return TraceStats(
        frame_count=frame_count,
        raw_bytes=raw_bytes,
        trace_bytes=trace_bytes,
        sha256=digest.hexdigest(),
    )


def encode_path_to_bytes(source: Path) -> tuple[bytes, TraceStats]:
    """Encode a file without retaining its original contents."""

    destination = io.BytesIO()
    with source.open("rb") as stream:
        stats = encode_stream(stream, destination)
    return destination.getvalue(), stats


def encode_path_to_path(source: Path, destination: Path) -> TraceStats:
    """Incrementally encode a file into a spool."""

    with source.open("rb") as input_stream, destination.open("wb") as output_stream:
        return encode_stream(input_stream, output_stream)


def decode_stream(
    source: BinaryIO,
    destination: BinaryIO,
    *,
    after_frame: Callable[[int], None] | None = None,
) -> TraceStats:
    """Validate and reconstruct a complete framed proof stream."""

    magic = _read_exact(source, len(MAGIC), "trace header")
    if magic != MAGIC:
        raise TraceFormatError("trace header: bad magic or unsupported version")

    digest = hashlib.sha256()
    frame_count = 0
    raw_bytes = 0

    while True:
        tag_bytes = source.read(1)
        if not tag_bytes:
            raise TraceFormatError(
                f"frame {frame_count}: missing trailer after complete frames"
            )
        tag = tag_bytes[0]
        if tag == TRAILER:
            trailer_tail = _read_exact(
                source,
                TRAILER_RECORD.size - 1,
                f"trailer after frame {frame_count}",
            )
            expected_count, expected_bytes, expected_digest = struct.unpack(
                ">QQ32s", trailer_tail
            )
            if expected_count != frame_count:
                raise TraceFormatError(
                    "trailer: frame count mismatch "
                    f"(expected {expected_count}, reconstructed {frame_count})"
                )
            if expected_bytes != raw_bytes:
                raise TraceFormatError(
                    "trailer: byte count mismatch "
                    f"(expected {expected_bytes}, reconstructed {raw_bytes})"
                )
            actual_digest = digest.digest()
            if expected_digest != actual_digest:
                raise TraceFormatError("trailer: SHA-256 mismatch")
            if source.read(1):
                raise TraceFormatError("trailer: unexpected bytes after trailer")
            return TraceStats(
                frame_count=frame_count,
                raw_bytes=raw_bytes,
                trace_bytes=source.tell(),
                sha256=actual_digest.hex(),
            )

        if tag not in {FRAME_RAW, FRAME_ZLIB}:
            raise TraceFormatError(f"frame {frame_count}: unknown tag {tag}")
        header_tail = _read_exact(
            source,
            FRAME_HEADER.size - 1,
            f"frame {frame_count} header",
        )
        raw_length, stored_length, expected_crc = struct.unpack(
            ">III", header_tail
        )
        if raw_length > MAX_FRAME_BYTES or stored_length > MAX_FRAME_BYTES:
            raise TraceFormatError(
                f"frame {frame_count}: declared length exceeds the format limit"
            )
        if tag == FRAME_RAW and raw_length != stored_length:
            raise TraceFormatError(
                f"frame {frame_count}: raw frame lengths disagree"
            )
        payload = _read_exact(
            source,
            stored_length,
            f"frame {frame_count} payload",
        )
        if tag == FRAME_ZLIB:
            try:
                decompressor = zlib.decompressobj()
                raw = decompressor.decompress(payload, raw_length + 1)
                raw += decompressor.flush()
            except zlib.error as error:
                raise TraceFormatError(
                    f"frame {frame_count}: zlib decompression failed"
                ) from error
            if (
                decompressor.unconsumed_tail
                or decompressor.unused_data
                or not decompressor.eof
            ):
                raise TraceFormatError(
                    f"frame {frame_count}: invalid zlib stream boundary"
                )
        else:
            raw = payload
        if len(raw) != raw_length:
            raise TraceFormatError(
                f"frame {frame_count}: reconstructed length mismatch "
                f"(expected {raw_length}, found {len(raw)})"
            )
        if zlib.crc32(raw) != expected_crc:
            raise TraceFormatError(f"frame {frame_count}: CRC-32 mismatch")

        destination.write(raw)
        digest.update(raw)
        raw_bytes += len(raw)
        frame_count += 1
        if after_frame is not None:
            after_frame(frame_count)


def decode_bytes(trace: bytes) -> tuple[bytes, TraceStats]:
    """Reconstruct a compact in-memory log."""

    source = io.BytesIO(trace)
    destination = io.BytesIO()
    stats = decode_stream(source, destination)
    return destination.getvalue(), stats


def atomic_replay(
    trace: Path,
    destination: Path,
    *,
    pause_after_frame: int | None = None,
    ready_file: Path | None = None,
) -> TraceStats:
    """Replay a spool and publish only after complete validation."""

    destination.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(
        dir=destination.parent,
        prefix=f".{destination.name}.",
        suffix=".tmp",
    )
    temporary = Path(temporary_name)

    def after_frame(frame_count: int) -> None:
        if pause_after_frame is None or frame_count != pause_after_frame:
            return
        if ready_file is not None:
            ready_file.write_text(str(frame_count), encoding="ascii")
        time.sleep(300)

    try:
        with trace.open("rb") as source, os.fdopen(descriptor, "wb") as output:
            stats = decode_stream(source, output, after_frame=after_frame)
            output.flush()
            os.fsync(output.fileno())
        os.replace(temporary, destination)
        return stats
    except BaseException:
        try:
            os.close(descriptor)
        except OSError:
            pass
        temporary.unlink(missing_ok=True)
        raise


def _worker(arguments: argparse.Namespace) -> None:
    if arguments.mode == "noop":
        print(hashlib.sha256(b"").hexdigest())
        return
    source = arguments.input.resolve()
    if arguments.mode == "eager-retain":
        payload = source.read_bytes()
        print(hashlib.sha256(payload).hexdigest())
        return
    if arguments.mode == "compact-retain":
        trace, _ = encode_path_to_bytes(source)
        print(hashlib.sha256(trace).hexdigest())
        return
    if arguments.mode == "compact-replay":
        payload, _ = decode_bytes(source.read_bytes())
        print(hashlib.sha256(payload).hexdigest())
        return
    if arguments.mode == "spooled-replay":
        if arguments.output is None:
            raise TraceFormatError("spooled-replay requires --output")
        stats = atomic_replay(source, arguments.output.resolve())
        print(stats.sha256)
        return
    raise TraceFormatError(f"unsupported worker mode: {arguments.mode}")


def _parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)

    encode_parser = subparsers.add_parser("encode")
    encode_parser.add_argument("--input", type=Path, required=True)
    encode_parser.add_argument("--output", type=Path, required=True)

    replay_parser = subparsers.add_parser("replay")
    replay_parser.add_argument("--input", type=Path, required=True)
    replay_parser.add_argument("--output", type=Path, required=True)
    replay_parser.add_argument("--pause-after-frame", type=int)
    replay_parser.add_argument("--ready-file", type=Path)

    worker_parser = subparsers.add_parser("worker")
    worker_parser.add_argument(
        "--mode",
        choices=(
            "noop",
            "eager-retain",
            "compact-retain",
            "compact-replay",
            "spooled-replay",
        ),
        required=True,
    )
    worker_parser.add_argument("--input", type=Path)
    worker_parser.add_argument("--output", type=Path)
    return parser.parse_args()


def main() -> None:
    arguments = _parse_args()
    if arguments.command == "encode":
        stats = encode_path_to_path(
            arguments.input.resolve(),
            arguments.output.resolve(),
        )
        print(asdict(stats))
        return
    if arguments.command == "replay":
        stats = atomic_replay(
            arguments.input.resolve(),
            arguments.output.resolve(),
            pause_after_frame=arguments.pause_after_frame,
            ready_file=arguments.ready_file.resolve()
            if arguments.ready_file is not None
            else None,
        )
        print(asdict(stats))
        return
    _worker(arguments)


if __name__ == "__main__":
    try:
        main()
    except (OSError, TraceFormatError) as error:
        print(f"proof_trace.py: {error}", file=sys.stderr)
        raise SystemExit(2) from error
