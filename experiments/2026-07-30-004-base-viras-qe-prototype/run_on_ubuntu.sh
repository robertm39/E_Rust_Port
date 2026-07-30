#!/usr/bin/env bash
set -euo pipefail

readonly SOURCE_ROOT="/opt/e-rust-port/source"
readonly WORK_ROOT="/opt/e-rust-port/viras-004"
readonly Z3_ARCHIVE="/opt/e-rust-port/z3-2d48fd119ce5074b880944c2b1c59e537c99cd46.tar.gz"
readonly EXPECTED_Z3_ARCHIVE_SHA256="9b78c0cc9f330dab9f39c132aba39c92fdba2dbc0aac26dd07b3946592dd21d8"
readonly EXPERIMENT_REL="experiments/2026-07-30-004-base-viras-qe-prototype"

if [[ -e "${WORK_ROOT}" ]]; then
  echo "Refusing to reuse existing experiment work directory: ${WORK_ROOT}" >&2
  exit 2
fi
mkdir -p "${WORK_ROOT}/z3-source" "${WORK_ROOT}/evidence"

actual_archive_sha256="$(sha256sum "${Z3_ARCHIVE}" | awk '{print $1}')"
if [[ "${actual_archive_sha256}" != "${EXPECTED_Z3_ARCHIVE_SHA256}" ]]; then
  echo "Unexpected Z3 archive SHA-256: ${actual_archive_sha256}" >&2
  exit 2
fi

export DEBIAN_FRONTEND=noninteractive
apt-get update >"${WORK_ROOT}/apt.stdout.log" 2>"${WORK_ROOT}/apt.stderr.log"
apt-get install -y build-essential cmake ninja-build python3 \
  >>"${WORK_ROOT}/apt.stdout.log" 2>>"${WORK_ROOT}/apt.stderr.log"

tar -xzf "${Z3_ARCHIVE}" -C "${WORK_ROOT}/z3-source"
cmake \
  -S "${WORK_ROOT}/z3-source" \
  -B "${WORK_ROOT}/z3-build" \
  -G Ninja \
  -DCMAKE_BUILD_TYPE=Release \
  -DZ3_BUILD_EXECUTABLE=ON \
  -DZ3_BUILD_TEST_EXECUTABLES=OFF \
  -DZ3_BUILD_LIBZ3_SHARED=OFF \
  >"${WORK_ROOT}/z3-configure.stdout.log" \
  2>"${WORK_ROOT}/z3-configure.stderr.log"
cmake --build "${WORK_ROOT}/z3-build" --parallel 4 \
  >"${WORK_ROOT}/z3-build.stdout.log" \
  2>"${WORK_ROOT}/z3-build.stderr.log"

cd "${SOURCE_ROOT}"
python3 "${EXPERIMENT_REL}/test_prototype.py" -v \
  >"${WORK_ROOT}/focused-tests.stdout.log" \
  2>"${WORK_ROOT}/focused-tests.stderr.log"

python3 "${EXPERIMENT_REL}/run_experiment.py" \
  --z3 "${WORK_ROOT}/z3-build/z3" \
  --seed 0xB451E2026 \
  --cases 1000 \
  --output "${WORK_ROOT}/evidence/report-1.json" \
  >"${WORK_ROOT}/run-1.stdout.log" \
  2>"${WORK_ROOT}/run-1.stderr.log"

python3 "${EXPERIMENT_REL}/run_experiment.py" \
  --z3 "${WORK_ROOT}/z3-build/z3" \
  --seed 0xB451E2026 \
  --cases 1000 \
  --output "${WORK_ROOT}/evidence/report-2.json" \
  >"${WORK_ROOT}/run-2.stdout.log" \
  2>"${WORK_ROOT}/run-2.stderr.log"

cmp "${WORK_ROOT}/evidence/report-1.json" "${WORK_ROOT}/evidence/report-2.json"
cp "${WORK_ROOT}/focused-tests.stdout.log" "${WORK_ROOT}/evidence/"
cp "${WORK_ROOT}/focused-tests.stderr.log" "${WORK_ROOT}/evidence/"
cp "${WORK_ROOT}/run-1.stdout.log" "${WORK_ROOT}/evidence/"
cp "${WORK_ROOT}/run-1.stderr.log" "${WORK_ROOT}/evidence/"
cp "${WORK_ROOT}/run-2.stdout.log" "${WORK_ROOT}/evidence/"
cp "${WORK_ROOT}/run-2.stderr.log" "${WORK_ROOT}/evidence/"
cp "${WORK_ROOT}/z3-configure.stdout.log" "${WORK_ROOT}/evidence/"
cp "${WORK_ROOT}/z3-configure.stderr.log" "${WORK_ROOT}/evidence/"
cp "${WORK_ROOT}/z3-build.stdout.log" "${WORK_ROOT}/evidence/"
cp "${WORK_ROOT}/z3-build.stderr.log" "${WORK_ROOT}/evidence/"

(
  cd "${WORK_ROOT}"
  tar -czf evidence.tar.gz evidence
)
sha256sum \
  "${WORK_ROOT}/evidence/report-1.json" \
  "${WORK_ROOT}/evidence/report-2.json" \
  "${WORK_ROOT}/evidence.tar.gz" \
  "${WORK_ROOT}/z3-build/z3" \
  >"${WORK_ROOT}/SHA256SUMS"
touch "${WORK_ROOT}/SUCCESS"
cat "${WORK_ROOT}/run-1.stdout.log"
cat "${WORK_ROOT}/SHA256SUMS"
