#!/usr/bin/env bash


# Node 0
NODE_0_NODE_ID="0"
NODE_0_API_ADDR="0.0.0.0:8100"
NODE_0_PUBLIC_API_URL="http://127.0.0.1:8100"
NODE_0_P2P_PORT="30300"
NODE_0_SEED_PHRASE="test test test test test test test test test test test absent"
# Node 1
NODE_1_NODE_ID="1"
NODE_1_API_ADDR="0.0.0.0:8101"
NODE_1_PUBLIC_API_URL="http://127.0.0.1:8101"
NODE_1_P2P_PORT="30301"
NODE_1_SEED_PHRASE="test test test test test test test test test test test actual"
# Node 2
NODE_2_NODE_ID="2"
NODE_2_API_ADDR="0.0.0.0:8102"
NODE_2_PUBLIC_API_URL="http://127.0.0.1:8102"
NODE_2_P2P_PORT="30302"
NODE_2_SEED_PHRASE="test test test test test test test test test test test agree"
# Node 3
NODE_3_NODE_ID="3"
NODE_3_API_ADDR="0.0.0.0:8103"
NODE_3_PUBLIC_API_URL="http://127.0.0.1:8103"
NODE_3_P2P_PORT="30303"
NODE_3_SEED_PHRASE="test test test test test test test test test test test alone"
# Node 4
NODE_4_NODE_ID="4"
NODE_4_API_ADDR="0.0.0.0:8104"
NODE_4_PUBLIC_API_URL="http://127.0.0.1:8104"
NODE_4_P2P_PORT="30304"
NODE_4_SEED_PHRASE="test test test test test test test test test test test analyst"
# Node 5
NODE_5_NODE_ID="5"
NODE_5_API_ADDR="0.0.0.0:8105"
NODE_5_PUBLIC_API_URL="http://127.0.0.1:8105"
NODE_5_P2P_PORT="30305"
NODE_5_SEED_PHRASE="test test test test test test test test test test test apart"
# Node 6
NODE_6_NODE_ID="6"
NODE_6_API_ADDR="0.0.0.0:8106"
NODE_6_PUBLIC_API_URL="http://127.0.0.1:8106"
NODE_6_P2P_PORT="30306"
NODE_6_SEED_PHRASE="test test test test test test test test test test test ask"
# Node 7
NODE_7_NODE_ID="7"
NODE_7_API_ADDR="0.0.0.0:8107"
NODE_7_PUBLIC_API_URL="http://127.0.0.1:8107"
NODE_7_P2P_PORT="30307"
NODE_7_SEED_PHRASE="test test test test test test test test test test test aunt"
# Node 8
NODE_8_NODE_ID="8"
NODE_8_API_ADDR="0.0.0.0:8108"
NODE_8_PUBLIC_API_URL="http://127.0.0.1:8108"
NODE_8_P2P_PORT="30308"
NODE_8_SEED_PHRASE="test test test test test test test test test test test ball"
# Node 9
NODE_9_NODE_ID="9"
NODE_9_API_ADDR="0.0.0.0:8109"
NODE_9_PUBLIC_API_URL="http://127.0.0.1:8109"
NODE_9_P2P_PORT="30309"
NODE_9_SEED_PHRASE="test test test test test test test test test test test bean"
# Node 10
NODE_10_NODE_ID="10"
NODE_10_API_ADDR="0.0.0.0:8110"
NODE_10_PUBLIC_API_URL="http://127.0.0.1:8110"
NODE_10_P2P_PORT="30310"
NODE_10_SEED_PHRASE="test test test test test test test test test test test beyond"
# Node 11
NODE_11_NODE_ID="11"
NODE_11_API_ADDR="0.0.0.0:8111"
NODE_11_PUBLIC_API_URL="http://127.0.0.1:8111"
NODE_11_P2P_PORT="30311"
NODE_11_SEED_PHRASE="test test test test test test test test test test test blast"
# Node 12
NODE_12_NODE_ID="12"
NODE_12_API_ADDR="0.0.0.0:8112"
NODE_12_PUBLIC_API_URL="http://127.0.0.1:8112"
NODE_12_P2P_PORT="30312"
NODE_12_SEED_PHRASE="test test test test test test test test test test test boat"
# Node 13
NODE_13_NODE_ID="13"
NODE_13_API_ADDR="0.0.0.0:8113"
NODE_13_PUBLIC_API_URL="http://127.0.0.1:8113"
NODE_13_P2P_PORT="30313"
NODE_13_SEED_PHRASE="test test test test test test test test test test test boy"
# Node 14
NODE_14_NODE_ID="14"
NODE_14_API_ADDR="0.0.0.0:8114"
NODE_14_PUBLIC_API_URL="http://127.0.0.1:8114"
NODE_14_P2P_PORT="30314"
NODE_14_SEED_PHRASE="test test test test test test test test test test test bulb"
# Node 15
NODE_15_NODE_ID="15"
NODE_15_API_ADDR="0.0.0.0:8115"
NODE_15_PUBLIC_API_URL="http://127.0.0.1:8115"
NODE_15_P2P_PORT="30315"
NODE_15_SEED_PHRASE="test test test test test test test test test test test cabbage"

PIDS=()

cleanup() {
    echo "Cleaning up..."
    for pid in "${PIDS[@]}"; do
        if kill -0 $pid 2>/dev/null; then
            echo "Killing process $pid"
            kill -9 $pid
        fi
    done
    exit 0
}

trap cleanup SIGINT
trap cleanup SIGTERM

cargo build --release --bin node

export_node_vars() {
  local node_prefix="NODE_${1}_"
  for var in $(compgen -v | grep "^${node_prefix}"); do
    # Remove the prefix and export the variable
    new_var=${var#NODE_${1}_}
    export "$new_var=${!var}"
  done
}

export EXTERNAL_IP=127.0.0.1
export CONTRACT_MOCK_URL="http://127.0.0.1:8001"
export RUST_LOG=debug

NUM_INSTANCES=16
for ((i=0; i<NUM_INSTANCES; i++)); do
  export_node_vars $i
  env | grep -E "NODE_ID|API_ADDR|PUBLIC_API_URL|EXTERNAL_IP|P2P_PORT|SEED_PHRASE|BOOT_NODE"
  export STORAGE_DIR="./data/node${i}-storage"
  VALIDATOR_ADDRESS=$(cat ./data/validator_addr)
  ./target/release/node -b "$VALIDATOR_ADDRESS" &
  PIDS+=($!)
done

wait