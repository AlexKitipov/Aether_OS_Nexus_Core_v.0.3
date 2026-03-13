#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT_DIR}"

ROOTFS_DIR="rootfs"
KERNEL_PKG="aetheros-kernel"
VNODE_PKGS=(
  registry
  init-service
  display-compositor
  webview
)

echo "[build_all] Building kernel (${KERNEL_PKG})"
cargo +nightly build \
  -Z json-target-spec \
  -Z build-std=core,alloc,compiler_builtins \
  -Z build-std-features=compiler-builtins-mem \
  -p "${KERNEL_PKG}" \
  --manifest-path kernel/Cargo.toml \
  --target kernel/.cargo/aetheros-x86_64.json

echo "[build_all] Building V-Nodes on host target: ${VNODE_PKGS[*]}"
VNODE_ARGS=()
for pkg in "${VNODE_PKGS[@]}"; do
  VNODE_ARGS+=( -p "${pkg}" )
done
cargo build "${VNODE_ARGS[@]}"

mkdir -p "${ROOTFS_DIR}/vnode" target

for vnode in "${VNODE_PKGS[@]}"; do
  src="target/debug/${vnode}"
  dst="${ROOTFS_DIR}/vnode/${vnode}"
  if [[ ! -f "${src}" ]]; then
    echo "[build_all] ERROR: expected binary not found: ${src}" >&2
    exit 1
  fi

  cp "${src}" "${dst}"
  chmod +x "${dst}"
  echo "[build_all] copied ${src} -> ${dst}"
done

KERNEL_SRC="target/aetheros-x86_64/debug/${KERNEL_PKG}"
KERNEL_DST="target/kernel.bin"

if [[ ! -f "${KERNEL_SRC}" ]]; then
  echo "[build_all] ERROR: expected kernel binary not found: ${KERNEL_SRC}" >&2
  exit 1
fi

cp "${KERNEL_SRC}" "${KERNEL_DST}"
chmod +x "${KERNEL_DST}"
echo "[build_all] kernel ELF copied to ${KERNEL_DST}"

echo "[build_all] done"
