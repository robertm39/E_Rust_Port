#!/usr/bin/env bash
set -euo pipefail

cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --lib \
  heuristics::clausefeatures::tests::variable_distribution_rejects_zero_code_db_variable_like_c_assertion \
  -- --exact
cargo test --locked --all-targets --all-features
