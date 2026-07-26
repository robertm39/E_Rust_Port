#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/../.."

cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings -D clippy::pedantic
cargo test --locked --all-targets --all-features
cargo build --locked --release --bin eprover
python3 experiments/2026-07-25-036-strategy-io-timing/compare_strategy_timing.py \
    --repo . \
    --c-exe /root/.cache/e-rust-port/bin/worktree-snapshot/fol/eprover \
    --rust-exe target/release/eprover
