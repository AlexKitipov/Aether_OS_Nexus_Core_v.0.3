#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
KERNEL_PATH="${ROOT_DIR}/target/aetheros-x86_64/release/aetheros-kernel"
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

if ! rustup toolchain list | rg -q "^${TOOLCHAIN}"; then
  echo "${TOOLCHAIN} toolchain is not available. Installing ${TOOLCHAIN}..."
  rustup toolchain install "${TOOLCHAIN}"
fi

rustup override set "${TOOLCHAIN}"
rustup component add rust-src --toolchain "${TOOLCHAIN}"
rustup component add llvm-tools-preview --toolchain "${TOOLCHAIN}"

cargo +"${TOOLCHAIN}" build --release --target .cargo/aetheros-x86_64.json \
  -Zbuild-std=core,alloc,compiler_builtins \
  -Zbuild-std-features=compiler-builtins-mem

echo "Built kernel artifact: ${KERNEL_PATH}"

echo "[build_kernel_image] validating section memory map"
if command -v llvm-objdump >/dev/null 2>&1; then
  section_table="$(llvm-objdump -h "${KERNEL_PATH}")"
  text_start_addr="$(awk '$2==".text.start"{print $4}' <<<"${section_table}" | head -n1)"
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

  echo "[build_kernel_image] memory layout OK: .text.start=0x${text_start_addr}, .text=0x${text_addr}, .rodata=0x${rodata_addr}, .data=0x${data_addr}, .bss=0x${bss_addr}"
else
  echo "[build_kernel_image] WARNING: llvm-objdump not found, skipping memory map validation" >&2
fi

echo "Run with:"
echo "qemu-system-x86_64 -kernel target/aetheros-x86_64/release/aetheros-kernel -serial stdio -no-reboot -d int"

if [[ "${RUN_QEMU}" == "1" ]]; then
  qemu-system-x86_64 \
    -kernel "${KERNEL_PATH}" \
    -serial stdio \
    -no-reboot \
    -d int
fi
