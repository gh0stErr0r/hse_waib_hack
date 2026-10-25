FROM rust:1-bookworm AS build
RUN apt-get update && apt-get install -y --no-install-recommends libpq-dev && rm -rf /var/lib/apt/lists/*
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY migrations ./migrations
COPY schemas ./schemas
COPY src ./src
RUN cargo test --locked && cargo build --release --locked
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends libpq5 ca-certificates && rm -rf /var/lib/apt/lists/* && useradd --system --uid 10001 gastro
COPY --from=build /build/target/release/gastro-engine /usr/local/bin/gastro-engine
USER gastro
EXPOSE 8082
CMD ["gastro-engine"]
