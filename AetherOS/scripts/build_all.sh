#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT_DIR}"

TARGET_SPEC="aetheros-target.json"
TARGET_DIR="target/aetheros-target/debug"
ROOTFS_DIR="rootfs"

KERNEL_PKG="aetheros-kernel"
VNODE_PKGS=(
  registry
  init-service
  display-compositor
  webview
)

echo "[build_all] Building kernel (${KERNEL_PKG})"
cargo +nightly build -Z json-target-spec --target "${TARGET_SPEC}" -p "${KERNEL_PKG}"

echo "[build_all] Building V-Nodes: ${VNODE_PKGS[*]}"
VNODE_ARGS=()
for pkg in "${VNODE_PKGS[@]}"; do
  VNODE_ARGS+=( -p "${pkg}" )
done
cargo +nightly build -Z json-target-spec --target "${TARGET_SPEC}" "${VNODE_ARGS[@]}"

mkdir -p "${ROOTFS_DIR}/vnode" target

for vnode in "${VNODE_PKGS[@]}"; do
  src="${TARGET_DIR}/${vnode}"
  dst="${ROOTFS_DIR}/vnode/${vnode}"
  if [[ ! -f "${src}" ]]; then
    echo "[build_all] ERROR: expected binary not found: ${src}" >&2
    exit 1
  fi

  cp "${src}" "${dst}"
  chmod +x "${dst}"
  echo "[build_all] copied ${src} -> ${dst}"
done

KERNEL_SRC="${TARGET_DIR}/${KERNEL_PKG}"
KERNEL_DST="target/kernel.bin"

if [[ ! -f "${KERNEL_SRC}" ]]; then
  echo "[build_all] ERROR: expected kernel binary not found: ${KERNEL_SRC}" >&2
  exit 1
fi

cp "${KERNEL_SRC}" "${KERNEL_DST}"
chmod +x "${KERNEL_DST}"
echo "[build_all] kernel image copied to ${KERNEL_DST}"

echo "[build_all] done"
