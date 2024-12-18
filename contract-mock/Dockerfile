FROM rustlang/rust:nightly as build

RUN apt-get update && apt-get install -y clang

# Cache dependencies
WORKDIR /app
COPY Cargo.lock Cargo.lock
COPY Cargo.toml Cargo.toml
COPY common common
COPY primitives primitives
COPY sealing sealing
COPY spora spora
COPY shards shards
COPY snapshotdb snapshotdb
# FIXME: `cargo new` for each one of these:
COPY node node
COPY client client

WORKDIR /app/contract-mock
RUN USER=root cargo init --name contract-mock
RUN cargo build --release

# Build
RUN rm src/*.rs
RUN /bin/bash -c 'rm ../target/release/deps/contract_mock*'
COPY contract-mock .
RUN cargo build --release

# Final image
FROM rustlang/rust:nightly

WORKDIR /app
COPY --from=build /app/target/release/contract-mock /app/contract-mock
CMD /app/contract-mock
