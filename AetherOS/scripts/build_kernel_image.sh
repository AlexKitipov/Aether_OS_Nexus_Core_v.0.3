#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
KERNEL_PATH="${ROOT_DIR}/target/aetheros-x86_64/release/aetheros-kernel"
RUN_QEMU="${RUN_QEMU:-0}"

cd "${ROOT_DIR}"

if ! command -v qemu-system-x86_64 >/dev/null 2>&1; then
  echo "qemu-system-x86_64 is not installed. Install QEMU first (example: sudo apt-get install qemu-system-x86)." >&2
fi

if ! rustup toolchain list | rg -q '^nightly'; then
  echo "Nightly toolchain is not available. Installing nightly..."
  rustup toolchain install nightly
fi

rustup component add rust-src --toolchain nightly
rustup component add llvm-tools-preview --toolchain nightly

if ! cargo bootimage --version >/dev/null 2>&1; then
  cargo +nightly install bootimage --locked
fi

cargo +nightly bootimage -p aetheros-kernel --manifest-path "${KERNEL_DIR}/Cargo.toml" --release \
  -- -Zbuild-std -Zbuild-std-features=compiler-builtins-mem -Zjson-target-spec

echo "Built bootable kernel image: ${IMAGE_PATH}"

echo "Built kernel artifact: ${KERNEL_PATH}"
echo "Run with:"
echo "qemu-system-x86_64 -kernel target/aetheros-x86_64/release/aetheros-kernel"

if [[ "${RUN_QEMU}" == "1" ]]; then
  qemu-system-x86_64 -kernel "${KERNEL_PATH}"
fi
