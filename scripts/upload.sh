#!/usr/bin/env bash

cargo run --release --bin client -- --validator-url=http://127.0.0.1:8099 --contract-url=http://127.0.0.1:8001 \
  upload -f "$1" -m='test test test test test test test test test test test discover'

  
