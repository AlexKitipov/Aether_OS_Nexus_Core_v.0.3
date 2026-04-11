#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT_DIR}"

# Avoid inheriting host-specific compiler flags that can break third-party crates
# (for example `-Z no-implicit-prelude`, which triggers missing `Result`/`Option`
# type errors inside dependencies like serde_core).
unset RUSTFLAGS
unset CARGO_ENCODED_RUSTFLAGS
unset CARGO_BUILD_RUSTFLAGS

TOOLCHAIN="nightly-2024-12-01"
ROOTFS_DIR="rootfs"
KERNEL_PKG="aetheros-kernel"
KERNEL_TARGET=".cargo/aetheros-x86_64.json"
VNODE_PKGS=(
  registry
  init-service
  display-compositor
  webview
)

echo "[build_all] Building kernel (${KERNEL_PKG})"
cargo +"${TOOLCHAIN}" build --release --target "${KERNEL_TARGET}" \
  -Zbuild-std=core,alloc,compiler_builtins \
  -Zbuild-std-features=compiler-builtins-mem

echo "[diag][stage=build_all.vnode_build] Building V-Nodes on kernel target: ${VNODE_PKGS[*]}"
VNODE_ARGS=()
for pkg in "${VNODE_PKGS[@]}"; do
  VNODE_ARGS+=( -p "${pkg}" )
done
cargo +"${TOOLCHAIN}" build --release --target "${KERNEL_TARGET}" \
  -Zbuild-std=core,alloc,compiler_builtins \
  -Zbuild-std-features=compiler-builtins-mem \
  "${VNODE_ARGS[@]}"

mkdir -p "${ROOTFS_DIR}/vnode" target

for vnode in "${VNODE_PKGS[@]}"; do
  src="target/aetheros-x86_64/release/${vnode}"
  dst="${ROOTFS_DIR}/vnode/${vnode}"
  if [[ ! -f "${src}" ]]; then
    echo "[build_all] ERROR: expected binary not found: ${src}" >&2
    exit 1
  fi

  cp "${src}" "${dst}"
  chmod +x "${dst}"
  echo "[build_all] copied ${src} -> ${dst}"
done

echo "[build_all] done"
