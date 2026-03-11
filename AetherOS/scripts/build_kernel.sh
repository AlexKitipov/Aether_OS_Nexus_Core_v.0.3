#!/usr/bin/env bash
set -e

# Build kernel with nightly and custom target
cargo +nightly build \
    -Z json-target-spec \
    --target aetheros-target.json \
    -p aetheros-kernel
