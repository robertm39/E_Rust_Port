#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
cd "$repo_root"

experiment=2026-07-17-025-equality-resolution-branching-csu
artifact_dir=.artifacts/experiments/$experiment/trace
manifest=$HOME/.cache/e-rust-port/reference.json
input=$repo_root/experiments/$experiment/input.p
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
    local status
    set +e
    "$executable" "${arguments[@]}" "$input" \
        >"$artifact_dir/$label.out" 2>"$artifact_dir/$label.err"
    status=$?
    set -e
    if [[ $status -ne 9 ]]; then
        printf '%s returned %d, expected processed-clause limit status 9\n' \
            "$label" "$status" >&2
        exit 1
    fi
    grep 'inference(er' "$artifact_dir/$label.out" \
        | sed -E 's/c_0_[0-9]+/c_0_N/g' \
        >"$artifact_dir/$label-resolvents.normalized"
    grep -F '% Equation resolutions' "$artifact_dir/$label.out" \
        >"$artifact_dir/$label-count.txt"
}

run_one c "$c_binary"
run_one rust "$rust_binary"

diff -u "$artifact_dir/c-resolvents.normalized" "$artifact_dir/rust-resolvents.normalized"
grep -Eq ': +2$' "$artifact_dir/c-count.txt"
grep -Eq ': +2$' "$artifact_dir/rust-count.txt"

cat "$artifact_dir/rust-resolvents.normalized"
cat "$artifact_dir/c-count.txt"
cat "$artifact_dir/rust-count.txt"
