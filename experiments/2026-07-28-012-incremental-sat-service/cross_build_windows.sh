#!/usr/bin/env bash
set -euo pipefail

artifact_root=${1:-/opt/e-rust-port/artifacts/sat-service}
source_root=${2:-/opt/e-rust-port/source}
adapter="$source_root/experiments/2026-07-28-012-incremental-sat-service/adapter.cpp"
cross_c=x86_64-w64-mingw32-gcc-posix
cross_cxx=x86_64-w64-mingw32-g++-posix
cross_ar=x86_64-w64-mingw32-ar

cadical_root="$artifact_root/src/cadical"
cadical_build="$cadical_root/winbuild"
rm -rf "$cadical_build"
mkdir "$cadical_build"
cp "$cadical_root/build/makefile" "$cadical_build/makefile"
sed -i \
  -e "s|^CXX=.*|CXX=$cross_cxx|" \
  -e "s|^CC=.*|CC=$cross_c|" \
  -e 's|^CXXFLAGS=|CXXFLAGS=-DNUNLOCKED |' \
  -e 's|^CFLAGS=|CFLAGS=-DNUNLOCKED |' \
  -e 's|^LIBS=.*|LIBS=-lwinpthread|' \
  -e 's|^CONTRIB=.*|CONTRIB=no|' \
  -e 's|^IPASIR=.*|IPASIR=no|' \
  -e "s|^\tar rc|\t$cross_ar rc|" \
  "$cadical_build/makefile"
make -C "$cadical_build" -j2 libcadical.a
"$cross_cxx" -std=c++17 -O3 -static -static-libgcc -static-libstdc++ \
  -DADAPTER_CADICAL -I"$cadical_root/src" \
  "$adapter" "$cadical_build/libcadical.a" -lwinpthread \
  -o "$artifact_root/bin/cadical-adapter-windows.exe"

picosat_root="$artifact_root/src/picosat"
picosat_build="$artifact_root/src/picosat-win"
rm -rf "$picosat_build"
mkdir "$picosat_build"
cp "$picosat_root/picosat.c" "$picosat_root/picosat.h" \
  "$picosat_root/version.c" "$picosat_root/config.h" "$picosat_build/"
"$cross_c" -O3 -DNDEBUG -DNGETRUSAGE -c "$picosat_build/picosat.c" \
  -o "$picosat_build/picosat.o"
"$cross_c" -O3 -DNDEBUG -c "$picosat_build/version.c" \
  -o "$picosat_build/version.o"
"$cross_ar" rcs "$picosat_build/libpicosat.a" \
  "$picosat_build/picosat.o" "$picosat_build/version.o"
"$cross_cxx" -std=c++17 -O3 -static -static-libgcc -static-libstdc++ \
  -DADAPTER_PICOSAT -I"$picosat_build" \
  "$adapter" "$picosat_build/libpicosat.a" -lwinpthread \
  -o "$artifact_root/bin/picosat-adapter-windows.exe"

minisat_source="$artifact_root/src/minisat"
minisat_root="$artifact_root/src/minisat-win"
rm -rf "$minisat_root"
cp -R "$minisat_source" "$minisat_root"
# MiniSat 2.2's fallback definition predates the defaulted bool parameter
# now declared in System.h. Modern MinGW diagnoses the mismatch.
sed -i \
  's/double Minisat::memUsedPeak() { return 0; }/double Minisat::memUsedPeak(bool) { return 0; }/' \
  "$minisat_root/minisat/utils/System.cc"
"$cross_cxx" -std=c++17 -O3 -fpermissive \
  -static -static-libgcc -static-libstdc++ \
  -D__STDC_LIMIT_MACROS -D__STDC_FORMAT_MACROS -DADAPTER_MINISAT \
  -I"$minisat_root" "$adapter" \
  "$minisat_root/minisat/core/Solver.cc" \
  "$minisat_root/minisat/utils/Options.cc" \
  "$minisat_root/minisat/utils/System.cc" \
  -lz -lwinpthread -o "$artifact_root/bin/minisat-adapter-windows.exe"

file "$artifact_root"/bin/*-adapter-windows.exe
sha256sum "$artifact_root"/bin/*-adapter-windows.exe
stat -c '%n %s' "$artifact_root"/bin/*-adapter-windows.exe
