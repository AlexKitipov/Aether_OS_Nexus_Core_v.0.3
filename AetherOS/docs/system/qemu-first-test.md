# Първи тест с QEMU

Този документ описва минималните стъпки за първо стартиране на ядрото в QEMU с новия `bootloader_api` 0.11 build flow.

## 1) Инсталирай QEMU

```bash
sudo apt-get update
sudo apt-get install -y qemu-system-x86
```

Провери:

```bash
qemu-system-x86_64 --version
```

## 2) Подготви Rust инструментите

```bash
rustup toolchain install nightly
rustup component add rust-src --toolchain nightly
rustup component add llvm-tools-preview --toolchain nightly
```

## 3) Построй bootloader BIOS image

```bash
cd AetherOS
BOOT_MODE=bios ./scripts/build_kernel_image.sh
```

Очакван файл:

```text
target/x86_64-unknown-none/release/aetheros-bios.img
```

The build also produces the kernel ELF at `target/x86_64-unknown-none/release/aetheros-kernel`, but it is an input to the bootloader image builder and must not be passed directly to QEMU's `-kernel` option.

## 4) Стартирай в QEMU

```bash
BOOT_MODE=bios ./scripts/run_qemu.sh
```

The BIOS image is the standard smoke-test path because it exercises the `bootloader_api` 0.11 handoff. The UEFI path remains available for manual testing when `OVMF_CODE` is configured:

```bash
BOOT_MODE=uefi OVMF_CODE=/path/to/OVMF_CODE.fd ./scripts/run_qemu.sh
```

## 5) Reproducible smoke test and logs

```bash
BOOT_MODE=bios ./scripts/build_kernel_image.sh
./scripts/test_qemu.sh
```

The smoke test boots the BIOS disk image with a fixed 256 MiB memory configuration, stores serial output in `target/qemu-smoke/serial.log`, and stores QEMU interrupt/reset tracing in `target/qemu-smoke/qemu-debug.log`. It succeeds only when QEMU stays alive through the timeout, reports no triple fault or post-power-on reset, and the expected boot-progress marker appears in the serial log.

## Бърз автоматичен вариант

```bash
BOOT_MODE=bios bash scripts/build_kernel_image.sh
BOOT_MODE=bios RUN_QEMU=1 bash scripts/build_kernel_image.sh
```
