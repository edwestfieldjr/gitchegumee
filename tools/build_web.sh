#!/usr/bin/env bash
# Build the website and browser version of the game into dist/.
set -euo pipefail
cd "$(dirname "$0")/.."

target_dir="${CARGO_TARGET_DIR:-target}"
cargo build --locked --profile wasm-release --target wasm32-unknown-unknown

rm -rf dist
mkdir -p dist
cp -r site/. dist/
wasm-bindgen --no-typescript --target web --out-dir dist/play --out-name game \
    "$target_dir/wasm32-unknown-unknown/wasm-release/downbound.wasm"
cp -r assets dist/play/assets
