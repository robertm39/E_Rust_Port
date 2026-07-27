#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 6 ]]; then
    echo "usage: benchmark-recursive-tsm-wsl.sh REFERENCE RUST INPUT OUTPUT_CSV RUNS ITERATIONS" >&2
    exit 2
fi

reference_source=$1
rust_source=$2
input=$3
output_csv=$4
runs=$5
iterations=$6
arguments=(--index-type=IndexSymbol --index-depth=3 --tsm-type=Recursive)

native_bin_dir=$(mktemp -d)
trap 'rm -rf "$native_bin_dir"' EXIT
install -m 755 "$reference_source" "$native_bin_dir/reference-tsm-classify"
install -m 755 "$rust_source" "$native_bin_dir/rust-tsm-classify"
reference=$native_bin_dir/reference-tsm-classify
rust=$native_bin_dir/rust-tsm-classify

mkdir -p "$(dirname "$output_csv")"
printf 'implementation,run,iterations,exit_code,wall_seconds,user_seconds,system_seconds,max_rss_kib\n' >"$output_csv"

for ((run = 1; run <= runs; run++)); do
    if ((run % 2 == 0)); then
        implementations=(rust reference)
    else
        implementations=(reference rust)
    fi
    for implementation in "${implementations[@]}"; do
        if [[ $implementation == reference ]]; then
            binary=$reference
        else
            binary=$rust
        fi
        timing=$(mktemp)
        /usr/bin/time \
            -f "$implementation,$run,$iterations,%x,%e,%U,%S,%M" \
            -o "$timing" \
            bash -c '
                binary=$1
                input=$2
                iterations=$3
                shift 3
                for ((iteration = 0; iteration < iterations; iteration++)); do
                    "$binary" "$@" <"$input" >/dev/null
                done
            ' benchmark-batch "$binary" "$input" "$iterations" "${arguments[@]}"
        cat "$timing" >>"$output_csv"
        rm -f "$timing"
    done
done

cat "$output_csv"
