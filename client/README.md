# Sharded storage CLI client

## How to run

### Upload

```
cargo run --release --bin client -v http://validator -c http://contract upload -f <file> -m "seed phrase"
```

Outputs uploaded cluster ID that can be used in the download command.

### Download

```
cargo run --release --bin client -v http://validator -c http://contract download -i <cluster id> -o <out file>
```
