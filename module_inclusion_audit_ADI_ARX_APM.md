# Module Inclusion Audit: ADI, ARX, APM

## Scope requested
Requested directories:
- `AetherOS_Coder/ADI/src`
- `AetherOS_Coder/ARX/src`
- `AetherOS_Coder/APM/src`

These directories do **not** exist in this repository layout. Each crate uses a non-standard root module layout with `[lib] path = "mod.rs"`.

## Cargo entrypoints
- `AetherOS/AetherOS_Coder/ADI/Cargo.toml` → `[lib] path = "mod.rs"`
- `AetherOS/AetherOS_Coder/ARX/Cargo.toml` → `[lib] path = "mod.rs"`
- `AetherOS/AetherOS_Coder/APM/Cargo.toml` → `[lib] path = "mod.rs"`

## Results

### ADI
Reachable through `ADI/mod.rs`:
- `interface.rs`
- `analyzer/mod.rs` → `rules.rs`, `score.rs`, `report.rs`
- `analysis/mod.rs` → `capabilities.rs`, `device_scan.rs`, `metadata.rs`
- `translator/mod.rs` → `mapper.rs`, `adapter.rs`, `abi.rs`
- `sandbox/mod.rs` → `model.rs`, `governor.rs`, `runtime.rs`, `memory.rs`, `executor.rs`, `ipc.rs`, `libraries/mod.rs` → `io.rs`, `mem.rs`, `time.rs`

Potential orphan relative to ADI crate root:
- `ADI/sandbox/src/lib.rs`
- `ADI/sandbox/src/tests/runtime_integration_test.rs`

These belong to a nested crate/work area under `ADI/sandbox/` and are not pulled by ADI root `mod.rs`.

### ARX
Reachable through `ARX/mod.rs`:
- `context.rs`, `loader.rs`, `process.rs`, `sandbox.rs`, `syscall.rs`, `api/mod.rs`
- `api/mod.rs` uses `#[path = "api.*.rs"]` attributes to include:
  - `api/api.io.rs`
  - `api/api.ipc.rs`
  - `api/api.mem.rs`
  - `api/api.time.rs`

No orphan `.rs` files detected under ARX.

### APM
Reachable through `APM/mod.rs`:
- `manifest.rs`, `installer.rs`, `updater.rs`, `registry.rs`

No orphan `.rs` files detected under APM.

## Build inclusion conclusion
- ADI, ARX, and APM crates are compiled (seen in build output: `Compiling adi`, `Compiling arx`, `Compiling apm`).
- All crate-root module files for ARX and APM are included and reachable.
- ADI has two `.rs` files under nested `sandbox/src/` that are not part of ADI root module tree; they appear to be for the nested sandbox crate/test area.
