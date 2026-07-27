#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 4 ]]; then
    echo "usage: benchmark.sh LABEL BINARY PROBLEM OUTPUT_DIR" >&2
    exit 2
fi

label=$1
binary=$2
problem=$3
output_dir=$4
mkdir -p "$output_dir"
metrics="$output_dir/$label.csv"
status_file="$output_dir/$label.status"
: >"$metrics"
: >"$status_file"

args=(
    --auto
    --silent
    --cpu-limit=60
    --memory-limit=2048
    --detsort-rw
    --detsort-new
)

for run in 1 2 3 4 5; do
    output=$(mktemp)
    /usr/bin/time \
        -f "$run,%e,%U,%S,%M" \
        -o "$metrics.tmp" \
        "$binary" "$problem" "${args[@]}" >"$output"
    cat "$metrics.tmp" >>"$metrics"
    status=$(grep -m1 -E '^[#%] SZS status ' "$output" || true)
    printf '%s,%s\n' "$run" "$status" >>"$status_file"
    rm -f "$output" "$metrics.tmp"
done

cat "$metrics"
cat "$status_file"
