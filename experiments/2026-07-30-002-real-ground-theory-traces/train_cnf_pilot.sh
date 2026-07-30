#!/usr/bin/env bash
set -Eeuo pipefail

repo_root=${1:-/opt/e-rust-port/source}
result_root=${2:-/opt/e-rust-port/artifacts/train-pilot}

cd "$repo_root"
. /root/.cargo/env
tar -xzf /root/train-pilot.tar.gz
cargo build --locked --release --bin umlaut
mkdir -p "$result_root"

problems=(
  problems/casc_2025/TFI/SWW667_2.p
  problems/casc_2025/TFE/ITP348_1.p
  problems/casc_2025/TFI/HWV050_6.p
  problems/casc_2025/TFI/SYO522_1.p
)

for problem in "${problems[@]}"; do
  name=$(basename "$problem" .p)
  set +e
  /usr/bin/time -v target/release/umlaut --cnf --tstp-out "$problem" \
    >"$result_root/$name.stdout" \
    2>"$result_root/$name.stderr"
  return_code=$?
  set -e
  clause_count=$(grep -Ec '^(cnf|tcf)\(' "$result_root/$name.stdout" || true)
  arithmetic_count=$(
    grep -Eo '\$(less|lesseq|greater|greatereq|sum|difference|product|quotient)' \
      "$result_root/$name.stdout" |
      wc -l
  )
  printf '%s\t%s\t%s\t%s\n' \
    "$name" "$return_code" "$clause_count" "$arithmetic_count"
done
