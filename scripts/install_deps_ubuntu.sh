#!/usr/bin/env bash
set -euo pipefail

sudo apt-get update
sudo apt-get install -y \
  qemu-system-x86 \
  cpio \
  llvm \
  lld \
  binutils \
  python3 \
  python3-pip

python3 -m pip install --upgrade pip
python3 -m pip install -r requirements.txt

echo "Ubuntu dependencies installed successfully."
