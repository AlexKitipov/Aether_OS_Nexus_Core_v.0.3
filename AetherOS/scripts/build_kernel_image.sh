#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
KERNEL_PATH="${ROOT_DIR}/target/aetheros-x86_64/release/aetheros-kernel"
RUN_QEMU="${RUN_QEMU:-0}"
TOOLCHAIN="nightly-2026-03-13"

cd "${ROOT_DIR}"

# Avoid inheriting host-specific compiler flags that can break third-party crates
# (for example `-Z no-implicit-prelude`, which triggers missing `Result`/`Option`
# type errors inside dependencies like serde_core).
unset RUSTFLAGS
unset CARGO_ENCODED_RUSTFLAGS

if ! command -v qemu-system-x86_64 >/dev/null 2>&1; then
  echo "qemu-system-x86_64 is not installed. Install QEMU first (example: sudo apt-get install qemu-system-x86)." >&2
fi

if ! rustup toolchain list | rg -q "^${TOOLCHAIN}"; then
  echo "${TOOLCHAIN} toolchain is not available. Installing ${TOOLCHAIN}..."
  rustup toolchain install "${TOOLCHAIN}"
fi

rustup override set "${TOOLCHAIN}"
rustup component add rust-src
rustup component add llvm-tools-preview || true

cargo +"${TOOLCHAIN}" build --release --target .cargo/aetheros-x86_64.json \
  -Zbuild-std=core,alloc,compiler_builtins \
  -Zbuild-std-features=compiler-builtins-mem \
  -Zjson-target-spec

echo "Built kernel artifact: ${KERNEL_PATH}"
echo "Run with:"
echo "qemu-system-x86_64 -kernel target/aetheros-x86_64/release/aetheros-kernel"

if [[ "${RUN_QEMU}" == "1" ]]; then
  qemu-system-x86_64 -kernel "${KERNEL_PATH}"
fi
