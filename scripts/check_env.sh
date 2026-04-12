#!/usr/bin/env bash
set -euo pipefail

REQUIRED_COMMANDS=(
  qemu-system-x86_64
  cpio
  timeout
  ld.lld
  llvm-ar
)

RUST_TOOLCHAIN="nightly-2024-12-01"
RUST_COMPONENTS=(
  rust-src
  llvm-tools-preview
)
RUST_TARGETS=(
  x86_64-unknown-uefi
  x86_64-unknown-none
)

missing=0

print_section() {
  echo
  echo "== $1 =="
}

check_command() {
  local cmd="$1"
  if command -v "$cmd" >/dev/null 2>&1; then
    echo "[OK] command found: $cmd"
  else
    echo "[MISSING] command not found: $cmd"
    missing=1
  fi
}

check_toolchain() {
  if rustup toolchain list | grep -q "$RUST_TOOLCHAIN"; then
    echo "[OK] Rust toolchain installed: $RUST_TOOLCHAIN"
  else
    echo "[MISSING] Rust toolchain not installed: $RUST_TOOLCHAIN"
    echo "          Install with: rustup toolchain install $RUST_TOOLCHAIN"
    missing=1
  fi
}

check_component() {
  local component="$1"
  if rustup component list --toolchain "$RUST_TOOLCHAIN" 2>/dev/null | grep -q "^${component} .*installed"; then
    echo "[OK] Rust component installed ($RUST_TOOLCHAIN): $component"
  else
    echo "[MISSING] Rust component missing ($RUST_TOOLCHAIN): $component"
    echo "          Install with: rustup component add --toolchain $RUST_TOOLCHAIN $component"
    missing=1
  fi
}

check_target() {
  local target="$1"
  if rustup target list --toolchain "$RUST_TOOLCHAIN" 2>/dev/null | grep -q "^${target} (installed)"; then
    echo "[OK] Rust target installed ($RUST_TOOLCHAIN): $target"
  else
    echo "[MISSING] Rust target missing ($RUST_TOOLCHAIN): $target"
    echo "          Install with: rustup target add --toolchain $RUST_TOOLCHAIN $target"
    missing=1
  fi
}

print_section "Checking required host commands"
for cmd in "${REQUIRED_COMMANDS[@]}"; do
  check_command "$cmd"
done

print_section "Checking Rust toolchain"
if command -v rustup >/dev/null 2>&1; then
  check_toolchain

  for component in "${RUST_COMPONENTS[@]}"; do
    check_component "$component"
  done

  for target in "${RUST_TARGETS[@]}"; do
    check_target "$target"
  done
else
  echo "[MISSING] rustup is not installed"
  echo "          Install Rust/rustup first: https://rustup.rs"
  missing=1
fi

print_section "Summary"
if [[ "$missing" -eq 0 ]]; then
  echo "Environment check passed: all required dependencies are installed."
  exit 0
else
  echo "Environment check failed: install missing dependencies and re-run scripts/check_env.sh"
  exit 1
fi
