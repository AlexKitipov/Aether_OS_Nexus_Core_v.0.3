# Първи тест с QEMU

Този документ описва минималните стъпки за първо стартиране на ядрото в QEMU.

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
cargo +nightly install bootimage --locked
```

## 3) Построй bootable image с `cargo bootimage`

```bash
cd AetherOS
cargo +nightly bootimage -p aetheros-kernel --manifest-path kernel/Cargo.toml --release
```

Очакван файл:

```text
kernel/target/x86_64-aether_os/release/bootimage-aetheros-kernel.bin
```

## 4) Стартирай в QEMU

```bash
qemu-system-x86_64 \
  -machine q35 \
  -m 2G \
  -serial stdio \
  -drive format=raw,file=kernel/target/x86_64-aether_os/release/bootimage-aetheros-kernel.bin
```

## Бърз автоматичен вариант

Има и helper скрипт:

```bash
bash scripts/build_kernel_image.sh
```

Ако искаш скриптът директно да стартира QEMU след build:

```bash
RUN_QEMU=1 bash scripts/build_kernel_image.sh
```
