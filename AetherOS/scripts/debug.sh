#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
KERNEL="${ROOT_DIR}/target/aetheros-x86_64/release/aetheros-kernel"

cd "${ROOT_DIR}"

if ! command -v qemu-system-x86_64 >/dev/null 2>&1; then
  echo "[debug] ERROR: qemu-system-x86_64 is not installed" >&2
  exit 1
fi

if [[ ! -f "${KERNEL}" ]]; then
  echo "[debug] ERROR: kernel not found at ${KERNEL}" >&2
  echo "[debug] Hint: run ./scripts/build.sh first." >&2
  exit 1
fi

exec qemu-system-x86_64 \
  -kernel "${KERNEL}" \
  -serial stdio \
  -no-reboot \
  -d int \
  -S -s
