#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
KERNEL_DIR="${ROOT_DIR}/kernel"
IMAGE_PATH="${KERNEL_DIR}/target/x86_64-aether_os/release/bootimage-aetheros-kernel.bin"
RUN_QEMU="${RUN_QEMU:-0}"

pushd "${ROOT_DIR}" >/dev/null

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

cargo +nightly bootimage -p aetheros-kernel --manifest-path "${KERNEL_DIR}/Cargo.toml" --release

echo "Built bootable kernel image: ${IMAGE_PATH}"

echo "Run with:"
echo "qemu-system-x86_64 -machine q35 -m 2G -serial stdio -drive format=raw,file=${IMAGE_PATH}"

if [[ "${RUN_QEMU}" == "1" ]]; then
  qemu-system-x86_64 -machine q35 -m 2G -serial stdio -drive format=raw,file="${IMAGE_PATH}"
fi

popd >/dev/null
