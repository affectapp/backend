#!/bin/bash
# Builds and runs a local affect server only on http2.
# Use ./container.sh to support http2 (frontend requests).
export CONFIG_PATH=server/affect.toml
export RUST_LOG=debug
cargo run