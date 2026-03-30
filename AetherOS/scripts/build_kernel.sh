#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
TOOLCHAIN="nightly-2026-03-13"
cd "${ROOT_DIR}"

# Avoid inheriting host-specific compiler flags that can break third-party crates
# (for example `-Z no-implicit-prelude`, which triggers missing `Result`/`Option`
# type errors inside dependencies like serde_core).
unset RUSTFLAGS
unset CARGO_ENCODED_RUSTFLAGS
unset CARGO_BUILD_RUSTFLAGS

cargo +"${TOOLCHAIN}" build --release --target .cargo/aetheros-x86_64.json \
  -Zbuild-std=core,alloc,compiler_builtins \
  -Zbuild-std-features=compiler-builtins-mem \
  -Zjson-target-spec
