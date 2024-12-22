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

RUN USER=root cargo new node
RUN USER=root cargo new contract-mock
RUN USER=root cargo new client

WORKDIR /app/node
RUN cargo build --release

# Build
RUN rm src/*.rs
RUN /bin/bash -c 'rm ../target/release/deps/node*'
COPY node .
RUN cargo build --release

# Final image
FROM rustlang/rust:nightly

WORKDIR /app
COPY --from=build /app/target/release/node /app/node
CMD /app/node
