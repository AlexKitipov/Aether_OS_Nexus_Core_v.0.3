#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
KERNEL="${ROOT_DIR}/target/aetheros-x86_64/release/aetheros-kernel"
TIMEOUT_SECONDS="${QEMU_TEST_TIMEOUT:-12}"

if ! command -v qemu-system-x86_64 >/dev/null 2>&1; then
  echo "[test_qemu] ERROR: qemu-system-x86_64 is not installed" >&2
  exit 1
fi

if [[ ! -f "${KERNEL}" ]]; then
  echo "[test_qemu] ERROR: kernel not found at ${KERNEL}" >&2
  echo "[test_qemu] Hint: run ./scripts/build_kernel_image.sh first." >&2
  exit 1
fi

LOG_FILE="$(mktemp)"
trap 'rm -f "${LOG_FILE}"' EXIT

echo "[test_qemu] running QEMU smoke test (timeout: ${TIMEOUT_SECONDS}s)"

set +e
timeout "${TIMEOUT_SECONDS}" qemu-system-x86_64 \
  -kernel "${KERNEL}" \
  -display none \
  -serial stdio \
  -no-reboot \
  -no-shutdown >"${LOG_FILE}" 2>&1
EXIT_CODE=$?
set -e

if [[ ${EXIT_CODE} -eq 124 ]]; then
  echo "[test_qemu] PASS: QEMU stayed alive for ${TIMEOUT_SECONDS}s"
  echo "[test_qemu] log excerpt:"
  sed -n '1,40p' "${LOG_FILE}"
  exit 0
fi

echo "[test_qemu] FAIL: QEMU exited unexpectedly (exit code: ${EXIT_CODE})" >&2
sed -n '1,80p' "${LOG_FILE}" >&2
exit ${EXIT_CODE}
