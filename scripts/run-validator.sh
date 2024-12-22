#!/usr/bin/env bash

export EXTERNAL_IP=127.0.0.1
export API_ADDR="0.0.0.0:8099"
export PUBLIC_API_URL="http://127.0.0.1:8099"
export P2P_PORT="30299"
export SEED_PHRASE="goose must course few long easy charge false sponsor float clinic example"
export CONTRACT_MOCK_URL="http://127.0.0.1:8001"
unset NODE_ID
env | grep -E "API_ADDR|PUBLIC_API_URL|P2P_PORT|SEED_PHRASE"
cargo run --release --bin node