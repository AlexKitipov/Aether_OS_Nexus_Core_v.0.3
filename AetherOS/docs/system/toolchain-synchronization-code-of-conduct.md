# Toolchain Synchronization Code of Conduct

## Purpose

This code of conduct establishes a **single source of truth** for toolchain state across:

- AI assistants (Copilot, Gemini, Codex/local agents)
- Local automation scripts
- CI/build orchestration
- Native Rustup installations

Canonical reference:

- **Rustup toolchain:** `nightly-2024-12-01-x86_64-unknown-linux-gnu`
- **rustc:** `1.85.0-nightly (7442931d4 2024-11-30)`

The goal is to eliminate drift between local and generated instructions, improving build determinism and runtime stability.

## 1) Single Source of Truth

All automation and contributors must align to the canonical Rustup state:

- `toolchain = nightly-2024-12-01-x86_64-unknown-linux-gnu`
- `rustc = 1.85.0-nightly (7442931d4 2024-11-30)`
- Required components:
  - `cargo`
  - `clippy`
  - `rustc`
  - `rust-std`
  - `rust-docs`
  - `rust-src`
  - `llvm-tools`
  - `rustfmt`

Install/unpack progress states (downloaded, unpacked, installed) are considered valid transitional states only during reconciliation.

## 2) Unified Validation Protocol

Every validating agent must produce/consume the same normalized snapshot format:

```json
{
  "toolchain": "nightly-2024-12-01-x86_64-unknown-linux-gnu",
  "rustc": "1.85.0-nightly (7442931d4 2024-11-30)",
  "components": {
    "cargo": "installed",
    "clippy": "installed",
    "rustc": "installed",
    "rust-std": "installed",
    "rust-docs": "installed",
    "rust-src": "installed",
    "llvm-tools": "installed",
    "rustfmt": "installed"
  },
  "status": "synced"
}
```

If any agent detects deviation, it must report:

- `status: out-of-sync`
- exact field-level mismatch
- proposed correction to canonical values

## 3) Build and Runtime Stabilization Rules

To preserve reproducibility:

1. Do not introduce alternate nightly channels in project-level toolchain files.
2. Do not gate scripts on host-default toolchain when a pinned nightly is expected.
3. Keep local snapshots in `.aether/` aligned with the canonical state.
4. Treat mismatches as release blockers for build, snapshot, and runtime-sensitive pipelines.

## 4) Files to Check During Synchronization

### Rustup-managed state

- `~/.rustup/toolchains/nightly-2024-12-01-x86_64-unknown-linux-gnu/`
- `~/.rustup/settings.toml`
- `~/.rustup/update-hashes/`
- `~/.rustup/tmp/`

### Component manifests

- `manifest-rustc`
- `manifest-cargo`
- `manifest-clippy`
- `manifest-rust-std`
- `manifest-rust-docs`
- `manifest-rust-src`

### Project build configuration

- `rust-toolchain.toml`
- `AetherOS/rust-toolchain.toml`
- `Cargo.toml`
- `Cargo.lock`
- `.cargo/config.toml` (if present)

### Agent/build snapshots

- `.aether/toolchain.json`
- `.aether/build-state.json`

## 5) Synchronization Workflow

1. Extract canonical Rustup log values.
2. Generate local snapshot.
3. Compare snapshot vs canonical baseline.
4. Mark mismatches as `out-of-sync`.
5. Apply corrections to project/tool snapshots.
6. Re-validate and mark `synced`.

## 6) Compliance Outcome

This policy is considered successful when:

- all tracked files declare `nightly-2024-12-01-x86_64-unknown-linux-gnu`
- agent snapshots resolve to identical canonical values
- build pipelines become deterministic across local and automated environments
