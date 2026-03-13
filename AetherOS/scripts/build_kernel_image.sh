#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
KERNEL_PATH="${ROOT_DIR}/target/aetheros-x86_64/release/aetheros-kernel"
RUN_QEMU="${RUN_QEMU:-0}"

cd "${ROOT_DIR}"

cargo +nightly build \
  -Z json-target-spec \
  -Z build-std=core,alloc,compiler_builtins \
  -Z build-std-features=compiler-builtins-mem \
  -p aetheros-kernel \
  --manifest-path kernel/Cargo.toml \
  --target kernel/.cargo/aetheros-x86_64.json \
  --release

echo "Built kernel artifact: ${KERNEL_PATH}"
echo "Run with:"
echo "qemu-system-x86_64 -kernel target/aetheros-x86_64/release/aetheros-kernel"

if [[ "${RUN_QEMU}" == "1" ]]; then
  qemu-system-x86_64 -kernel "${KERNEL_PATH}"
fi
