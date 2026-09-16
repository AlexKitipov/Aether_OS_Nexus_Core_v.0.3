#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
QEMU_BIN="${QEMU_BIN:-qemu-system-x86_64}"
QEMU_IMAGE="${QEMU_IMAGE:-${ROOT_DIR}/target/x86_64-unknown-none/release/aetheros-bios.img}"
QEMU_LOG_DIR="${QEMU_LOG_DIR:-${ROOT_DIR}/target/qemu-smoke}"
QEMU_TIMEOUT_SECONDS="${QEMU_TIMEOUT_SECONDS:-12}"
QEMU_MEMORY_MIB="${QEMU_MEMORY_MIB:-256}"
QEMU_BOOT_MARKER="${QEMU_BOOT_MARKER:-[kernel] Nexus Core v0.3 READY.}"

SERIAL_LOG="${QEMU_LOG_DIR}/serial.log"
DEBUG_LOG="${QEMU_LOG_DIR}/qemu-debug.log"
STDERR_LOG="${QEMU_LOG_DIR}/qemu-stderr.log"

if ! command -v "${QEMU_BIN}" >/dev/null 2>&1; then
  echo "[qemu_smoke] ERROR: QEMU binary not found: ${QEMU_BIN}" >&2
  exit 1
fi

if [[ ! -f "${QEMU_IMAGE}" ]]; then
  echo "[qemu_smoke] ERROR: BIOS disk image not found: ${QEMU_IMAGE}" >&2
  echo "[qemu_smoke] Hint: run BOOT_MODE=bios ./scripts/build_kernel_image.sh first." >&2
  exit 1
fi

mkdir -p "${QEMU_LOG_DIR}"
: >"${SERIAL_LOG}"
: >"${DEBUG_LOG}"
: >"${STDERR_LOG}"

echo "[qemu_smoke] booting BIOS image: ${QEMU_IMAGE}"
echo "[qemu_smoke] logs: serial=${SERIAL_LOG}, debug=${DEBUG_LOG}, stderr=${STDERR_LOG}"

set +e
timeout --kill-after=3s "${QEMU_TIMEOUT_SECONDS}s" "${QEMU_BIN}" \
  -machine pc \
  -cpu qemu64 \
  -m "${QEMU_MEMORY_MIB}" \
  -drive "format=raw,file=${QEMU_IMAGE}" \
  -display none \
  -monitor none \
  -serial "file:${SERIAL_LOG}" \
  -no-reboot \
  -no-shutdown \
  -d int,cpu_reset \
  -D "${DEBUG_LOG}" \
  2>"${STDERR_LOG}"
QEMU_EXIT_CODE=$?
set -e

if rg -qi 'triple fault|triple-fault' "${DEBUG_LOG}" "${STDERR_LOG}"; then
  echo "[qemu_smoke] FAIL: QEMU reported a triple fault." >&2
  exit 1
fi

# QEMU emits one reset record for initial power-on when cpu_reset tracing is
# enabled. More than one reset means the guest reset after boot started.
RESET_COUNT="$(rg -i -c 'CPU Reset|cpu reset' "${DEBUG_LOG}" "${STDERR_LOG}" 2>/dev/null | awk -F: '{ total += $NF } END { print total + 0 }')"
if [[ "${RESET_COUNT}" -gt 1 ]]; then
  echo "[qemu_smoke] FAIL: QEMU reported ${RESET_COUNT} CPU resets (expected only initial power-on reset)." >&2
  exit 1
fi

if [[ "${QEMU_EXIT_CODE}" -ne 124 ]]; then
  echo "[qemu_smoke] FAIL: QEMU exited unexpectedly (exit code ${QEMU_EXIT_CODE})." >&2
  exit "${QEMU_EXIT_CODE}"
fi

if ! rg -Fq "${QEMU_BOOT_MARKER}" "${SERIAL_LOG}"; then
  echo "[qemu_smoke] FAIL: boot-progress marker not found: ${QEMU_BOOT_MARKER}" >&2
  exit 1
fi

echo "[qemu_smoke] PASS: marker observed and QEMU stayed alive for ${QEMU_TIMEOUT_SECONDS}s."
