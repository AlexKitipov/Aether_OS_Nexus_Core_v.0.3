#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT_DIR}"

cargo +nightly build \
  -Z json-target-spec \
  -Z build-std=core,alloc,compiler_builtins \
  -Z build-std-features=compiler-builtins-mem \
  -p aetheros-kernel \
  --manifest-path kernel/Cargo.toml \
  --target kernel/.cargo/aetheros-x86_64.json \
  --release
