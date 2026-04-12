#!/usr/bin/env bash
set -euo pipefail

REQUIRED_COMMANDS=(
  qemu-system-x86_64
  cpio
  timeout
  ld.lld
  llvm-ar
  python3
  pip3
)

KERNEL_TOOLCHAIN_COMMANDS=(
  gcc
  clang
  nasm
  x86_64-elf-gcc
)

IMAGE_PIPELINE_COMMANDS=(
  grub-mkrescue
  mcopy
)

UEFI_FIRMWARE_PATHS=(
  /usr/share/OVMF/OVMF_CODE.fd
  /usr/share/OVMF/OVMF.fd
  /usr/share/edk2/ovmf/OVMF_CODE.fd
  /usr/share/qemu/OVMF.fd
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

PYTHON_MODULES=(
  websockets
  numpy
  PIL
  IPython
)

SCRIPT_GLOBS=(
  scripts/*.sh
  AetherOS/scripts/*.sh
)

MIN_ROOT_MB=4096
MIN_TMP_MB=1024

failures=0
warnings=0

print_section() {
  echo
  echo "== $1 =="
}

mark_fail() {
  failures=1
}

mark_warn() {
  warnings=1
}

check_command_required() {
  local cmd="$1"
  if command -v "$cmd" >/dev/null 2>&1; then
    echo "[OK] command found: $cmd"
  else
    echo "[MISSING] required command not found: $cmd"
    mark_fail
  fi
}

check_command_optional() {
  local cmd="$1"
  local hint="$2"
  if command -v "$cmd" >/dev/null 2>&1; then
    echo "[OK] optional command found: $cmd"
  else
    echo "[WARN] optional command not found: $cmd ($hint)"
    mark_warn
  fi
}

check_toolchain() {
  if rustup toolchain list | grep -q "$RUST_TOOLCHAIN"; then
    echo "[OK] Rust toolchain installed: $RUST_TOOLCHAIN"
  else
    echo "[MISSING] Rust toolchain not installed: $RUST_TOOLCHAIN"
    echo "          Install with: rustup toolchain install $RUST_TOOLCHAIN"
    mark_fail
  fi
}

check_component() {
  local component="$1"
  if rustup component list --toolchain "$RUST_TOOLCHAIN" 2>/dev/null | grep -q "^${component} .*installed"; then
    echo "[OK] Rust component installed ($RUST_TOOLCHAIN): $component"
  else
    echo "[MISSING] Rust component missing ($RUST_TOOLCHAIN): $component"
    echo "          Install with: rustup component add --toolchain $RUST_TOOLCHAIN $component"
    mark_fail
  fi
}

check_target() {
  local target="$1"
  if rustup target list --installed --toolchain "$RUST_TOOLCHAIN" 2>/dev/null | grep -q "^${target}$"; then
    echo "[OK] Rust target installed ($RUST_TOOLCHAIN): $target"
  else
    echo "[MISSING] Rust target missing ($RUST_TOOLCHAIN): $target"
    echo "          Install with: rustup target add --toolchain $RUST_TOOLCHAIN $target"
    mark_fail
  fi
}

check_python_runtime() {
  local py_version pip_version
  py_version="$(python3 --version 2>/dev/null || true)"
  pip_version="$(pip3 --version 2>/dev/null || true)"

  if [[ -n "$py_version" ]]; then
    echo "[OK] $py_version"
  else
    echo "[MISSING] python3 is not available"
    mark_fail
  fi

  if [[ -n "$pip_version" ]]; then
    echo "[OK] $pip_version"
  else
    echo "[MISSING] pip3 is not available"
    mark_fail
  fi

  if command -v python3 >/dev/null 2>&1; then
    local modules_csv
    modules_csv="$(IFS=,; echo "${PYTHON_MODULES[*]}")"
    if python3 - "$modules_csv" <<'PY'
import sys

modules = [m for m in sys.argv[1].split(",") if m]
missing = []
for module in modules:
    try:
        __import__(module)
    except Exception:
        missing.append(module)
if missing:
    print("[MISSING] Python modules not importable:", ", ".join(missing))
    raise SystemExit(1)
print("[OK] Python module imports passed:", ", ".join(modules))
PY
    then
      :
    else
      mark_fail
    fi
  fi
}

check_scripts_permissions() {
  local script shebang
  shopt -s nullglob
  for pattern in "${SCRIPT_GLOBS[@]}"; do
    for script in $pattern; do
      [[ -f "$script" ]] || continue

      if [[ -x "$script" ]]; then
        echo "[OK] executable bit set: $script"
      else
        echo "[WARN] executable bit missing: $script (run: chmod +x $script)"
        mark_warn
      fi

      shebang="$(head -n 1 "$script" || true)"
      if [[ "$shebang" == "#!/usr/bin/env bash" ]] || [[ "$shebang" == "#!/bin/bash" ]]; then
        echo "[OK] shebang looks valid: $script"
      else
        echo "[WARN] unexpected/missing bash shebang in $script"
        mark_warn
      fi
    done
  done
  shopt -u nullglob
}

check_kvm() {
  if [[ -e /dev/kvm ]]; then
    echo "[OK] /dev/kvm exists"
    if [[ -r /dev/kvm && -w /dev/kvm ]]; then
      echo "[OK] current user can access /dev/kvm"
    else
      echo "[WARN] /dev/kvm exists but current user lacks read/write permissions"
      mark_warn
    fi
  else
    echo "[WARN] /dev/kvm not present (QEMU will run without acceleration)"
    mark_warn
  fi
}

check_disk_space() {
  local root_free_mb tmp_free_mb
  root_free_mb="$(df -Pm / | awk 'NR==2 {print $4}')"
  tmp_free_mb="$(df -Pm /tmp | awk 'NR==2 {print $4}')"

  if [[ "$root_free_mb" -ge "$MIN_ROOT_MB" ]]; then
    echo "[OK] root free space: ${root_free_mb}MB"
  else
    echo "[WARN] low root free space: ${root_free_mb}MB (< ${MIN_ROOT_MB}MB)"
    mark_warn
  fi

  if [[ "$tmp_free_mb" -ge "$MIN_TMP_MB" ]]; then
    echo "[OK] /tmp free space: ${tmp_free_mb}MB"
  else
    echo "[WARN] low /tmp free space: ${tmp_free_mb}MB (< ${MIN_TMP_MB}MB)"
    mark_warn
  fi
}

check_uefi_firmware() {
  local found=0
  for fw in "${UEFI_FIRMWARE_PATHS[@]}"; do
    if [[ -f "$fw" ]]; then
      echo "[OK] UEFI firmware found: $fw"
      found=1
      break
    fi
  done

  if [[ "$found" -eq 0 ]]; then
    echo "[WARN] No OVMF firmware binary detected in common locations"
    echo "       Install package: ovmf (or edk2-ovmf depending on distro)"
    mark_warn
  fi
}

print_section "Checking required host commands"
for cmd in "${REQUIRED_COMMANDS[@]}"; do
  check_command_required "$cmd"
done

print_section "Checking kernel toolchain commands (optional but recommended)"
for cmd in "${KERNEL_TOOLCHAIN_COMMANDS[@]}"; do
  check_command_optional "$cmd" "needed by some kernel/boot pipelines"
done

print_section "Checking image/ISO pipeline commands (optional)"
for cmd in "${IMAGE_PIPELINE_COMMANDS[@]}"; do
  check_command_optional "$cmd" "needed when producing bootable ISO/FAT images"
done

print_section "Checking Rust toolchain"
if command -v rustup >/dev/null 2>&1; then
  check_toolchain

  for component in "${RUST_COMPONENTS[@]}"; do
    check_component "$component"
  done

  echo "[INFO] Installed targets for $RUST_TOOLCHAIN:"
  rustup target list --installed --toolchain "$RUST_TOOLCHAIN" 2>/dev/null || true

  for target in "${RUST_TARGETS[@]}"; do
    check_target "$target"
  done
else
  echo "[MISSING] rustup is not installed"
  echo "          Install Rust/rustup first: https://rustup.rs"
  mark_fail
fi

print_section "Checking Python runtime and modules"
check_python_runtime

print_section "Checking script permissions and shebangs"
check_scripts_permissions

print_section "Checking QEMU acceleration (KVM)"
check_kvm

print_section "Checking disk and /tmp capacity"
check_disk_space

print_section "Checking UEFI firmware"
check_uefi_firmware

print_section "Cross-platform note"
uname_out="$(uname -s 2>/dev/null || echo unknown)"
case "$uname_out" in
  Linux)
    echo "[OK] Linux host detected: $uname_out"
    echo "[INFO] Package names in scripts/install_deps_ubuntu.sh target Debian/Ubuntu-like distros."
    ;;
  Darwin)
    echo "[WARN] macOS host detected. Use brew equivalents for qemu, llvm, nasm, mtools, and ovmf."
    mark_warn
    ;;
  *)
    echo "[WARN] Non-Linux host detected: $uname_out. Prefer Linux or WSL2 for reproducible kernel builds."
    mark_warn
    ;;
esac

print_section "Summary"
if [[ "$failures" -eq 0 && "$warnings" -eq 0 ]]; then
  echo "Environment check passed: all required dependencies are installed and optional checks are healthy."
  exit 0
fi

if [[ "$failures" -eq 0 ]]; then
  echo "Environment check completed with warnings only."
  echo "Build may work, but review warnings above for best reliability/performance."
  exit 0
fi

echo "Environment check failed: install missing required dependencies and re-run scripts/check_env.sh"
exit 1
