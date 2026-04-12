# Environment Audit Checklist (Kernel/UEFI/QEMU)

Use this checklist when validating a new machine for AetherOS development.

## 1) Kernel toolchain dependencies

- [ ] `gcc --version`
- [ ] `clang --version`
- [ ] `nasm -v`
- [ ] `x86_64-elf-gcc --version` (if your boot flow needs cross-toolchain)
- [ ] `grub-mkrescue --version` (ISO path)
- [ ] `mcopy -V` (FAT image path)

## 2) Rust toolchain + target installation

- [ ] `rustup toolchain list | grep nightly-2024-12-01`
- [ ] `rustup component list --toolchain nightly-2024-12-01 | grep rust-src`
- [ ] `rustup component list --toolchain nightly-2024-12-01 | grep llvm-tools-preview`
- [ ] `rustup target list --installed --toolchain nightly-2024-12-01 | grep x86_64-unknown-uefi`
- [ ] `rustup target list --installed --toolchain nightly-2024-12-01 | grep x86_64-unknown-none`

## 3) Python runtime integrity

- [ ] `python3 --version`
- [ ] `pip3 --version`
- [ ] `python3 -c "import websockets, numpy, PIL, IPython"`

## 4) Script permissions and shebang validation

- [ ] All `scripts/*.sh` are executable (`chmod +x`)
- [ ] Shebang uses bash (`#!/usr/bin/env bash`)

## 5) QEMU acceleration readiness (KVM)

- [ ] `/dev/kvm` exists
- [ ] User has read/write permissions to `/dev/kvm`
- [ ] Optional: user in `kvm` group (`groups | grep kvm`)

## 6) Disk space and temporary storage

- [ ] `df -h /` has at least 4G free
- [ ] `df -h /tmp` has at least 1G free

## 7) UEFI firmware dependencies

- [ ] OVMF firmware exists (examples):
  - `/usr/share/OVMF/OVMF_CODE.fd`
  - `/usr/share/OVMF/OVMF.fd`

## 8) Cross-platform reproducibility notes

- [ ] If not on Ubuntu, map package names to distro equivalents
- [ ] For Windows, prefer WSL2 + Ubuntu
- [ ] Document host OS and package manager in your build log

## One-command check

Run the integrated checker:

```bash
./scripts/check_env.sh
```
