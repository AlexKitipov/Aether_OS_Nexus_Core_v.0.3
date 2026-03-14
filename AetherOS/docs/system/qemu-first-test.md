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
cargo +nightly build --release --target .cargo/aetheros-x86_64.json -Zbuild-std=core,alloc,compiler_builtins -Zbuild-std-features=compiler-builtins-mem -Z unstable-options -Z json-target-spec
```

Очакван файл:

```text
target/aetheros-x86_64/release/aetheros-kernel
```

## 4) Стартирай в QEMU

```bash
qemu-system-x86_64 -kernel target/aetheros-x86_64/release/aetheros-kernel
```

## Бърз автоматичен вариант

```bash
bash scripts/build_kernel_image.sh
RUN_QEMU=1 bash scripts/build_kernel_image.sh
```
