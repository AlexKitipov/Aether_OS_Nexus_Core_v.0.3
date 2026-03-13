#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
KERNEL_PATH="${ROOT_DIR}/target/aetheros-x86_64/release/aetheros-kernel"
RUN_QEMU="${RUN_QEMU:-0}"

cd "${ROOT_DIR}"

cargo build --release --target .cargo/aetheros-x86_64.json

echo "Built kernel artifact: ${KERNEL_PATH}"
echo "Run with:"
echo "qemu-system-x86_64 -kernel target/aetheros-x86_64/release/aetheros-kernel"

if [[ "${RUN_QEMU}" == "1" ]]; then
  qemu-system-x86_64 -kernel "${KERNEL_PATH}"
fi
