#!/usr/bin/env bash
set -euo pipefail

artifact_root=${1:-/opt/e-rust-port/artifacts/sat-service}
source_root=${2:-/opt/e-rust-port/source}
output=${3:-"$artifact_root/results/build-packaging.txt"}
experiment="$source_root/experiments/2026-07-28-012-incremental-sat-service"
umlaut_revision=${UMLAUT_REVISION:-unknown-exported-snapshot}

mkdir -p "$(dirname "$output")"
temp=$(mktemp -d)
trap 'rm -rf "$temp"' EXIT

{
  echo "SYSTEM"
  uname -a
  lscpu | grep -E '^(Architecture|CPU\(s\)|Model name|Thread|Core|Socket)'
  echo
  echo "TOOLCHAINS"
  g++ --version | head -1
  gcc --version | head -1
  x86_64-w64-mingw32-g++-posix --version | head -1
  rustc --version
  python3 --version
  dpkg-query -W \
    g++-mingw-w64-x86-64 libz-mingw-w64-dev zlib1g-dev \
    2>/dev/null
  echo
  echo "SOURCE IDENTITIES"
  echo "Umlaut base revision: $umlaut_revision"
  printf 'CaDiCaL version: '
  cat "$artifact_root/src/cadical/VERSION"
  printf 'PicoSAT version: '
  cat "$artifact_root/src/picosat/VERSION"
  echo "MiniSat revision: 37dc6c67e2af26379d88ce349eb9c4c6160e8543"
  sha256sum \
    "$experiment/adapter.cpp" \
    "$experiment/internal_adapter.rs" \
    "$experiment/internal-adapter.patch" \
    "$experiment/capture-satcheck.patch"
  echo
  echo "SOURCE ARCHIVES"
  sha256sum \
    "/opt/e-rust-port/artifacts/cadical-3.0.1-source.tar.gz" \
    "/opt/e-rust-port/artifacts/minisat-37dc6c6.tar.gz"
  echo
  echo "LICENSES"
  sha256sum \
    "$artifact_root/src/cadical/LICENSE" \
    "$artifact_root/src/minisat/LICENSE" \
    "$artifact_root/src/picosat/LICENSE"
  echo
  echo "ADAPTER FILES"
  adapters=(
    cadical-adapter
    internal-dpll-adapter
    minisat-adapter
    picosat-adapter
    cadical-adapter-static
    minisat-adapter-static
    picosat-adapter-static
    cadical-adapter-windows.exe
    minisat-adapter-windows.exe
    picosat-adapter-windows.exe
  )
  for name in "${adapters[@]}"; do
    path="$artifact_root/bin/$name"
    echo
    echo "[$name]"
    file "$path"
    sha256sum "$path"
    wc -c "$path"
    if [[ "$name" != *.exe ]]; then
      size "$path"
      ldd "$path" 2>&1 || true
    fi
  done
  echo
  echo "STRIPPED DYNAMIC ADAPTERS"
  for name in cadical-adapter internal-dpll-adapter minisat-adapter picosat-adapter; do
    cp "$artifact_root/bin/$name" "$temp/$name"
    strip "$temp/$name"
    sha256sum "$temp/$name"
    wc -c "$temp/$name"
    size "$temp/$name"
  done
  echo
  echo "CAPABILITY RECORDS"
  for name in cadical-adapter internal-dpll-adapter minisat-adapter picosat-adapter; do
    "$artifact_root/bin/$name" \
      "$artifact_root/workloads/semantic/assumption_core.isat" | head -1
  done
} > "$output"

cat "$output"
