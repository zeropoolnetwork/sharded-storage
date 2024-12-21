# Sharded storage

A pre-alpha implementation of ZeroPool Sharded Storage.

## Implementation

## Deploy

Run the contract-mock and the validator first. Obtain the validator's multiaddress and use it as storage node's
bootnode.

Refer to `docker-compose.example.yml` for a specific example of how to deploy the services with Docker.

## Usage Examples

### Upload a file

```
cargo run --release --bin client -- --validator-url=http://127.0.0.1:8011 --contract-url=http://127.0.0.1:8010 \
  upload -f test.txt -m="test test test test test test test test test test test junk"
```

This this output the cluster ID that can be used in the download command.

### Download a file

```
cargo run --release --bin client -- --validator-url=http://127.0.0.1:8011 --contract-url=http://127.0.0.1:8010 \
  download -o out.txt -i 4a09785674d14344d92b1212b6e810369535ea1c
```

### Our testnet

`--validator-url=http://45.131.67.89:8011 --contract-url=http://45.131.67.89:8010 `

## Performance

TODO

## Documentation

https://zeropool.network/docs/sharded-storage