#!/usr/bin/env bash
set -euo pipefail

cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --lib \
  learn::kbinsert::tests::parse_example_clause_skips_pattern_search_over_branch_limit \
  -- --exact
cargo test --locked --all-targets --all-features
