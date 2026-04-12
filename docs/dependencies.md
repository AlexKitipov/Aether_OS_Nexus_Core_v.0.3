# Dependencies

This document centralizes the host/runtime dependencies required to build and run **AetherOS Nexus Core v0.3**.

## Tested platform

- **Ubuntu 22.04 LTS** (primary tested environment)

## System packages

Required host commands/packages:

- `qemu-system-x86_64` (from `qemu-system-x86`)
- `cpio`
- `timeout` (from `coreutils`)
- `llvm`
- `lld` (`ld.lld`)
- `binutils` (`llvm-ar` may also come from LLVM toolchain)
- `python3`
- `python3-pip`

## Rust toolchain

Pinned toolchain/config used by this repository:

- Toolchain: `nightly-2024-12-01`
- Components:
  - `rust-src`
  - `llvm-tools-preview`
- Targets:
  - `x86_64-unknown-uefi`
  - `x86_64-unknown-none`

Install example:

```bash
rustup toolchain install nightly-2024-12-01
rustup component add --toolchain nightly-2024-12-01 rust-src llvm-tools-preview
rustup target add --toolchain nightly-2024-12-01 x86_64-unknown-uefi x86_64-unknown-none
```

## Python packages

For `AetherOS/Nexus/UI/tools/*.py` scripts, install:

- `websockets`
- `numpy`
- `Pillow`
- `IPython`

Install example:

```bash
pip install -r requirements.txt
```

## Helper scripts

- Verify environment: `scripts/check_env.sh`
- Ubuntu install helper: `scripts/install_deps_ubuntu.sh`
