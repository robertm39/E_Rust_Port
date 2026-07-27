#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 4 ]]; then
    echo "usage: benchmark-phases.sh C_BINARY RUST_BINARY PROBLEM OUTPUT_DIR" >&2
    exit 2
fi

c_binary=$1
rust_binary=$2
problem=$3
output_dir=$4
mkdir -p "$output_dir"
metrics="$output_dir/phase-metrics.csv"
statuses="$output_dir/phase-status.csv"
: >"$metrics"
: >"$statuses"

run_one() {
    local implementation=$1
    local binary=$2
    local mode=$3
    local run=$4
    shift 4

    local output
    local timing
    local exit_code
    local status
    output=$(mktemp)
    timing=$(mktemp)
    if /usr/bin/time \
        -f "$implementation,$mode,$run,%x,%e,%U,%S,%M" \
        -o "$timing" \
        "$binary" "$problem" "$@" >"$output"; then
        exit_code=0
    else
        exit_code=$?
    fi
    cat "$timing" >>"$metrics"
    status=$(grep -m1 -E '^[#%] SZS status ' "$output" || true)
    printf '%s,%s,%s,%s,%s\n' \
        "$implementation" "$mode" "$run" "$exit_code" "$status" >>"$statuses"
    rm -f "$output" "$timing"
}

for mode in syntax cnf auto; do
    case "$mode" in
        syntax)
            args=(--syntax-only --silent)
            ;;
        cnf)
            args=(--cnf --silent)
            ;;
        auto)
            args=(
                --auto
                --silent
                --cpu-limit=60
                --memory-limit=2048
                --detsort-rw
                --detsort-new
            )
            ;;
    esac

    for run in 1 2 3 4 5; do
        if ((run % 2 == 1)); then
            order=(c rust)
        else
            order=(rust c)
        fi
        for implementation in "${order[@]}"; do
            if [[ "$implementation" == c ]]; then
                run_one c "$c_binary" "$mode" "$run" "${args[@]}"
            else
                run_one rust "$rust_binary" "$mode" "$run" "${args[@]}"
            fi
        done
    done
done

cat "$metrics"
cat "$statuses"
