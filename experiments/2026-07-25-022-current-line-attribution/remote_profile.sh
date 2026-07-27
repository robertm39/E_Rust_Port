#!/usr/bin/env bash
set -Eeuo pipefail

if (($# != 2)); then
    echo "usage: remote_profile.sh SOURCE_ROOT ARTIFACT_ROOT" >&2
    exit 64
fi

source_root=$1
artifact_root=$2
measure_root=/opt/e-rust-port/measure
normal_target="$measure_root/normal-target"
lines_target="$measure_root/lines-target"
normal_bin="$measure_root/normal-eprover"
lines_bin="$measure_root/lines-eprover"
problem="$source_root/eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop"

mkdir -p "$artifact_root" "$measure_root"
cd "$source_root"

sha256sum src/clauses/pdtrees.rs src/terms/termbanks.rs \
    src/terms/subst.rs src/terms/termtypes.rs |
    tee "$artifact_root/profiled-source-sha256.txt"

echo "== Build normal release =="
CARGO_TARGET_DIR="$normal_target" \
    /root/.cargo/bin/cargo build --locked --release --bin eprover
cp "$normal_target/release/eprover" "$normal_bin"

echo "== Build line-table release =="
CARGO_PROFILE_RELEASE_DEBUG=line-tables-only \
    CARGO_TARGET_DIR="$lines_target" \
    /root/.cargo/bin/cargo build --locked --release --bin eprover
cp "$lines_target/release/eprover" "$lines_bin"

sha256sum "$normal_bin" "$lines_bin" |
    tee "$artifact_root/binary-sha256.txt"
stat --printf='%n,%s\n' "$normal_bin" "$lines_bin" |
    tee "$artifact_root/binary-size.csv"

common_args=(
    "$problem"
    --auto
    --silent
    --cpu-limit=600
    --memory-limit=2048
    --detsort-rw
    --detsort-new
)

profile()
{
    local label=$1
    local binary=$2

    valgrind --tool=callgrind \
        --log-file="$artifact_root/callgrind-$label.log" \
        --callgrind-out-file="$artifact_root/callgrind-$label.out" \
        "$binary" "${common_args[@]}" \
        >"$artifact_root/$label.stdout" \
        2>"$artifact_root/$label.stderr"
    callgrind_annotate --tree=both --threshold=95 \
        "$artifact_root/callgrind-$label.out" \
        >"$artifact_root/callgrind-$label-tree.txt"
}

echo "== Exact normal release profile =="
profile normal "$normal_bin"
echo "== Exact line-table release profile =="
profile lines "$lines_bin"

callgrind_annotate --inclusive=no --threshold=100 --auto=no \
    "$artifact_root/callgrind-lines.out" \
    >"$artifact_root/callgrind-lines-self.txt"
callgrind_annotate --inclusive=yes --threshold=100 --auto=no \
    "$artifact_root/callgrind-lines.out" \
    >"$artifact_root/callgrind-lines-inclusive.txt"

{
    printf 'normal='
    awk '/^summary:/{print $2}' "$artifact_root/callgrind-normal.out"
    printf 'lines='
    awk '/^summary:/{print $2}' "$artifact_root/callgrind-lines.out"
} | tee "$artifact_root/callgrind-instructions.txt"

cmp "$artifact_root/normal.stdout" "$artifact_root/lines.stdout"
test ! -s "$artifact_root/normal.stderr"
test ! -s "$artifact_root/lines.stderr"
sha256sum "$artifact_root/normal.stdout" "$artifact_root/lines.stdout" |
    tee "$artifact_root/proof-sha256.txt"
