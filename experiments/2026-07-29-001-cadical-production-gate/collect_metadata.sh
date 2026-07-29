#!/usr/bin/env bash
set -Eeuo pipefail

if (($# != 3)); then
    echo "usage: collect_metadata.sh SOURCE_ROOT CADICAL_SOURCE OUTPUT" >&2
    exit 2
fi

source_root="$(realpath "$1")"
cadical_source="$(realpath "$2")"
output="$3"
if [[ "$source_root" != /opt/e-rust-port/source ]]; then
    echo "unexpected source root: $source_root" >&2
    exit 2
fi
if [[ "$cadical_source" != /opt/e-rust-port/cadical-src ]]; then
    echo "unexpected CaDiCaL source: $cadical_source" >&2
    exit 2
fi

{
    date --iso-8601=seconds
    uname -a
    rustc --version
    cargo --version
    gcc --version | head -n 1
    c++ --version | head -n 1
    x86_64-w64-mingw32-gcc-posix --version | head -n 1
    x86_64-w64-mingw32-g++-posix --version | head -n 1
    printf 'cadical-version='
    cat "$cadical_source/VERSION"
    sha256sum \
        /opt/e-rust-port/cadical-c607304.tar.gz \
        /opt/e-rust-port/fresh-corpus.tar.gz \
        "$source_root/Cargo.toml" \
        "$source_root/build.rs" \
        "$source_root/src/clauses/satservice.rs" \
        "$source_root/src/clauses/cadical.rs" \
        "$source_root/src/clauses/satinterface.rs" \
        "$source_root/native/cadical_ffi/umlaut_cadical.h" \
        "$source_root/native/cadical_ffi/umlaut_cadical.cpp"
    sha256sum "$source_root"/experiments/2026-07-29-001-cadical-production-gate/*
} >"$output"
