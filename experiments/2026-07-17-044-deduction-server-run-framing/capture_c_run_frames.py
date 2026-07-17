#!/usr/bin/env python3
"""Capture one C e_deduction_server RUN exchange at the TCP-message boundary."""

from __future__ import annotations

import argparse
import json
import os
import signal
import socket
import struct
import subprocess
import time
from pathlib import Path


SUCCESS = b"200 ok : success\n"


def pack_frame(payload: bytes) -> bytes:
    return struct.pack("!I", len(payload) + 4) + payload


def recv_exact(stream: socket.socket, length: int) -> bytes:
    chunks: list[bytes] = []
    remaining = length
    while remaining:
        chunk = stream.recv(remaining)
        if not chunk:
            raise ConnectionError("deduction server closed the connection")
        chunks.append(chunk)
        remaining -= len(chunk)
    return b"".join(chunks)


def recv_frame(stream: socket.socket) -> tuple[bytes, bytes]:
    header = recv_exact(stream, 4)
    (total_length,) = struct.unpack("!I", header)
    if total_length < 4:
        raise ValueError(f"invalid TCP-message length {total_length}")
    payload = recv_exact(stream, total_length - 4)
    return header + payload, payload


def reserve_port() -> int:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as listener:
        listener.bind(("127.0.0.1", 0))
        return int(listener.getsockname()[1])


def connect_when_ready(port: int, process: subprocess.Popen[bytes]) -> socket.socket:
    deadline = time.monotonic() + 10.0
    while time.monotonic() < deadline:
        if process.poll() is not None:
            stdout, stderr = process.communicate()
            raise RuntimeError(
                f"deduction server exited early ({process.returncode}):\n"
                f"stdout={stdout!r}\nstderr={stderr!r}"
            )
        try:
            return socket.create_connection(("127.0.0.1", port), timeout=0.25)
        except OSError:
            time.sleep(0.05)
    raise TimeoutError("deduction server did not accept connections")


def capture(server: Path, prover: Path) -> dict[str, object]:
    port = reserve_port()
    process = subprocess.Popen(
        [str(server), "-p", str(port), str(prover)],
        cwd=server.parent,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        start_new_session=True,
    )
    frames: list[dict[str, object]] = []
    try:
        with connect_when_ready(port, process) as stream:
            stream.settimeout(20.0)
            for payload in (
                b"RUN reference_job",
                b"fof(reference_axiom, axiom, p(a)).\n",
                b"GO\n",
            ):
                stream.sendall(pack_frame(payload))

            while True:
                wire, payload = recv_frame(stream)
                frames.append(
                    {
                        "index": len(frames),
                        "payload_length": len(payload),
                        "payload": payload.decode("utf-8", errors="backslashreplace"),
                        "wire_hex": wire.hex(),
                    }
                )
                if payload == SUCCESS:
                    break
                if len(frames) >= 16:
                    raise RuntimeError("RUN response exceeded 16 TCP frames")

            stream.sendall(pack_frame(b"QUIT"))
    finally:
        if process.poll() is None:
            os.killpg(process.pid, signal.SIGTERM)
        stdout, stderr = process.communicate(timeout=5.0)

    return {
        "command": "RUN reference_job",
        "input_frames": [
            "RUN reference_job",
            "fof(reference_axiom, axiom, p(a)).\n",
            "GO\n",
        ],
        "response_frames": frames,
        "server_stdout": stdout.decode("utf-8", errors="backslashreplace"),
        "server_stderr": stderr.decode("utf-8", errors="backslashreplace"),
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--server", required=True, type=Path)
    parser.add_argument("--prover", required=True, type=Path)
    args = parser.parse_args()
    print(json.dumps(capture(args.server.resolve(), args.prover.resolve()), indent=2))


if __name__ == "__main__":
    main()
