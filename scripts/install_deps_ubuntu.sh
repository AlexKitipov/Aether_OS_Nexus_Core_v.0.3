#!/usr/bin/env bash
set -euo pipefail

sudo apt-get update
sudo apt-get install -y \
  qemu-system-x86 \
  qemu-utils \
  cpio \
  llvm \
  lld \
  binutils \
  gcc \
  clang \
  nasm \
  mtools \
  grub-pc-bin \
  grub-efi-amd64-bin \
  xorriso \
  ovmf \
  python3 \
  python3-pip

python3 -m pip install --upgrade pip
python3 -m pip install -r requirements.txt

echo "Ubuntu dependencies installed successfully."
