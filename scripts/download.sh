#!/usr/bin/env bash

cargo run --release --bin client -- --validator-url=http://127.0.0.1:8099 --contract-url=http://127.0.0.1:8001 \
  download -o "$2" -i "$1"
