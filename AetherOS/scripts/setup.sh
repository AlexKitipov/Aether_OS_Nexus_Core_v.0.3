#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT_DIR}"

if ! command -v rustup >/dev/null 2>&1; then
  echo "[setup] ERROR: rustup is not installed. Install rustup first: https://rustup.rs" >&2
  exit 1
fi

TOOLCHAIN="nightly-2024-12-01"

rustup toolchain install "${TOOLCHAIN}"
rustup override set "${TOOLCHAIN}"
rustup component add rust-src --toolchain "${TOOLCHAIN}"
rustup component add llvm-tools-preview --toolchain "${TOOLCHAIN}"

if command -v apt-get >/dev/null 2>&1; then
  echo "[setup] Installing Linux dependencies via apt-get..."
  sudo apt-get update
  sudo apt-get install -y llvm lld binutils qemu-system-x86
fi

echo "[setup] Completed. Toolchain: ${TOOLCHAIN}"

echo "[setup] To build, run: ./scripts/build.sh"
