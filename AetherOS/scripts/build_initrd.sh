#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
ROOTFS_DIR="${ROOT_DIR}/rootfs"
OUT_FILE="${ROOT_DIR}/target/initrd.img"

mkdir -p "${ROOT_DIR}/target"

if [[ ! -d "${ROOTFS_DIR}" ]]; then
  echo "[build_initrd] ERROR: rootfs directory does not exist: ${ROOTFS_DIR}" >&2
  exit 1
fi

if ! command -v cpio >/dev/null 2>&1; then
  echo "[diag][stage=build_initrd.archive][status=error] cpio is required but not installed" >&2
  exit 1
fi

(
  cd "${ROOTFS_DIR}"
  find . -print0 | cpio --null -o -H newc > "${OUT_FILE}"
)

echo "[build_initrd] created ${OUT_FILE}"
