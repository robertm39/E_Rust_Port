#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 4 ]]; then
    echo "usage: benchmark-scaling.sh C_BINARY RUST_BINARY CORPUS_DIR OUTPUT_DIR" >&2
    exit 2
fi

c_binary=$1
rust_binary=$2
corpus_dir=$3
output_dir=$4
mkdir -p "$output_dir"
metrics="$output_dir/scaling-metrics.csv"
statuses="$output_dir/scaling-status.csv"
: >"$metrics"
: >"$statuses"

run_one() {
    local shape=$1
    local count=$2
    local implementation=$3
    local binary=$4
    local phase=$5
    local run=$6
    local problem=$7
    shift 7

    local output
    local timing
    local exit_code
    local status
    output=$(mktemp)
    timing=$(mktemp)
    if /usr/bin/time \
        -f "$shape,$count,$implementation,$phase,$run,%x,%e,%U,%S,%M" \
        -o "$timing" \
        "$binary" "$problem" "$@" >"$output"; then
        exit_code=0
    else
        exit_code=$?
    fi
    cat "$timing" >>"$metrics"
    status=$(grep -m1 -E '^[#%] SZS status ' "$output" || true)
    printf '%s,%s,%s,%s,%s,%s,%s\n' \
        "$shape" "$count" "$implementation" "$phase" "$run" "$exit_code" "$status" \
        >>"$statuses"
    rm -f "$output" "$timing"
}

shopt -s nullglob
problems=("$corpus_dir"/*.p)
if [[ ${#problems[@]} -eq 0 ]]; then
    echo "no .p files found in $corpus_dir" >&2
    exit 2
fi

for problem in "${problems[@]}"; do
    stem=$(basename "$problem" .p)
    shape=${stem%-*}
    count=${stem##*-}
    count=$((10#$count))
    for phase in syntax cnf; do
        if [[ "$phase" == syntax ]]; then
            args=(--syntax-only --silent)
        else
            args=(--cnf --silent)
        fi
        for run in 1 2 3; do
            if ((run % 2 == 1)); then
                order=(c rust)
            else
                order=(rust c)
            fi
            for implementation in "${order[@]}"; do
                if [[ "$implementation" == c ]]; then
                    run_one "$shape" "$count" c "$c_binary" "$phase" "$run" "$problem" "${args[@]}"
                else
                    run_one "$shape" "$count" rust "$rust_binary" "$phase" "$run" "$problem" "${args[@]}"
                fi
            done
        done
    done
done

cat "$metrics"
cat "$statuses"
