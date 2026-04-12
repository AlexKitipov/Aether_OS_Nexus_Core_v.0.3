# Dependencies

This document centralizes the host/runtime dependencies required to build and run **AetherOS Nexus Core v0.3**.

## Tested platform

- **Ubuntu 22.04 LTS** (primary tested environment)

## System packages

### Required baseline commands/packages

- `qemu-system-x86_64` (from `qemu-system-x86`)
- `cpio`
- `timeout` (from `coreutils`)
- `llvm`
- `lld` (`ld.lld`)
- `binutils` (`llvm-ar` may also come from LLVM toolchain)
- `python3`
- `python3-pip`

### Additional kernel/image pipeline dependencies (recommended)

- `gcc` and/or `clang`
- `nasm`
- `x86_64-elf-gcc` (cross-compiler, optional depending on boot pipeline)
- `grub-mkrescue` (from `grub-pc-bin` + `xorriso`)
- `mtools` (`mcopy`, FAT image workflows)

### UEFI firmware for QEMU

- `OVMF_CODE.fd` / `OVMF.fd` from package `ovmf` (or distro equivalent such as edk2-ovmf)

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
rustup target list --installed --toolchain nightly-2024-12-01
```

## Python runtime and modules

For `AetherOS/Nexus/UI/tools/*.py` scripts:

- Runtime checks: `python3 --version`, `pip3 --version`
- Required modules:
  - `websockets`
  - `numpy`
  - `Pillow`
  - `IPython`

Install example:

```bash
pip install -r requirements.txt
```

## Script execution integrity

Before running helper scripts, validate:

- executable bit is set (`chmod +x scripts/*.sh`)
- shebang lines are correct (prefer `#!/usr/bin/env bash`)

## QEMU acceleration (KVM)

For acceptable performance on Linux:

- `/dev/kvm` should exist
- current user should have read/write access to `/dev/kvm`

Without KVM, QEMU emulation can be significantly slower.

## Disk and tmpfs capacity

Low free space commonly breaks kernel builds/initrd generation.

Suggested minimums:

- root filesystem free space: `>= 4 GB`
- `/tmp` free space: `>= 1 GB`

## Cross-platform reproducibility notes

- Current helper script (`scripts/install_deps_ubuntu.sh`) targets Debian/Ubuntu package names.
- macOS and other Linux distributions may require alternate package names and paths.
- Windows users should prefer WSL2 + Ubuntu for reproducible kernel builds.

## Helper scripts

- Verify environment: `scripts/check_env.sh`
- Ubuntu install helper: `scripts/install_deps_ubuntu.sh`
