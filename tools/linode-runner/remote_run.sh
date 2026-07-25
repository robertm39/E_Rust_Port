#!/usr/bin/env bash
set -Eeuo pipefail

if (($# != 4)); then
    echo "usage: remote_run.sh SOURCE_ROOT ARTIFACT_ROOT JOBS EPROVER_COMMIT" >&2
    exit 2
fi

source_root="$(realpath "$1")"
artifact_root="$2"
jobs="$3"
eprover_commit="$4"

if [[ "$source_root" != /opt/e-rust-port/source ]]; then
    echo "refusing unexpected source root: $source_root" >&2
    exit 2
fi
if [[ "$artifact_root" != /opt/e-rust-port/artifacts/e-rust-codex-* ]]; then
    echo "refusing unexpected artifact root: $artifact_root" >&2
    exit 2
fi
if [[ ! "$jobs" =~ ^[1-9][0-9]*$ ]]; then
    echo "invalid job count: $jobs" >&2
    exit 2
fi
if [[ ! "$eprover_commit" =~ ^[A-Za-z0-9._-]+$ ]]; then
    echo "invalid E commit identifier: $eprover_commit" >&2
    exit 2
fi

export PATH="/root/.cargo/bin:$PATH"
export CARGO_TERM_COLOR=never
export RUST_BACKTRACE=1

mkdir -p "$artifact_root"
exec > >(tee "$artifact_root/runner.log") 2>&1

echo "== Environment =="
date --iso-8601=seconds
uname -a
rustc --version
cargo --version
gcc --version | head -n 1
valgrind --version
df -h /opt/e-rust-port

echo "== Rust release build =="
cd "$source_root"
/usr/bin/time -v -o "$artifact_root/rust-build-time.txt" \
    cargo build --locked --release --bin eprover 2>&1 |
    tee "$artifact_root/rust-build.txt"
rust_binary="$source_root/target/release/eprover"
test -x "$rust_binary"

echo "== C release build =="
c_build="/opt/e-rust-port/c-build/$(basename "$artifact_root")"
mkdir -p "$(dirname "$c_build")"
cp -a "$source_root/eprover" "$c_build"
python3 - "$c_build" <<'PY'
import sys
from pathlib import Path

root = Path(sys.argv[1])
for candidate in root.rglob("*"):
    if not candidate.is_file():
        continue
    data = candidate.read_bytes()
    if b"\0" in data or b"\r\n" not in data:
        continue
    candidate.write_bytes(data.replace(b"\r\n", b"\n"))
PY
while IFS= read -r -d '' candidate; do
    if [[ "$(head -c 2 "$candidate" || true)" == '#!' ]]; then
        chmod u+x "$candidate"
    fi
done < <(find "$c_build" -type f -print0)
cd "$c_build"
bash ./configure 2>&1 | tee "$artifact_root/c-configure.txt"
printf '#define ECOMMITID "%s"\n' "$eprover_commit" >PROVER/e_gitcommit.h
/usr/bin/time -v -o "$artifact_root/c-build-time.txt" \
    make -j "$jobs" 2>&1 |
    tee "$artifact_root/c-build.txt"
c_binary="$c_build/PROVER/eprover"
test -x "$c_binary"

problem="$c_build/EXAMPLE_PROBLEMS/SMOKETEST/socrates.p"
test -f "$problem"
export TPTP="$(dirname "$problem")"
common_args=("$problem" --auto --silent --cpu-limit=10)

echo "== Native smoke runs =="
/usr/bin/time -v "$rust_binary" "${common_args[@]}" \
    >"$artifact_root/rust-smoke.stdout" \
    2>"$artifact_root/rust-smoke.stderr"
/usr/bin/time -v "$c_binary" "${common_args[@]}" \
    >"$artifact_root/c-smoke.stdout" \
    2>"$artifact_root/c-smoke.stderr"
grep -q 'SZS status' "$artifact_root/rust-smoke.stdout"
grep -q 'SZS status' "$artifact_root/c-smoke.stdout"

echo "== Callgrind smoke runs =="
valgrind --tool=callgrind \
    --callgrind-out-file="$artifact_root/callgrind-rust.out" \
    "$rust_binary" "${common_args[@]}" \
    >"$artifact_root/callgrind-rust.stdout" \
    2>"$artifact_root/callgrind-rust.stderr"
valgrind --tool=callgrind \
    --callgrind-out-file="$artifact_root/callgrind-c.out" \
    "$c_binary" "${common_args[@]}" \
    >"$artifact_root/callgrind-c.stdout" \
    2>"$artifact_root/callgrind-c.stderr"
callgrind_annotate --inclusive=yes --threshold=0.1 \
    "$artifact_root/callgrind-rust.out" \
    >"$artifact_root/callgrind-rust.txt"
callgrind_annotate --inclusive=yes --threshold=0.1 \
    "$artifact_root/callgrind-c.out" \
    >"$artifact_root/callgrind-c.txt"

echo "== Results =="
sha256sum "$rust_binary" "$c_binary" >"$artifact_root/binary-sha256.txt"
{
    printf 'rust='
    awk '/summary:/ {print $2}' "$artifact_root/callgrind-rust.out"
    printf 'c='
    awk '/summary:/ {print $2}' "$artifact_root/callgrind-c.out"
} >"$artifact_root/callgrind-instructions.txt"
printf 'ok\n' >"$artifact_root/SUCCESS"
cat "$artifact_root/callgrind-instructions.txt"
du -sh "$artifact_root"
