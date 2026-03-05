#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
KERNEL_DIR="${ROOT_DIR}/kernel"

pushd "${ROOT_DIR}" >/dev/null

rustup component add rust-src --toolchain nightly
rustup component add llvm-tools-preview --toolchain nightly

cargo +nightly install bootimage --locked || true

cargo +nightly bootimage -p aetheros-kernel --manifest-path "${KERNEL_DIR}/Cargo.toml" --release

IMAGE_PATH="${KERNEL_DIR}/target/x86_64-unknown-none/release/bootimage-aetheros-kernel.bin"

echo "Built bootable kernel image: ${IMAGE_PATH}"

echo "Run with:"
echo "qemu-system-x86_64 -machine q35 -m 2G -serial stdio -drive format=raw,file=${IMAGE_PATH}"

popd >/dev/null
