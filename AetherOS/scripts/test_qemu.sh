#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"

QEMU_TIMEOUT_SECONDS="${QEMU_TEST_TIMEOUT:-12}" \
  QEMU_LOG_DIR="${QEMU_LOG_DIR:-${ROOT_DIR}/target/qemu-smoke}" \
  exec "${ROOT_DIR}/scripts/qemu_smoke_test.sh"
