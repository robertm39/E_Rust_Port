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
export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc
export E_RUST_PORT_COMPAT_ROOT="/opt/e-rust-port/compat-cache/$(basename "$artifact_root")"
export E_RUST_PORT_COMPAT_ARTIFACT_ROOT="$artifact_root/compatibility"

mkdir -p "$artifact_root"
exec > >(tee "$artifact_root/runner.log") 2>&1

echo "== Environment =="
date --iso-8601=seconds
uname -a
rustc --version
cargo --version
cargo fmt --version
cargo clippy --version
gcc --version | head -n 1
x86_64-w64-mingw32-gcc --version | head -n 1
valgrind --version
df -h /opt/e-rust-port

echo "== Native Linux Rust quality gates =="
cd "$source_root"
/usr/bin/time -v -o "$artifact_root/rust-fmt-time.txt" \
    cargo fmt --all -- --check 2>&1 |
    tee "$artifact_root/rust-fmt.txt"
/usr/bin/time -v -o "$artifact_root/rust-test-time.txt" \
    cargo test --locked --all-targets --all-features 2>&1 |
    tee "$artifact_root/rust-test.txt"
/usr/bin/time -v -o "$artifact_root/rust-clippy-time.txt" \
    cargo clippy --locked --all-targets --all-features -- \
        -D warnings -D clippy::pedantic 2>&1 |
    tee "$artifact_root/rust-clippy.txt"
/usr/bin/time -v -o "$artifact_root/rust-build-time.txt" \
    cargo build --locked --release --bins 2>&1 |
    tee "$artifact_root/rust-build.txt"

rust_bin_dir="$source_root/target/release"
rust_binary="$rust_bin_dir/eprover"
test -x "$rust_binary"

echo "== Windows GNU x64 compile-only gates =="
/usr/bin/time -v -o "$artifact_root/windows-test-build-time.txt" \
    cargo test --locked --all-targets --all-features --no-run \
        --target x86_64-pc-windows-gnu 2>&1 |
    tee "$artifact_root/windows-test-build.txt"
/usr/bin/time -v -o "$artifact_root/windows-release-build-time.txt" \
    cargo build --locked --release --bins \
        --target x86_64-pc-windows-gnu 2>&1 |
    tee "$artifact_root/windows-release-build.txt"

windows_bin_dir="$source_root/target/x86_64-pc-windows-gnu/release"
windows_eprover="$windows_bin_dir/eprover.exe"
test -f "$windows_eprover"
find "$windows_bin_dir" -maxdepth 1 -type f -name '*.exe' -print0 |
    sort -z |
    xargs -0 file >"$artifact_root/windows-binaries-file.txt"
grep -q 'PE32+' "$artifact_root/windows-binaries-file.txt"
find "$windows_bin_dir" -maxdepth 1 -type f -name '*.exe' -print0 |
    sort -z |
    xargs -0 sha256sum >"$artifact_root/windows-binaries-sha256.txt"
printf 'Compiled only; no Windows binary was executed.\n' \
    >"$artifact_root/windows-compile-only.txt"

echo "== Native Linux C reference builds =="
compat_driver="$source_root/tools/linode-runner/linux_compat.py"
python3 "$compat_driver" doctor
/usr/bin/time -v -o "$artifact_root/c-reference-build-time.txt" \
    python3 "$compat_driver" build-reference \
        --repo-root "$source_root" \
        --eprover-commit "$eprover_commit" 2>&1 |
    tee "$artifact_root/c-reference-build.txt"
cp "$E_RUST_PORT_COMPAT_ROOT/reference.json" \
    "$artifact_root/c-reference-manifest.json"

c_fol_binary="$E_RUST_PORT_COMPAT_ROOT/bin/$eprover_commit/fol/eprover"
c_ho_binary="$E_RUST_PORT_COMPAT_ROOT/bin/$eprover_commit/ho/eprover-ho"
test -x "$c_fol_binary"
test -x "$c_ho_binary"

problem="$E_RUST_PORT_COMPAT_ROOT/sources/$eprover_commit/fol/EXAMPLE_PROBLEMS/SMOKETEST/socrates.p"
test -f "$problem"
export TPTP="$(dirname "$problem")"
common_args=("$problem" --auto --silent --cpu-limit=10)

echo "== Native Linux Rust/C smoke runs =="
/usr/bin/time -v "$rust_binary" "${common_args[@]}" \
    >"$artifact_root/rust-smoke.stdout" \
    2>"$artifact_root/rust-smoke.stderr"
/usr/bin/time -v "$c_fol_binary" "${common_args[@]}" \
    >"$artifact_root/c-smoke.stdout" \
    2>"$artifact_root/c-smoke.stderr"
