#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
KERNEL_DIR="${ROOT_DIR}/kernel"
KERNEL_PATH="${KERNEL_DIR}/target/x86_64-aether_os/release/aetheros-kernel"
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

if cargo bootimage --version >/dev/null 2>&1; then
  echo "note: bootimage is installed, but the current kernel uses bootloader 0.11 APIs."
  echo "note: cargo bootimage is not compatible with bootloader 0.11 and fails with metadata errors."
fi

cargo +nightly -Zbuild-std -Zbuild-std-features=compiler-builtins-mem -Zjson-target-spec \
  build -p aetheros-kernel --manifest-path "${KERNEL_DIR}/Cargo.toml" --target "${KERNEL_DIR}/x86_64-aether_os.json" --release

echo "Built kernel artifact: ${KERNEL_PATH}"
echo "note: this produces the kernel binary only; bootable disk image creation requires a bootloader 0.11 image builder flow."

echo "Run with (kernel ELF):"
echo "qemu-system-x86_64 -machine q35 -m 2G -serial stdio -kernel ${KERNEL_PATH}"

if [[ "${RUN_QEMU}" == "1" ]]; then
  qemu-system-x86_64 -machine q35 -m 2G -serial stdio -kernel "${KERNEL_PATH}"
fi

popd >/dev/null
