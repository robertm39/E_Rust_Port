#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
cd "$repo_root"

experiment=2026-07-15-003-lfho-paramod-direct-mgu
artifact_dir=.artifacts/experiments/$experiment/ordering-boundary
manifest=$HOME/.cache/e-rust-port/reference.json
input=$repo_root/experiments/$experiment/rigid-prefix.p
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

for ordering in KBO LPO LPOCopy LPO4Copy; do
    for implementation in c rust; do
        if [[ $implementation == c ]]; then
            executable=${binaries[0]}
        else
            executable=${binaries[1]}
        fi
        prefix=$artifact_dir/$ordering-$implementation
        set +e
        "$executable" \
            --term-ordering="$ordering" \
            --literal-selection-strategy=NoSelection \
            --pm-from-index=NoIndex \
            --pm-into-index=NoIndex \
            --processed-clauses-limit=2 \
            --output-level=2 \
            --print-statistics \
            "$input" >"$prefix.out" 2>"$prefix.err"
        status=$?
        set -e
        printf '%s\t%s\t%d\n' "$ordering" "$implementation" "$status"
        sed -n '1,5p' "$prefix.err"
        grep -E '^% (Processed clauses|Generated clauses|Paramodulations)' "$prefix.out" || true
    done
done
