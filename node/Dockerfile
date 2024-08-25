FROM rustlang/rust:nightly as build

ARG FEATURES=""
ENV FEATURES=$FEATURES

RUN apt-get update && apt-get install -y clang

# Cache dependencies
WORKDIR /app
COPY Cargo.lock Cargo.lock
COPY Cargo.toml Cargo.toml
COPY common common
WORKDIR /app/node
RUN USER=root cargo init --name zeropool-sharded-storage-node
RUN cargo build --release --features "$FEATURES"

# Build
RUN rm src/*.rs
RUN /bin/bash -c 'rm ./target/release/deps/zeropool_sharded_storage_node*'
COPY ./src ./src
RUN cargo build --release --features "$FEATURES"

# Final image
FROM rustlang/rust:nightly

WORKDIR /app
COPY --from=build /app/target/release/zeropool-sharded-storage-node /app/zeropool-sharded-storage-node
CMD /app/zeropool-sharded-storage-node
