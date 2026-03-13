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
cargo build --release --target .cargo/aetheros-x86_64.json

echo "[build_all] Building V-Nodes on host target: ${VNODE_PKGS[*]}"
VNODE_ARGS=()
for pkg in "${VNODE_PKGS[@]}"; do
  VNODE_ARGS+=( -p "${pkg}" )
done
cargo build --release "${VNODE_ARGS[@]}"

mkdir -p "${ROOTFS_DIR}/vnode" target

for vnode in "${VNODE_PKGS[@]}"; do
  src="target/release/${vnode}"
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
