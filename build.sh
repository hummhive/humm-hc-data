#!/usr/bin/env bash
set -e

CARGO_TARGET_DIR=target cargo build --release --target wasm32-unknown-unknown
hc dna pack ./dna
hc app pack ./happ