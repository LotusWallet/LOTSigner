#!/usr/bin/env bash
# Lotus Signer · 一键构建（宪法 G-I 路径 A 的落地物 / BETA-READINESS A1）
# 前置：rustup（stable）。零第三方 crate，无需访问 crates.io。
set -euo pipefail
cd "$(dirname "$0")"

export CARGO_NET_OFFLINE=true
# D1：把 commit 哈希注入产物，--version 可溯源
export LS_COMMIT="$(git rev-parse --short HEAD 2>/dev/null || echo unknown)"

echo "== 1/3 构建 CLI（lscli）=="
cargo build --release --offline -p lscli

echo "== 2/3 构建 wasm 内核（lswasm）=="
rustup target add wasm32-unknown-unknown >/dev/null 2>&1 || true
cargo build --release --offline --target wasm32-unknown-unknown -p lswasm

echo "== 3/3 布置 web/ =="
cp target/wasm32-unknown-unknown/release/lswasm.wasm web/lswasm.wasm
mkdir -p web/demo
cp g3/fixtures/S1.psbt g3/fixtures/S4.psbt web/demo/

echo
echo "完成。启动方式："
echo "  Windows : 双击 web/serve.cmd"
echo "  其他    : cd web && python3 -m http.server 4180"
echo "然后浏览器打开 http://127.0.0.1:4180"
