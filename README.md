# Sharded storage

A pre-alpha implementation of ZeroPool Sharded Storage.

## Running locally

Run the following commands in separate terminals:

`./scripts/run-contract-mock.sh`\
`./scripts/run-validator.sh`\
`./scripts/run-nodes.sh`

then you can run the client scripts:

`./scripts/upload.sh some_file.txt` will output the cluster ID\
`./scripts/download.sh <cluster_id> out.txt`

## Deploy

Run the contract-mock and the validator first. Obtain the validator's multiaddr and use it as storage node's
boot-node.

Refer to `docker-compose.example.yml` for a specific example of how to deploy the components with Docker.

## Client examples

The following examples show how to upload/download files from our testnet.

### Upload a file

```
cargo run --release --bin client -- --validator-url=http://45.131.67.89:8011 --contract-url=http://45.131.67.89:8010 \
  upload -f test.txt -m="test test test test test test test test test test test junk"
```

This outputs the cluster ID that can be used in the download command.

### Download a file

```
cargo run --release --bin client -- --validator-url=http://45.131.67.89:8011 --contract-url=http://45.131.67.89:8010 \
  download -o out.txt -i <cluster id>
```

## Testnet performance

We run a network of 16 nodes scattered across the globe. The nodes are running on 2-core VPS instances with 4GB of RAM
and 100Mbps network speed.

According to load testing done with locust (reports are included in `locust/reports`), the throughput of each node is
limited by its bandwidth (which is ~100Mbps).
It takes 4 shards from 4 different nodes to reconstruct a cluster.
Thus, the effective throughput of the testnet is about `node bandwidth * 4` in the worst case (implies that all clients
land on the same 4 nodes, which is unrealistic), and `node bandwidth * 16`
in the best case. The realistic scenario should be closer to the best case since the clients are distributed evenly in
the current implementation.

## Documentation

https://zeropool.network/docs/sharded-storage