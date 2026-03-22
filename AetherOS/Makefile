SHELL := /usr/bin/env bash

.PHONY: help kernel all initrd run test clean

help:
	@echo "AetherOS build/test targets"
	@echo "  make kernel  - Build kernel image"
	@echo "  make all     - Build kernel + selected V-Nodes"
	@echo "  make initrd  - Build initrd image from rootfs"
	@echo "  make run     - Run kernel in QEMU"
	@echo "  make test    - Non-interactive QEMU smoke test"
	@echo "  make clean   - Cargo clean workspace"

kernel:
	./scripts/build_kernel_image.sh

all:
	./scripts/build_all.sh

initrd:
	./scripts/build_initrd.sh

run: kernel
	./scripts/run_qemu.sh

test: kernel
	./scripts/test_qemu.sh

clean:
	cargo clean
