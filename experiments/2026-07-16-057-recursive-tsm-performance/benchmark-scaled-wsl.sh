#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 5 ]]; then
    echo "usage: benchmark-scaled-wsl.sh REFERENCE RUST INPUT OUTPUT_CSV RUNS [CLASSIFIER_ARG ...]" >&2
    exit 2
fi

reference_source=$1
rust_source=$2
input=$3
output_csv=$4
runs=$5
shift 5
if [[ $# -eq 0 ]]; then
    arguments=(--index-type=IndexSymbol --index-depth=3 --tsm-type=Recursive)
else
    arguments=("$@")
fi

native_bin_dir=$(mktemp -d)
trap 'rm -rf "$native_bin_dir"' EXIT
install -m 755 "$reference_source" "$native_bin_dir/reference-tsm-classify"
install -m 755 "$rust_source" "$native_bin_dir/rust-tsm-classify"

mkdir -p "$(dirname "$output_csv")"
printf 'implementation,run,exit_code,wall_seconds,user_seconds,system_seconds,max_rss_kib\n' >"$output_csv"

for ((run = 1; run <= runs; run++)); do
    if ((run % 2 == 0)); then
        implementations=(rust reference)
    else
        implementations=(reference rust)
    fi
    for implementation in "${implementations[@]}"; do
        binary=$native_bin_dir/$implementation-tsm-classify
        timing=$(mktemp)
        /usr/bin/time \
            -f "$implementation,$run,%x,%e,%U,%S,%M" \
            -o "$timing" \
            "$binary" "${arguments[@]}" <"$input" >/dev/null
        cat "$timing" >>"$output_csv"
        rm -f "$timing"
    done
done

cat "$output_csv"
