#!/usr/bin/env bash
set -Eeuo pipefail

if (($# != 2)); then
    echo "usage: remote_measure.sh SOURCE_ROOT ARTIFACT_ROOT" >&2
    exit 64
fi

source_root=$1
artifact_root=$2
measure_pairs="$source_root/experiments/2026-07-25-001-inline-term-arity-lto/measure_pairs.py"
analyze_pairs="$source_root/experiments/2026-07-25-001-inline-term-arity-lto/analyze_pairs.py"
measure_root=/opt/e-rust-port/measure
candidate_target="$measure_root/candidate-target"
parent_target="$measure_root/parent-target"
candidate_bin="$measure_root/candidate-eprover"
parent_bin="$measure_root/parent-eprover"
problem="$source_root/eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop"
accepted_archive="$source_root/accepted-source.tar"
candidate_source="$artifact_root/candidate-source"
accepted_source="$artifact_root/accepted-source"
production_files=(src/terms/termtypes.rs)

mkdir -p "$artifact_root" "$measure_root" "$candidate_source" "$accepted_source"
cd "$source_root"

test -f "$accepted_archive"
tar -xf "$accepted_archive" -C "$accepted_source"
for path in "${production_files[@]}"; do
    mkdir -p "$candidate_source/$(dirname "$path")"
    cp "$source_root/$path" "$candidate_source/$path"
done
trap 'for path in "${production_files[@]}"; do cp "$candidate_source/$path" "$source_root/$path"; done' EXIT
sha256sum "${production_files[@]}" |
    tee "$artifact_root/candidate-source-sha256.txt"

echo "== Candidate focused gates =="
/root/.cargo/bin/cargo fmt --all -- --check
/root/.cargo/bin/cargo test --locked --lib --all-features -j1 \
    terms::termtypes::tests::borrowed_first_order_argument_push_preserves_all_arity_shapes \
    -- --test-threads=1
/root/.cargo/bin/cargo test --locked --lib --all-features -j1 \
    orderings::cto_kbolin::tests -- --test-threads=1
/root/.cargo/bin/cargo clippy --locked --all-targets --all-features -j1 -- \
    -D warnings -D clippy::pedantic

echo "== Build candidate =="
CARGO_TARGET_DIR="$candidate_target" \
    /root/.cargo/bin/cargo build --locked --release --bin eprover
cp "$candidate_target/release/eprover" "$candidate_bin"

echo "== Restore accepted parent source =="
for path in "${production_files[@]}"; do
    cp "$accepted_source/$path" "$source_root/$path"
done

echo "== Build parent =="
CARGO_TARGET_DIR="$parent_target" \
    /root/.cargo/bin/cargo build --locked --release --bin eprover
cp "$parent_target/release/eprover" "$parent_bin"

for path in "${production_files[@]}"; do
    cp "$candidate_source/$path" "$source_root/$path"
    cmp "$candidate_source/$path" "$source_root/$path"
done

sha256sum "$parent_bin" "$candidate_bin" |
    tee "$artifact_root/binary-sha256.txt"
stat --printf='%n,%s\n' "$parent_bin" "$candidate_bin" |
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
        >"$artifact_root/callgrind-$label.stdout" \
        2>"$artifact_root/callgrind-$label.stderr"
    callgrind_annotate --tree=both --threshold=95 \
        "$artifact_root/callgrind-$label.out" \
        >"$artifact_root/callgrind-$label-tree.txt"
}

echo "== Exact parent Callgrind =="
profile parent "$parent_bin"
echo "== Exact candidate Callgrind =="
profile candidate "$candidate_bin"

{
    printf 'parent='
    awk '/^summary:/{print $2}' "$artifact_root/callgrind-parent.out"
    printf 'candidate='
    awk '/^summary:/{print $2}' "$artifact_root/callgrind-candidate.out"
} | tee "$artifact_root/callgrind-instructions.txt"
cmp "$artifact_root/callgrind-parent.stdout" \
    "$artifact_root/callgrind-candidate.stdout"
test ! -s "$artifact_root/callgrind-parent.stderr"
test ! -s "$artifact_root/callgrind-candidate.stderr"
sha256sum \
    "$artifact_root/callgrind-parent.stdout" \
    "$artifact_root/callgrind-candidate.stdout" |
    tee "$artifact_root/callgrind-proof-sha256.txt"

echo "== Alternating native measurements =="
python3 "$measure_pairs" \
    --parent "$parent_bin" \
    --candidate "$candidate_bin" \
    --problem "$problem" \
    --warmups 4 \
    --pairs 64 \
    --warmup-csv "$artifact_root/native-warmup.csv" \
    --measurement-csv "$artifact_root/native-lusk.csv"
python3 "$analyze_pairs" \
    "$artifact_root/native-lusk.csv" \
    --output "$artifact_root/native-summary.json"
cat "$artifact_root/native-summary.json"
