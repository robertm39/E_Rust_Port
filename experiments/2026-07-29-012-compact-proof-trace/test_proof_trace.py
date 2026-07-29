#!/usr/bin/env python3
"""Controller tests for the compact proof-trace codec."""

from __future__ import annotations

import importlib.util
import io
import struct
import sys
import tempfile
import unittest
from pathlib import Path


MODULE_PATH = Path(__file__).with_name("proof_trace.py")
SPEC = importlib.util.spec_from_file_location("proof_trace", MODULE_PATH)
assert SPEC is not None and SPEC.loader is not None
proof_trace = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = proof_trace
SPEC.loader.exec_module(proof_trace)


def encoded(payload: bytes) -> bytes:
    destination = io.BytesIO()
    proof_trace.encode_stream(io.BytesIO(payload), destination)
    return destination.getvalue()


class ProofTraceTests(unittest.TestCase):
    def test_round_trip_preserves_boundary_cases(self) -> None:
        payloads = (
            b"",
            b"\n",
            b"one line",
            b"one line\n",
            b"\n\n",
            b"cnf(a,plain,p(a),file('x.p',a)).\r\n"
            b"cnf(b,plain,$false,inference(resolve,[status(thm)],[a])).\r\n",
        )
        for payload in payloads:
            with self.subTest(payload=payload):
                trace = encoded(payload)
                reconstructed, stats = proof_trace.decode_bytes(trace)
                self.assertEqual(reconstructed, payload)
                self.assertEqual(stats.raw_bytes, len(payload))

    def test_encoding_is_deterministic(self) -> None:
        payload = (b"fof(a,axiom,p(a)).\n" * 50) + b"tail"
        self.assertEqual(encoded(payload), encoded(payload))

    def test_compresses_repeated_proof_records(self) -> None:
        payload = b"".join(
            f"cnf(c_0_{index},plain,p(a),"
            f"inference(rewrite,[status(thm)],[c_0_1])).\n".encode("ascii")
            for index in range(1_000)
        )
        self.assertLess(len(encoded(payload)), len(payload) * 7 // 10)

    def test_truncated_frame_is_rejected_without_publication(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            trace = root / "proof.uptl"
            trace.write_bytes(encoded(b"first\nsecond\n")[:-5])
            output = root / "proof.out"
            with self.assertRaisesRegex(
                proof_trace.TraceFormatError,
                "trailer",
            ):
                proof_trace.atomic_replay(trace, output)
            self.assertFalse(output.exists())
            self.assertEqual(list(root.glob(".*.tmp")), [])

    def test_payload_flip_is_rejected_without_publication(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            mutated = bytearray(encoded(b"proof proof proof proof\n"))
            payload_index = len(proof_trace.MAGIC) + proof_trace.FRAME_HEADER.size
            mutated[payload_index] ^= 1
            trace = root / "proof.uptl"
            trace.write_bytes(mutated)
            output = root / "proof.out"
            with self.assertRaises(proof_trace.TraceFormatError):
                proof_trace.atomic_replay(trace, output)
            self.assertFalse(output.exists())
            self.assertEqual(list(root.glob(".*.tmp")), [])

    def test_invalid_length_is_rejected_without_publication(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            mutated = bytearray(encoded(b"proof\n"))
            raw_length_offset = len(proof_trace.MAGIC) + 1
            mutated[raw_length_offset : raw_length_offset + 4] = struct.pack(
                ">I", proof_trace.MAX_FRAME_BYTES + 1
            )
            trace = root / "proof.uptl"
            trace.write_bytes(mutated)
            output = root / "proof.out"
            with self.assertRaisesRegex(
                proof_trace.TraceFormatError,
                "frame 0",
            ):
                proof_trace.atomic_replay(trace, output)
            self.assertFalse(output.exists())
            self.assertEqual(list(root.glob(".*.tmp")), [])

    def test_atomic_replay_replaces_only_after_complete_validation(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            payload = b"first\nsecond\n"
            trace = root / "proof.uptl"
            trace.write_bytes(encoded(payload))
            output = root / "proof.out"
            stats = proof_trace.atomic_replay(trace, output)
            self.assertEqual(output.read_bytes(), payload)
            self.assertEqual(stats.frame_count, 1)


if __name__ == "__main__":
    unittest.main()
