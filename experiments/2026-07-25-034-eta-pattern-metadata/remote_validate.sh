#!/usr/bin/env bash
set -euo pipefail

cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --lib eta_normalizable -- --nocapture
cargo test --locked --all-targets --all-features
