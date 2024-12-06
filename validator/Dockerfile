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
COPY storage storage

COPY validator validator
COPY contract-mock contract-mock

WORKDIR /app/validator
RUN USER=root cargo init --name validator
RUN cargo build --release

# Build
RUN rm src/*.rs
RUN /bin/bash -c 'rm ./target/release/deps/validator*'
COPY validator/src ./src
RUN cargo build --release

# Final image
FROM rustlang/rust:nightly

WORKDIR /app
COPY --from=build /app/target/release/validator /app/validator
CMD /app/validator
