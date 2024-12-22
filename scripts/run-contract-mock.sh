#!/usr/bin/env bash

export RUST_LOG=debug
export PORT=8001
cargo run --release --bin contract-mock