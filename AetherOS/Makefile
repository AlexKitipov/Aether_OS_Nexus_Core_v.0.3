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
	@echo "[diag][stage=make.kernel] launching kernel build script"
	@echo "Building kernel (pinned nightly flow)"
	# Some tool wrappers and CI shells invoke `make` with no manifest context,
	# which can trigger `CARGO_MANIFEST_DIR` warnings in legacy cargo-bootimage
	# setups. Export a stable kernel manifest path for deterministic behavior.
	CARGO_MANIFEST_DIR="$(CURDIR)/$(KERNEL_DIR)" bash ./scripts/build_kernel_image.sh

all:
	@echo "[diag][stage=make.all] launching full workspace build script"
	bash ./scripts/build_all.sh

initrd:
	@echo "[diag][stage=make.initrd] building initrd archive"
	bash ./scripts/build_initrd.sh

run: kernel
	@echo "[diag][stage=make.run] launching QEMU runtime"
	bash ./scripts/run_qemu.sh

test: kernel
	@echo "[diag][stage=make.test] launching QEMU smoke test"
	bash ./scripts/test_qemu.sh

clean:
	cargo clean
