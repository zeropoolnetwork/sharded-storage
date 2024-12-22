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

## Performance

We run a network of 16 nodes scattered across the globe. The nodes are running on 2-core VPS instances with 4GB of RAM
and 100Mbps network speed.

These are the results of the performance test (client/benches/throughput) run from a single machine with a ~300Mbps
network:

```
concurrency: 1, total throughput: 0.68 MB/sec, avg request time: 1.46 sec
concurrency: 4, total throughput: 4.03 MB/sec, avg request time: 1.04 sec
concurrency: 8, total throughput: 6.62 MB/sec, avg request time: 1.23 sec
concurrency: 16, total throughput: 9.49 MB/sec, avg request time: 1.73 sec
concurrency: 32, total throughput: 10.22 MB/sec, avg request time: 3.18 sec
```

## Documentation

https://zeropool.network/docs/sharded-storage