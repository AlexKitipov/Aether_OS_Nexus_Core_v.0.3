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
```

## 3) Построй kernel ELF (препоръчано чрез helper скрипта)

```bash
cd AetherOS
bash scripts/build_kernel_image.sh
```

Очакван файл:

```text
kernel/target/x86_64-aether_os/release/aetheros-kernel
```

## 4) Стартирай в QEMU

```bash
qemu-system-x86_64 \
  -machine q35 \
  -m 2G \
  -serial stdio \
  -kernel kernel/target/x86_64-aether_os/release/aetheros-kernel
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
