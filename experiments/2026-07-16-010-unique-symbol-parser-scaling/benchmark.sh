#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 5 ]]; then
    echo "usage: benchmark.sh C_BINARY BASELINE_RUST_BINARY CURRENT_RUST_BINARY CORPUS_DIR OUTPUT_CSV" >&2
    exit 2
fi

c_binary=$1
baseline_binary=$2
current_binary=$3
corpus_dir=$4
output_csv=$5
mkdir -p "$(dirname "$output_csv")"
: >"$output_csv"
shape=${SHAPE:-atom}
phase=${PHASE:-syntax}
case "$phase" in
    syntax) phase_args=(--syntax-only) ;;
    cnf) phase_args=(--cnf) ;;
    *) echo "PHASE must be syntax or cnf" >&2; exit 2 ;;
esac

IFS=',' read -r -a implementations <<<"${IMPLEMENTATIONS:-c,baseline,current}"
for implementation in "${implementations[@]}"; do
    case "$implementation" in
        c) binary=$c_binary ;;
        baseline) binary=$baseline_binary ;;
        current) binary=$current_binary ;;
        *) echo "unknown implementation: $implementation" >&2; exit 2 ;;
    esac
    for owners in 00100 01000 05000 10000 20000; do
        owner_count=$((10#$owners))
        problem="$corpus_dir/unique-$shape-$owners.p"
        for run in 1 2 3 4 5; do
            timing=$(mktemp)
            /usr/bin/time \
                -f "$implementation,$owner_count,$run,%x,%e,%U,%S,%M" \
                -o "$timing" \
                "$binary" "${phase_args[@]}" --silent --output-file=/dev/null "$problem"
            cat "$timing" >>"$output_csv"
            rm -f "$timing"
        done
    done
done

cat "$output_csv"
