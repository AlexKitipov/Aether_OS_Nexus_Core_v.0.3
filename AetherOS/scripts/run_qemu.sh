#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
KERNEL="${ROOT_DIR}/target/kernel.bin"
INITRD="${ROOT_DIR}/target/initrd.img"

if ! command -v qemu-system-x86_64 >/dev/null 2>&1; then
  echo "[run_qemu] ERROR: qemu-system-x86_64 is not installed" >&2
  exit 1
fi

if [[ ! -f "${KERNEL}" ]]; then
  echo "[run_qemu] ERROR: kernel not found at ${KERNEL}" >&2
  exit 1
fi

if [[ ! -f "${INITRD}" ]]; then
  echo "[run_qemu] ERROR: initrd not found at ${INITRD}" >&2
  exit 1
fi

qemu-system-x86_64 \
  -m 512M \
  -kernel "${KERNEL}" \
  -initrd "${INITRD}" \
  -serial stdio \
  -display none
