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

## 3) Построй kernel ELF

```bash
cd AetherOS
cargo +nightly build --release --target x86_64-unknown-none.json -Zbuild-std=core,alloc,compiler_builtins -Zbuild-std-features=compiler-builtins-mem -Zjson-target-spec
```

Очакван файл:

```text
target/x86_64-unknown-none/release/aetheros-kernel
```

## 4) Стартирай в QEMU

This kernel is a bare-metal ELF image and is not directly bootable with `qemu-system-x86_64 -kernel` unless wrapped by a compatible bootloader or UEFI image.

## Бърз автоматичен вариант

```bash
bash scripts/build_kernel_image.sh
RUN_QEMU=1 bash scripts/build_kernel_image.sh
```
