#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="${SCRIPT_DIR}"

cd "${REPO_ROOT}"

echo "Building all V-Nodes..."
# Compile all V-Node projects within the workspace in release mode
cargo build --release --workspace --target x86_64-unknown-none

echo "Creating initrd staging directory..."
# Create a staging directory for initrd contents
STAGING_DIR="${REPO_ROOT}/../initrd_staging/vnodes"
mkdir -p "${STAGING_DIR}"

# Define V-Nodes and their target names
VNODES=(
    "aetheros-logger:logger.ax"
    "aetheros-echo:echo.ax"
    "aetheros-test:test.ax"
    "aetheros-display-compositor:compositor.ax"
    "aetheros-webview:webview.ax"
    "aetheros-shell:shell.ax"
    "aetheros-vfs:vfs.ax"
    "aetheros-init-service:init-service.ax"
    "aetheros-dns-resolver:dns-resolver.ax"
    "aetheros-net-bridge:net-bridge.ax"
    "aetheros-net-stack:net-stack.ax"
    "aetheros-registry:registry.ax"
    "aetheros-socket-api:socket-api.ax"
    "aetheros-file-manager:file-manager.ax"
    "aetheros-mail-service:mail-service.ax"
    "aetheros-model-runtime:model-runtime.ax"
)

# Copy compiled binaries to the staging directory
for vnode_info in "${VNODES[@]}"; do
    IFS=':' read -r crate_name target_name <<< "${vnode_info}"
    src_path="target/x86_64-unknown-none/release/${crate_name}"
    dest_path="${STAGING_DIR}/${target_name}"
    if [ -f "$src_path" ]; then
        echo "Copying $crate_name to $dest_path"
        cp "$src_path" "$dest_path"
    else
        echo "Warning: $src_path not found. Skipping $crate_name."
    fi
done

echo "All specified V-Node binaries staged for initrd."
