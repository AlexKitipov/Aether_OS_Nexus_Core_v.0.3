#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
BOOT_MODE="${BOOT_MODE:-bios}"
BIOS_IMAGE="${ROOT_DIR}/target/aetheros-x86_64/release/aetheros-bios.img"
UEFI_IMAGE="${ROOT_DIR}/target/aetheros-x86_64/release/aetheros-uefi.img"

if ! command -v qemu-system-x86_64 >/dev/null 2>&1; then
  echo "[run_qemu] ERROR: qemu-system-x86_64 is not installed" >&2
  exit 1
fi

case "${BOOT_MODE}" in
  bios)
    if [[ ! -f "${BIOS_IMAGE}" ]]; then
      echo "[run_qemu] ERROR: BIOS disk image not found at ${BIOS_IMAGE}" >&2
      echo "[run_qemu] Hint: run BOOT_MODE=bios ./scripts/build_kernel_image.sh first." >&2
      exit 1
    fi

    exec qemu-system-x86_64 \
      -drive "format=raw,file=${BIOS_IMAGE}" \
      -serial stdio \
      -no-reboot \
      -d int
    ;;
  uefi)
    if [[ ! -f "${UEFI_IMAGE}" ]]; then
      echo "[run_qemu] ERROR: UEFI disk image not found at ${UEFI_IMAGE}" >&2
      echo "[run_qemu] Hint: run BOOT_MODE=uefi ./scripts/build_kernel_image.sh first." >&2
      exit 1
    fi

    if [[ -z "${OVMF_CODE:-}" ]]; then
      echo "[run_qemu] ERROR: OVMF_CODE must point to an OVMF_CODE.fd firmware file for UEFI boot." >&2
      exit 1
    fi

    exec qemu-system-x86_64 \
      -drive "if=pflash,format=raw,readonly=on,file=${OVMF_CODE}" \
      -drive "format=raw,file=${UEFI_IMAGE}" \
      -serial stdio \
      -no-reboot \
      -d int
    ;;
  both)
    echo "[run_qemu] ERROR: BOOT_MODE=both is only valid for image creation; choose bios or uefi to run." >&2
    exit 1
    ;;
  *)
    echo "[run_qemu] ERROR: BOOT_MODE must be one of: bios, uefi" >&2
    exit 1
    ;;
esac
