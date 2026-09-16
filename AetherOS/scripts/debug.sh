#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
BIOS_IMAGE="${ROOT_DIR}/target/x86_64-unknown-none/release/aetheros-bios.img"

cd "${ROOT_DIR}"

if ! command -v qemu-system-x86_64 >/dev/null 2>&1; then
  echo "[debug] ERROR: qemu-system-x86_64 is not installed" >&2
  exit 1
fi

if [[ ! -f "${BIOS_IMAGE}" ]]; then
  echo "[debug] ERROR: BIOS disk image not found at ${BIOS_IMAGE}" >&2
  echo "[debug] Hint: run BOOT_MODE=bios ./scripts/build_kernel_image.sh first." >&2
  exit 1
fi

exec qemu-system-x86_64 \
  -drive "format=raw,file=${BIOS_IMAGE}" \
  -display none \
  -serial stdio \
  -no-reboot \
  -no-shutdown \
  -d int \
  -S -s
