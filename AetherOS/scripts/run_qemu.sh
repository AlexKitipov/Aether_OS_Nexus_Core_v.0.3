#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
KERNEL="${ROOT_DIR}/target/aetheros-x86_64/release/aetheros-kernel"

if ! command -v qemu-system-x86_64 >/dev/null 2>&1; then
  echo "[run_qemu] ERROR: qemu-system-x86_64 is not installed" >&2
  exit 1
fi

if [[ ! -f "${KERNEL}" ]]; then
  echo "[run_qemu] ERROR: kernel not found at ${KERNEL}" >&2
  echo "[run_qemu] Hint: run ./scripts/build_kernel_image.sh first." >&2
  exit 1
fi

exec qemu-system-x86_64 -kernel "${KERNEL}"
