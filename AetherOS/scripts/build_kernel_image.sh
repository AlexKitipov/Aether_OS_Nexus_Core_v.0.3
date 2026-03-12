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

CARGO_BUILD_CMD=(cargo)
if rustup toolchain list | rg -q '^nightly'; then
  CARGO_BUILD_CMD=(cargo +nightly)
else
  echo "Nightly toolchain is not available. Installing nightly..."
  if rustup toolchain install nightly; then
    CARGO_BUILD_CMD=(cargo +nightly)
  else
    echo "warning: failed to install nightly; attempting build with current toolchain." >&2
    echo "warning: kernel build may fail because custom JSON targets require nightly rustc support." >&2
  fi
fi

# Best-effort: only install components when nightly is available.
if [[ "${CARGO_BUILD_CMD[*]}" == "cargo +nightly" ]]; then
  if ! rustup component add rust-src --toolchain nightly; then
    echo "warning: unable to install rust-src for nightly; continuing with existing toolchain state." >&2
  fi
  if ! rustup component add llvm-tools-preview --toolchain nightly; then
    echo "warning: unable to install llvm-tools-preview for nightly; continuing with existing toolchain state." >&2
  fi
fi

if cargo bootimage --version >/dev/null 2>&1; then
  echo "note: bootimage is installed, but the current kernel uses bootloader 0.11 APIs."
  echo "note: cargo bootimage is not compatible with bootloader 0.11 and fails with metadata errors."
fi

BASE_ARGS=(
  -Zbuild-std=core,alloc,compiler_builtins
  -Zbuild-std-features=compiler-builtins-mem
  build
  -p aetheros-kernel
  --manifest-path "${KERNEL_DIR}/Cargo.toml"
  --target "${KERNEL_DIR}/x86_64-aether_os.json"
  --release
)

if ! "${CARGO_BUILD_CMD[@]}" "${BASE_ARGS[@]}"; then
  echo "note: initial build attempt failed; retrying with cargo -Zjson-target-spec for JSON target compatibility."
  "${CARGO_BUILD_CMD[@]}" -Zjson-target-spec "${BASE_ARGS[@]}"
fi

echo "Built kernel artifact: ${KERNEL_PATH}"
echo "note: this produces the kernel binary only; bootable disk image creation requires a bootloader 0.11 image builder flow."

echo "Run with (kernel ELF):"
echo "qemu-system-x86_64 -machine q35 -m 2G -serial stdio -kernel ${KERNEL_PATH}"

if [[ "${RUN_QEMU}" == "1" ]]; then
  qemu-system-x86_64 -machine q35 -m 2G -serial stdio -kernel "${KERNEL_PATH}"
fi

popd >/dev/null
