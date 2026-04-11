#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
KERNEL_PATH="${ROOT_DIR}/target/x86_64-unknown-none/release/aetheros-kernel"
RUN_QEMU="${RUN_QEMU:-0}"
TOOLCHAIN="nightly-2024-12-01"

cd "${ROOT_DIR}"

# Avoid inheriting host-specific compiler flags that can break third-party crates
# (for example `-Z no-implicit-prelude`, which triggers missing `Result`/`Option`
# type errors inside dependencies like serde_core).
unset RUSTFLAGS
unset CARGO_ENCODED_RUSTFLAGS
unset CARGO_BUILD_RUSTFLAGS

if ! command -v qemu-system-x86_64 >/dev/null 2>&1; then
  echo "qemu-system-x86_64 is not installed. Install QEMU first (example: sudo apt-get install qemu-system-x86)." >&2
fi

if command -v rg >/dev/null 2>&1; then
  RUSTUP_TOOLCHAIN_CHECK="rustup toolchain list | rg -q '^${TOOLCHAIN}'"
else
  RUSTUP_TOOLCHAIN_CHECK="rustup toolchain list | grep -q '^${TOOLCHAIN}'"
fi
if ! eval "${RUSTUP_TOOLCHAIN_CHECK}"; then
  echo "${TOOLCHAIN} toolchain is not available. Installing ${TOOLCHAIN}..."
  rustup toolchain install "${TOOLCHAIN}"
fi

rustup override set "${TOOLCHAIN}"
rustup component add rust-src --toolchain "${TOOLCHAIN}"
rustup component add llvm-tools-preview --toolchain "${TOOLCHAIN}"

cargo +"${TOOLCHAIN}" build --release --target x86_64-unknown-none \
  -p aetheros-kernel \
  -p aetheros-kernel

echo "Built kernel artifact: ${KERNEL_PATH}"

echo "[diag][stage=build_kernel_image.section_validation] validating section memory map"
OBJDUMP_TOOL=""
if command -v llvm-objdump >/dev/null 2>&1; then
  OBJDUMP_TOOL="llvm-objdump"
elif command -v rust-objdump >/dev/null 2>&1; then
  OBJDUMP_TOOL="rust-objdump"
fi

if [[ -n "${OBJDUMP_TOOL}" ]]; then
  if ! section_table="$(${OBJDUMP_TOOL} -h "${KERNEL_PATH}" 2>/dev/null)"; then
    echo "[diag][stage=build_kernel_image.section_validation][status=warn] ${OBJDUMP_TOOL} exists but could not run; skipping memory map validation" >&2
    section_table=""
  fi

  if [[ -z "${section_table}" ]]; then
    echo "[diag][stage=build_kernel_image.section_validation][status=warn] no section table available; skipping memory map validation" >&2
  else
    text_start_addr="$(awk '$2==".text.start" || $2==".text._start"{print $4}' <<<"${section_table}" | head -n1)"
    text_addr="$(awk '$2==".text"{print $4}' <<<"${section_table}" | head -n1)"
    rodata_addr="$(awk '$2==".rodata"{print $4}' <<<"${section_table}" | head -n1)"
    data_addr="$(awk '$2==".data"{print $4}' <<<"${section_table}" | head -n1)"
    bss_addr="$(awk '$2==".bss"{print $4}' <<<"${section_table}" | head -n1)"

    if [[ -z "${text_start_addr}" || -z "${text_addr}" || -z "${rodata_addr}" || -z "${data_addr}" || -z "${bss_addr}" ]]; then
      echo "[build_kernel_image] ERROR: missing one or more required sections (.text.start/.text/.rodata/.data/.bss)" >&2
      exit 1
    fi

    expected_text_start_addr="00100000"
    if ! (( 16#${text_start_addr} == 16#${expected_text_start_addr} )); then
      echo "[build_kernel_image] ERROR: .text.start starts at 0x${text_start_addr}, expected 0x${expected_text_start_addr}" >&2
      exit 1
    fi

    if ! (( 16#${text_start_addr} <= 16#${text_addr} && 16#${text_addr} < 16#${rodata_addr} && 16#${rodata_addr} < 16#${data_addr} && 16#${data_addr} <= 16#${bss_addr} )); then
      echo "[build_kernel_image] ERROR: section order is invalid (.text.start -> .text -> .rodata -> .data -> .bss)" >&2
      exit 1
    fi

    echo "[diag][stage=build_kernel_image.section_validation][status=ok] memory layout OK: .text.start=0x${text_start_addr}, .text=0x${text_addr}, .rodata=0x${rodata_addr}, .data=0x${data_addr}, .bss=0x${bss_addr}"
  fi
else
  echo "[diag][stage=build_kernel_image.section_validation][status=warn] llvm-objdump/rust-objdump not found, skipping memory map validation" >&2
fi

echo "Run with:"
echo "qemu-system-x86_64 -kernel target/x86_64-unknown-none/release/aetheros-kernel -serial stdio -no-reboot -d int"

if [[ "${RUN_QEMU}" == "1" ]]; then
  qemu-system-x86_64 \
    -kernel "${KERNEL_PATH}" \
    -serial stdio \
    -no-reboot \
    -d int
fi