grep -q 'SZS status' "$artifact_root/rust-smoke.stdout"
grep -q 'SZS status' "$artifact_root/c-smoke.stdout"

echo "== Native Linux main compatibility matrix =="
/usr/bin/time -v -o "$artifact_root/main-comparison-time.txt" \
    python3 "$compat_driver" compare \
        --repo-root "$source_root" \
        --rust-bin "$rust_binary" \
        --report-only 2>&1 |
    tee "$artifact_root/main-comparison.txt"

echo "== Native Linux support-tool compatibility matrix =="
/usr/bin/time -v -o "$artifact_root/tool-comparison-time.txt" \
    python3 "$compat_driver" compare-tools \
        --repo-root "$source_root" \
        --rust-bin-dir "$rust_bin_dir" \
        --report-only 2>&1 |
    tee "$artifact_root/tool-comparison.txt"

echo "== Native Linux timing benchmark =="
/usr/bin/time -v -o "$artifact_root/benchmark-time.txt" \
    python3 "$compat_driver" benchmark \
        --repo-root "$source_root" \
        --rust-bin "$rust_binary" \
        --runs 5 2>&1 |
    tee "$artifact_root/benchmark.txt"

echo "== Native Linux Callgrind smoke runs =="
valgrind --tool=callgrind \
    --callgrind-out-file="$artifact_root/callgrind-rust.out" \
    "$rust_binary" "${common_args[@]}" \
    >"$artifact_root/callgrind-rust.stdout" \
    2>"$artifact_root/callgrind-rust.stderr"
valgrind --tool=callgrind \
    --callgrind-out-file="$artifact_root/callgrind-c.out" \
    "$c_fol_binary" "${common_args[@]}" \
    >"$artifact_root/callgrind-c.stdout" \
    2>"$artifact_root/callgrind-c.stderr"
callgrind_annotate --inclusive=yes --threshold=0.1 \
    "$artifact_root/callgrind-rust.out" \
    >"$artifact_root/callgrind-rust.txt"
callgrind_annotate --inclusive=yes --threshold=0.1 \
    "$artifact_root/callgrind-c.out" \
    >"$artifact_root/callgrind-c.txt"

echo "== Results =="
sha256sum "$rust_binary" "$c_fol_binary" "$c_ho_binary" \
    >"$artifact_root/linux-binaries-sha256.txt"
{
    printf 'rust='
    awk '/summary:/ {print $2}' "$artifact_root/callgrind-rust.out"
    printf 'c='
    awk '/summary:/ {print $2}' "$artifact_root/callgrind-c.out"
} >"$artifact_root/callgrind-instructions.txt"

python3 - "$artifact_root" <<'PY'
import json
import sys
from pathlib import Path

root = Path(sys.argv[1])

def one(pattern):
    matches = list(root.glob(pattern))
    if len(matches) != 1:
        raise SystemExit(f"expected one {pattern}, found {len(matches)}")
    return json.loads(matches[0].read_text(encoding="utf-8"))

main = one("compatibility/main/*/comparison.json")
tools = one("compatibility/tools/*/tool-comparison.json")
benchmark = one("compatibility/benchmark/*/benchmark.json")
summary = {
    "schema_version": 1,
    "main_case_count": main["case_count"],
    "main_mismatch_count": main["mismatch_count"],
    "main_expected_difference_count": main["expected_difference_count"],
    "tool_case_count": tools["case_count"],
    "tool_mismatch_count": tools["mismatch_count"],
    "tool_expected_difference_count": tools["expected_difference_count"],
    "benchmark_case_count": len(benchmark["cases"]),
    "benchmark_behavior_mismatch_count": benchmark["behavior_mismatch_count"],
    "benchmark_rust_to_c_wall_ratio": benchmark[
        "aggregate_rust_to_c_wall_ratio"
    ],
}
summary["unexpected_compatibility_mismatch_count"] = (
    summary["main_mismatch_count"] + summary["tool_mismatch_count"]
)
(root / "validation-summary.json").write_text(
    json.dumps(summary, indent=2, sort_keys=True) + "\n",
    encoding="utf-8",
)
print(json.dumps(summary, indent=2, sort_keys=True))
PY

printf 'ok\n' >"$artifact_root/VALIDATION_COMPLETE"
cat "$artifact_root/callgrind-instructions.txt"
du -sh "$artifact_root"

mismatch_count="$(
    python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["unexpected_compatibility_mismatch_count"])' \
        "$artifact_root/validation-summary.json"
)"
if ((mismatch_count != 0)); then
    printf '%s\n' "$mismatch_count" >"$artifact_root/COMPATIBILITY_MISMATCHES"
    echo "validation completed with $mismatch_count unexpected compatibility mismatches" >&2
    exit 3
fi

printf 'ok\n' >"$artifact_root/SUCCESS"
