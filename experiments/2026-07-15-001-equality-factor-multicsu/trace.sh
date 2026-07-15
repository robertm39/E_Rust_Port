#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
cd "$repo_root"

artifact_dir=.artifacts/experiments/2026-07-15-001-equality-factor-multicsu/trace
manifest=$HOME/.cache/e-rust-port/reference.json
input=$repo_root/experiments/2026-07-15-001-equality-factor-multicsu/input.p
mkdir -p "$artifact_dir"

mapfile -t binaries < <(python3 - "$manifest" <<'PY'
import json
import pathlib
import sys

manifest_path = pathlib.Path(sys.argv[1])
manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
commit = manifest["upstream_commit"]
print(manifest["builds"]["ho"]["binary"])
print(pathlib.Path.home() / ".cache" / "e-rust-port" / "rust-target" / commit / "release" / "eprover")
PY
)

c_binary=${binaries[0]}
rust_binary=${binaries[1]}
for binary in "$c_binary" "$rust_binary"; do
    if [[ ! -x "$binary" ]]; then
        printf 'missing executable: %s\n' "$binary" >&2
        printf 'run .\\e-interop.ps1 benchmark -Corpus experiments\\2026-07-15-001-equality-factor-multicsu -Runs 3 first\n' >&2
        exit 2
    fi
done

arguments=(
    --unif-mode=multi
    --pattern-oracle=false
    --fixpoint-oracle=false
    --func-proj-limit=1
    --imit-limit=1
    --max-unifiers=4
    --max-unif-steps=32
    --output-level=2
    --processed-clauses-limit=1
)

run_one() {
    local label=$1
    local executable=$2
    local expected_status=$3
    local status
    set +e
    "$executable" "${arguments[@]}" "$input" \
        >"$artifact_dir/$label.out" 2>"$artifact_dir/$label.err"
    status=$?
    set -e
    if [[ $status -ne $expected_status ]]; then
        printf '%s returned %d, expected processed-clause limit status %d\n' \
            "$label" "$status" "$expected_status" >&2
        exit 1
    fi
    grep 'inference(ef' "$artifact_dir/$label.out" \
        | sed -E 's/c_0_[0-9]+/c_0_N/g' \
        >"$artifact_dir/$label-factors.normalized"
    grep -F '% Factorizations' "$artifact_dir/$label.out" \
        >"$artifact_dir/$label-factor-count.txt"
}

run_one c "$c_binary" 9
run_one rust "$rust_binary" 9

diff -u "$artifact_dir/c-factors.normalized" "$artifact_dir/rust-factors.normalized"
grep -Eq ': +2$' "$artifact_dir/c-factor-count.txt"
grep -Eq ': +2$' "$artifact_dir/rust-factor-count.txt"

cat "$artifact_dir/rust-factors.normalized"
cat "$artifact_dir/c-factor-count.txt"
cat "$artifact_dir/rust-factor-count.txt"
